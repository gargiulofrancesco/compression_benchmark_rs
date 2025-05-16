use crate::longest_prefix_matcher::lpm16::LongestPrefixMatcher;
use crate::longest_prefix_matcher::lpm16::StaticLongestPrefixMatcher;
use super::Compressor;
use rustc_hash::FxHashMap;
use rand::seq::SliceRandom;
use rand::thread_rng;

const THRESHOLD: usize = 10;
const MAX_LENGTH: usize = 16;

pub struct OnPair16Compressor {
    compressed_data: Vec<u16>,                  // Store the compressed data as token IDs
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
}

impl Compressor for OnPair16Compressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPair16Compressor {
            compressed_data: Vec::with_capacity(data_size),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::with_capacity(2 * (1024 * 1024)), // 2 MB
            dictionary_end_positions: Vec::with_capacity(1 << 16),
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let lpm = self.train(data, end_positions);
        let lpm_static = lpm.finalize();
        self.parse(data, end_positions, &lpm_static);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        let dict_ptr = self.dictionary.as_ptr();
        let end_positions_ptr = self.dictionary_end_positions.as_ptr();
        let mut size = 0;

        for &token_id in self.compressed_data.iter(){
            unsafe {
                // Access dictionary positions using raw pointers
                let dict_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dict_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dict_end - dict_start;

                let src = dict_ptr.add(dict_start);
                let dst = buffer.as_mut_ptr().add(size);
                std::ptr::copy_nonoverlapping(src, dst, MAX_LENGTH);

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

        for &token_id in &self.compressed_data[item_start..item_end] {
            unsafe {
                // Access dictionary positions using raw pointers
                let dict_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dict_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dict_end - dict_start;

                let src = dict_ptr.add(dict_start);
                let dst = buffer.as_mut_ptr().add(size);
                std::ptr::copy_nonoverlapping(src, dst, MAX_LENGTH);

                size += length;
            }
        }

        size
    }

    fn space_used_bytes(&self) -> usize {
        (self.compressed_data.len() * std::mem::size_of::<u16>()) + self.dictionary.len() + (self.dictionary_end_positions.len() * std::mem::size_of::<u32>())
    }

    fn name(&self) -> &str {
        "OnPair16"
    }
}

impl OnPair16Compressor {
    fn train(&mut self, data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher {
        self.dictionary_end_positions.push(0);

        let mut frequency: FxHashMap<(u16, u16), usize> = FxHashMap::default();
        let mut lpm = LongestPrefixMatcher::new();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = vec![i as u8];
            lpm.insert(&token, i as u16);
            self.dictionary.extend(&token);
            self.dictionary_end_positions.push(self.dictionary.len() as u32);
        }

        // Shuffle entries
        let mut shuffled_indices: Vec<usize> = (0..end_positions.len()-1).collect();
        shuffled_indices.shuffle(&mut thread_rng());
        
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
                if next_token_id == 65535 {
                    break 'outer;
                }
                
                // Find the longest match
                let (match_token_id, match_length) = lpm.find_longest_match(&data[pos..end]).unwrap();

                let mut added_token = false;
                if match_length + previous_length <= MAX_LENGTH {
                    // Update token frequency and possibly merge tokens
                    *frequency.entry((previous_token_id, match_token_id)).or_insert(0) += 1;

                    if frequency[&(previous_token_id, match_token_id)] > THRESHOLD {
                        let merged_token = &data[pos - previous_length..pos + match_length];
                        added_token = lpm.insert(merged_token, next_token_id);
                        if added_token {
                            self.dictionary.extend(merged_token);
                            self.dictionary_end_positions.push(self.dictionary.len() as u32);
    
                            frequency.remove(&(previous_token_id, match_token_id));
                            previous_token_id = next_token_id;
                            previous_length = merged_token.len();
                            next_token_id += 1;
                        }
                    }
                }

                if !added_token {
                    previous_token_id = match_token_id;
                    previous_length = match_length;
                }
                
                pos += match_length;
            }
        }
    
        lpm
    }
    
    fn parse(&mut self, data: &[u8], end_positions: &[usize], lpm: &StaticLongestPrefixMatcher) {
        self.item_end_positions.push(0);
    
        for window in end_positions.windows(2) {
            let start = window[0];
            let end = window[1];

            if start == end {
                self.item_end_positions.push(self.compressed_data.len());
                continue;
            }
    
            let mut pos = start;
            while pos < end {
                // Find the longest match
                let (token_id, length) = lpm.find_longest_match(&data[pos..end]).unwrap();
                self.compressed_data.push(token_id);
                pos += length;
            }
    
            self.item_end_positions.push(self.compressed_data.len());
        }
    }
}