use crate::longest_prefix_matcher::lpm::LongestPrefixMatcher;
use super::Compressor;
use rustc_hash::FxHashMap;
use rand::seq::SliceRandom;
use rand::thread_rng;

const FAST_ACCESS_SIZE: usize = 16;

pub struct OnPairCompressor {
    compressed_data: Vec<u16>,                  // Store the compressed data as token IDs
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
}

impl Compressor for OnPairCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPairCompressor {
            compressed_data: Vec::with_capacity(data_size),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::with_capacity(2 * (1024 * 1024)), // 2 MB
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

        for &token_id in self.compressed_data.iter(){
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

        for &token_id in self.compressed_data[item_start..item_end].iter() {
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
        (self.compressed_data.len() * std::mem::size_of::<u16>()) + self.dictionary.len() + (self.dictionary_end_positions.len() * std::mem::size_of::<u32>())
    }

    fn name(&self) -> &str {
        "OnPair"
    }
}

impl OnPairCompressor {
    fn train(&mut self, data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher {
        self.dictionary_end_positions.push(0);
        
        let mut frequency: FxHashMap<(u16, u16), u16> = FxHashMap::default();
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

        // Initialize the adaptive threshold
        let sample_size = (data.len() as f64 * 0.1) as usize; // 10% of the data size
        let tokens_to_insert = u16::MAX as usize - 255; // 65536 - 256 tokens to insert
        let update_period = ((1.00 + u16::MAX as f64) * 0.001).ceil() as usize; // 0.1% of the dictionary size
        let mut threshold = Threshold::new(sample_size, tokens_to_insert, update_period);
        
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
            threshold.update(match_length, false);
    
            while pos < end {
                // Find the longest match
                let (match_token_id, match_length) = lpm.find_longest_match(&data[pos..end]).unwrap();
    
                 // Update token frequency and possibly merge tokens
                *frequency.entry((previous_token_id, match_token_id)).or_insert(0) += 1;
    
                if frequency[&(previous_token_id, match_token_id)] > threshold.get() {
                    let merged_token = &data[pos - previous_length..pos + match_length];
                    lpm.insert(merged_token, next_token_id);
                    self.dictionary.extend(merged_token);
                    self.dictionary_end_positions.push(self.dictionary.len() as u32);
                    
                    frequency.remove(&(previous_token_id, match_token_id));
                    previous_token_id = next_token_id;
                    previous_length = merged_token.len();

                    if next_token_id == u16::MAX {
                        break 'outer;
                    }

                    next_token_id += 1;
                    threshold.update(match_length, true);
                }
                else {
                    previous_token_id = match_token_id;
                    previous_length = match_length;
                    threshold.update(match_length, false);
                }
            
                pos += match_length;
            }
        }
    
        lpm
    }
    
    fn parse(&mut self, data: &[u8], end_positions: &[usize], lpm: &LongestPrefixMatcher) {
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

struct Threshold {
    threshold: u16,                     // The dynamic threshold value
    target_sample_size: usize,          // Target number of bytes to process before stopping
    current_sample_size: usize,         // Total bytes processed so far
    tokens_to_insert: usize,            // Number of tokens needed to fully populate the dictionary 
    update_period: usize,               // How many token insertions before we update the threshold
    current_update_merges: usize,       // Number of tokens inserted in the current update batch
    current_update_bytes: usize,        // Number of bytes processed in the current update batch
}    

impl Threshold {
    fn new(target_sample_size: usize, tokens_to_insert: usize, update_period: usize) -> Self {
        Threshold {
            threshold: 0,
            target_sample_size,
            current_sample_size: 0, 
            tokens_to_insert,
            update_period,
            current_update_merges: 0,
            current_update_bytes: 0,
        }
    }

    #[inline]
    fn get(&self) -> u16 {
        self.threshold
    }

    #[inline]
    fn update(&mut self, match_length: usize, did_merge: bool) {
        self.current_update_bytes += match_length;
        self.current_sample_size += match_length;

        if did_merge {
            self.tokens_to_insert -= 1;
            self.current_update_merges += 1;

            if self.current_update_merges == self.update_period {
                let bytes_per_token = (self.current_update_bytes as f64 / self.current_update_merges as f64).ceil() as usize;
                let predicted_missing_bytes = self.tokens_to_insert * bytes_per_token;
                let predicted_sample_size = self.current_sample_size + predicted_missing_bytes;

                if predicted_sample_size > self.target_sample_size {
                    self.threshold = self.threshold.saturating_sub(1);
                }
                else if predicted_sample_size < self.target_sample_size {
                    self.threshold = self.threshold.saturating_add(1);
                }

                self.current_update_bytes = 0;
                self.current_update_merges = 0;
            }
        }
    }
}