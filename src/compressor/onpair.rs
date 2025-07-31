//! OnPair compression algorithm for string collections
//!
//! Implements a two-phase compression strategy optimized for random access:
//! 1. **Training Phase**: Discovers frequent adjacent token pairs using longest prefix matching
//! 2. **Parsing Phase**: Compresses strings independently using the learned dictionary
//!
//! The algorithm maintains a 65,536-token dictionary with 2-byte token IDs.

use crate::longest_prefix_matcher::lpm::LongestPrefixMatcher;
use super::Compressor;
use rustc_hash::FxHashMap;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Optimization constant for memory copy operations
const FAST_ACCESS_SIZE: usize = 16;

/// OnPair compressor with unlimited token length
/// 
/// Core implementation of the OnPair algorithm supporting arbitrary-length tokens.
pub struct OnPairCompressor {
    compressed_data: Vec<u16>,              // Token ID sequences (2 bytes per token)
    item_end_positions: Vec<usize>,         // Compressed string boundaries
    dictionary: Vec<u8>,                    // Token definitions (variable length)
    dictionary_end_positions: Vec<u32>,     // Token boundary positions in dictionary
}

impl Compressor for OnPairCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPairCompressor {
            compressed_data: Vec::with_capacity(data_size),
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
        (self.compressed_data.len() * std::mem::size_of::<u16>()) 
        + self.dictionary.len() 
        + (self.dictionary_end_positions.len() * std::mem::size_of::<u32>())
    }

    fn name(&self) -> &str {
        "OnPair"
    }
}

impl OnPairCompressor {
    /// Phase 1: Dictionary population
    /// 
    /// Uses longest prefix matching to parse training data and identify frequent
    /// adjacent token pairs.
    /// 
    /// # Algorithm
    /// 1. Initialize 256 single-byte tokens  
    /// 2. Parse shuffled training data with longest prefix matching
    /// 3. Track adjacent token pair frequencies
    /// 4. Merge frequent pairs into new tokens until dictionary full (65,536 tokens)
    fn train(&mut self, data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher<u16> {
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

        // Set the threshold for merging tokens
        let data_size_mib = data.len() as f64 / (1024.0 * 1024.0);
        let threshold = data_size_mib.log2().max(2.0) as u16;
        
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

                    if next_token_id == u16::MAX {
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
    
    /// Phase 2: String compression using learned dictionary
    /// 
    /// Compresses each string independently by greedily applying longest prefix matching
    /// with the constructed dictionary. Each string becomes a sequence of token IDs.
    fn parse(&mut self, data: &[u8], end_positions: &[usize], lpm: &LongestPrefixMatcher<u16>) {
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
