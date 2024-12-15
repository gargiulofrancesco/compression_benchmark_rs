use crate::longest_prefix_matcher::LongestPrefixMatcher;
use super::Compressor;
use rustc_hash::FxHashMap;

const THRESHOLD: usize = 10;
const FAST_ACCESS_SIZE: usize = 16;

pub struct OnPairCompressor {
    data: Vec<u16>,                             // Store the compressed data as token IDs
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
}

impl Compressor for OnPairCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPairCompressor {
            data: Vec::with_capacity(data_size),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::new(),
            dictionary_end_positions: Vec::new(),
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let lpm = OnPairCompressor::train(data, end_positions);
        self.parse(data, end_positions, &lpm);
    }

    fn decompress(&self, buffer: &mut Vec<u8>) {
        let mut total_length = 0;

        for &token_id in self.data.iter() {
            let start = self.dictionary_end_positions[token_id as usize] as usize;
            let end = self.dictionary_end_positions[token_id as usize + 1] as usize;
            let length = end - start;
    
            unsafe {
                // Fast path for short strings
                let src_ptr = self.dictionary.as_ptr().add(start);
                let dst_ptr = buffer.as_mut_ptr().add(total_length);
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, FAST_ACCESS_SIZE);
                
                if length > FAST_ACCESS_SIZE {
                    let src_ptr: *const u8 = self.dictionary.as_ptr().add(start + FAST_ACCESS_SIZE);
                    let dst_ptr: *mut u8 = buffer.as_mut_ptr().add(total_length + FAST_ACCESS_SIZE);
                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, length - FAST_ACCESS_SIZE);
                }

                total_length += length;
            }
        }
    
        // Set the final length of the buffer safely
        unsafe {
            buffer.set_len(total_length);
        }
    }

    fn get_item_at(&mut self, index: usize, buffer: &mut Vec<u8>) {
        let item_start = self.item_end_positions[index];
        let item_end = self.item_end_positions[index + 1];
        let mut total_length = 0;

        for &token_id in self.data[item_start..item_end].iter() {
            let start = self.dictionary_end_positions[token_id as usize] as usize;
            let end = self.dictionary_end_positions[token_id as usize + 1] as usize;
            let length = end - start;
    
            unsafe {
                // Fast path for short strings
                let src_ptr = self.dictionary.as_ptr().add(start);
                let dst_ptr = buffer.as_mut_ptr().add(total_length);
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, FAST_ACCESS_SIZE);
                
                if length > FAST_ACCESS_SIZE {
                    let src_ptr: *const u8 = self.dictionary.as_ptr().add(start + FAST_ACCESS_SIZE);
                    let dst_ptr: *mut u8 = buffer.as_mut_ptr().add(total_length + FAST_ACCESS_SIZE);
                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, length - FAST_ACCESS_SIZE);
                }

                total_length += length;
            }
        }
    
        // Set the final length of the buffer safely
        unsafe {
            buffer.set_len(total_length);
        }
    }

    fn space_used_bytes(&self) -> usize {
        (self.data.len() * 2) + self.dictionary.len() + (self.dictionary_end_positions.len() * 4)
    }

    fn name(&self) -> &str {
        "On-Pair"
    }
}

impl OnPairCompressor {
    fn train(data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher<u16> {
        let mut frequency: FxHashMap<(u16, u16), usize> = FxHashMap::default();
        let mut lpm = LongestPrefixMatcher::new();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = vec![i as u8];
            lpm.insert(&token, i as u16);
        }

        let mut start = 0;
        let mut pos = 0;
        
        'outer: for &end in end_positions.iter() {
            if start == end {
                continue;
            }
    
            let (match_token_id, match_length) = lpm.find_longest_match(&data[pos..end]).unwrap();
            let mut previous_token_id = match_token_id;
            let mut previous_length = match_length;

            pos = start + previous_length;
    
            while pos < end {
                if next_token_id == 65535 {
                    break 'outer;
                }
                
                // Find the longest match
                let (match_token_id, match_length) = lpm.find_longest_match(&data[pos..end]).unwrap();
    
                 // Update token frequency and possibly merge tokens
                *frequency.entry((previous_token_id, match_token_id)).or_insert(0) += 1;
    
                if frequency[&(previous_token_id, match_token_id)] > THRESHOLD {
                    let merged_token = &data[pos - previous_length..pos + match_length];
                    lpm.insert(merged_token, next_token_id);
                    next_token_id += 1;
                    frequency.remove(&(previous_token_id, match_token_id));
                }
            
                previous_token_id = match_token_id;
                previous_length = match_length;
    
                pos += match_length;
            }
    
            start = end;
        }
    
        lpm
    }
    
    fn parse(&mut self, data: &[u8], end_positions: &[usize], lpm: &LongestPrefixMatcher<u16>) {
        // Initialize dictionary metadata
        self.dictionary_end_positions.push(0);
        self.item_end_positions.push(0);
    
        let mut dictionary_map: Vec<Option<u16>> = vec![None; 1<<16];
        let mut next_token_id: u16 = 0;
    
        let mut start = 0;
        for &end in end_positions.iter() {
            if start == end {
                self.item_end_positions.push(self.data.len());
                continue;
            }
    
            let mut pos = start;
            while pos < end {
                // Find the longest match
                let (match_token_id, length) = lpm.find_longest_match(&data[pos..end]).unwrap();
    
                if let Some(existing_token_id) = dictionary_map[match_token_id as usize] {
                    self.data.push(existing_token_id as u16);
                } else {
                    self.data.push(next_token_id as u16);
                    dictionary_map[match_token_id as usize] = Some(next_token_id);

                    self.dictionary.extend(&data[pos..pos + length]);
                    self.dictionary_end_positions.push(self.dictionary.len() as u32);
    
                    next_token_id += 1;
                }
    
                pos += length;
            }
    
            self.item_end_positions.push(self.data.len());
            start = end;
        }
    }    
}
