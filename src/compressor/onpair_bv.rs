use crate::bit_vector::BitVector;
use crate::longest_prefix_matcher::lpm::LongestPrefixMatcher;
use super::Compressor;
use rustc_hash::{FxHashMap, FxHashSet};
use rand::seq::SliceRandom;
use rand::thread_rng;

const BITS_PER_TOKEN: usize = 20;
const MAX_TOKEN_ID: usize = (1 << BITS_PER_TOKEN) - 1; 
const FAST_ACCESS_SIZE: usize = 16;

pub struct OnPairBVCompressor {
    compressed_data: BitVector,                 // Store the compressed data as token IDs
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
}

impl Compressor for OnPairBVCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPairBVCompressor {
            compressed_data: BitVector::with_capacity(data_size * BITS_PER_TOKEN),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::with_capacity(2 * 1024 * 1024), // 2 MiB
            dictionary_end_positions: Vec::with_capacity(1 << 16),
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let lpm = self.train(data, end_positions);
        self.parse(data, end_positions, &lpm);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        let dict_ptr = self.dictionary.as_ptr();
        let end_positions_ptr = self.dictionary_end_positions.as_ptr();
        let mut size = 0;

        for i in 0..self.compressed_data.len() / BITS_PER_TOKEN {
            let offset = i * BITS_PER_TOKEN;
            let token_id = self.compressed_data.get_bits(offset, BITS_PER_TOKEN).unwrap() as usize;
            
            unsafe {
                let dict_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dict_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dict_end - dict_start;

                let mut src = dict_ptr.add(dict_start);
                let mut dst = buffer.as_mut_ptr().add(size);
                std::ptr::copy_nonoverlapping(src, dst, FAST_ACCESS_SIZE);

                if length > FAST_ACCESS_SIZE {
                    src = src.add(FAST_ACCESS_SIZE); 
                    dst = dst.add(FAST_ACCESS_SIZE);
                    std::ptr::copy_nonoverlapping(src, dst, length - FAST_ACCESS_SIZE);
                }

                size += length;
            }
        }

        size
    }

    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        let item_start = self.item_end_positions[index];
        let item_end = self.item_end_positions[index + 1];
        let dict_ptr = self.dictionary.as_ptr();
        let end_positions_ptr = self.dictionary_end_positions.as_ptr();
        let mut size = 0;

        for i in item_start..item_end {
            let offset = i * BITS_PER_TOKEN;
            let token_id = self.compressed_data.get_bits(offset, BITS_PER_TOKEN).unwrap() as usize;

            unsafe {
                let dict_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dict_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dict_end - dict_start;

                let mut src = dict_ptr.add(dict_start);
                let mut dst = buffer.as_mut_ptr().add(size);
                std::ptr::copy_nonoverlapping(src, dst, FAST_ACCESS_SIZE);

                if length > FAST_ACCESS_SIZE {
                    src = src.add(FAST_ACCESS_SIZE); 
                    dst = dst.add(FAST_ACCESS_SIZE);
                    std::ptr::copy_nonoverlapping(src, dst, length - FAST_ACCESS_SIZE);
                }

                size += length;
            }
        }

        size
    }

    fn space_used_bytes(&self) -> usize {
        (self.compressed_data.len() / 8) + self.dictionary.len() + (self.dictionary_end_positions.len() * std::mem::size_of::<u32>())
    }

    fn name(&self) -> &str {
        "OnPair_BV"
    }
}

impl OnPairBVCompressor {
    fn train(&mut self, data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher<usize> {
        self.dictionary_end_positions.push(0);
        
        let mut frequency: FxHashMap<(usize, usize), usize> = FxHashMap::default();
        let mut lpm = LongestPrefixMatcher::new();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = vec![i as u8];
            lpm.insert(&token, i);
            self.dictionary.extend(&token);
            self.dictionary_end_positions.push(self.dictionary.len() as u32);
        }

        // Shuffle entries
        let mut shuffled_indices: Vec<usize> = (0..end_positions.len()-1).collect();
        shuffled_indices.shuffle(&mut thread_rng());

        // Set the threshold for merging tokens
        let data_size_mib = data.len() as f64 / (1024.0 * 1024.0);
        let threshold = data_size_mib.log2().max(2.0) as usize;
        
        // Iterate over entries
        'outer: for &index in shuffled_indices.iter() {
            let start = end_positions[index];
            let end = end_positions[index + 1];

            if start == end {
                continue;
            }
    
            let (match_token_id, match_length) = lpm.find_longest_match(&data[start..end]).unwrap();
            let mut previous_token_id = match_token_id;
            let mut previous_length = match_length;

            let mut pos = start + previous_length;
    
            while pos < end {
                // Find the longest match
                let (match_token_id, match_length) = lpm.find_longest_match(&data[pos..end]).unwrap();
    
                 // Update token frequency and possibly merge tokens
                *frequency.entry((previous_token_id, match_token_id)).or_insert(0) += 1;
    
                if frequency[&(previous_token_id, match_token_id)] >= threshold {
                    let merged_token = &data[pos - previous_length..pos + match_length];
                    lpm.insert(merged_token, next_token_id);
                    self.dictionary.extend(merged_token);
                    self.dictionary_end_positions.push(self.dictionary.len() as u32);
                    
                    frequency.remove(&(previous_token_id, match_token_id));
                    previous_token_id = next_token_id;
                    previous_length = merged_token.len();

                    if next_token_id == MAX_TOKEN_ID {
                        break 'outer;
                    }

                    next_token_id += 1;
                }
                else {
                    previous_token_id = match_token_id;
                    previous_length = match_length;
                }
            
                pos += match_length;
            }
        }
    
        lpm
    }
    
    fn parse(&mut self, data: &[u8], end_positions: &[usize], lpm: &LongestPrefixMatcher<usize>) {
        self.item_end_positions.push(0);

        for window in end_positions.windows(2) {
            let start = window[0];
            let end = window[1];

            if start == end {
                self.item_end_positions.push(self.compressed_data.len() / BITS_PER_TOKEN);
                continue;
            }
    
            let mut pos = start;
            while pos < end {
                // Find the longest match
                let (token_id, length) = lpm.find_longest_match(&data[pos..end]).unwrap();
                let bits = token_id as u64;
                self.compressed_data.append_bits(bits, BITS_PER_TOKEN); 
                pos += length;
            }
    
            self.item_end_positions.push(self.compressed_data.len() / BITS_PER_TOKEN);
        }
    }
}
