use super::Compressor;
use crate::longest_prefix_matcher::lpm::LongestPrefixMatcher;
use crate::bit_vector::BitVector;
use std::collections::BinaryHeap;
use rustc_hash::{FxHashMap, FxHashSet};
use rand::seq::SliceRandom;
use rand::thread_rng;

const MAX_TOKENS: u16 = 65535;
const FAST_ACCESS_SIZE: usize = 16;
const SAMPLE_SIZE_PERCENTAGE: f32 = 0.05; 

pub struct BPELPMCompressor {
    compressed_data: Vec<u16>,                  // Store the compressed data as bytes
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
}

impl Compressor for BPELPMCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        BPELPMCompressor {
            compressed_data: Vec::with_capacity(data_size),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::new(),
            dictionary_end_positions: Vec::new(),
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let sample_size = (data.len() as f32 * SAMPLE_SIZE_PERCENTAGE) as usize;
        let (sampled_data, sampled_end_positions) = Self::sampling(data, end_positions, sample_size);
        let lpm = self.train(&sampled_data, &sampled_end_positions);
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
        "BPE_LPM"
    }
}

impl BPELPMCompressor {
    fn train(&mut self, data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher {
        let mut lpm = LongestPrefixMatcher::new();
        self.dictionary_end_positions.push(0);

        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = vec![i as u8];
            lpm.insert(&token, i as u16);
            self.dictionary.extend(&token);
            self.dictionary_end_positions.push(self.dictionary.len() as u32);
        }

        // Initialize Token IDs
        let mut token_ids: Vec<u16> = data.iter().map(|&b| b as u16).collect();

        // The bitvector inidicates with zeroes the positions of merged bytes
        let mut bv = BitVector::with_ones(data.len());
        let end_positions_set: FxHashSet<usize> = end_positions.iter().skip(1).copied().collect();

        // Initialize pair positions  
        let mut pair_pos: FxHashMap<(u16, u16), FxHashSet<u32>> = FxHashMap::default();
        for i in 0..data.len()-1 {
            if end_positions_set.contains(&(i+1)) {
                continue;
            }
            let t1 = token_ids[i];
            let t2 = token_ids[i+1];
            pair_pos
                .entry((t1, t2))
                .or_insert(FxHashSet::default())
                .insert(i as u32);
        }

        // Initialize heap tracking the most frequent pairs
        let mut max_freq: BinaryHeap<(u32, (u16, u16))> = BinaryHeap::new();
        for (pair, pos_set) in pair_pos.iter() {
            max_freq.push((pos_set.len() as u32, *pair));
        }

        // Merge pairs
        let mut next_id = 256;
        while next_id<MAX_TOKENS && !max_freq.is_empty(){
            let (freq, (t1, t2)) = max_freq.pop().unwrap();
            let current_freq = pair_pos.get(&(t1, t2)).unwrap().len() as u32;
            if freq != current_freq {
                continue; // Skip if the frequency is not up-to-date
            }

            // Get the positions of the pair (t1, t2)
            let mut positions= pair_pos.remove(&(t1, t2)).unwrap().into_iter().collect::<Vec<u32>>();
            positions.sort();

            // Add the new token to the dictionary
            let t1_start = self.dictionary_end_positions[t1 as usize] as usize;
            let t1_end = self.dictionary_end_positions[t1 as usize + 1] as usize;
            let t2_start = self.dictionary_end_positions[t2 as usize] as usize;
            let t2_end = self.dictionary_end_positions[t2 as usize + 1] as usize;
            let mut merged_token = Vec::new();
            merged_token.extend_from_slice(&self.dictionary[t1_start..t1_end]);
            merged_token.extend_from_slice(&self.dictionary[t2_start..t2_end]);
            self.dictionary.extend(&merged_token);
            self.dictionary_end_positions.push(self.dictionary.len() as u32);
            lpm.insert(&merged_token, next_id);

            // Store updated pairs to minimize insertions in the max_freq heap
            let mut updated_pairs: FxHashSet<(u16, u16)> = FxHashSet::default();

            // Update occurrences of (t1, t2)
            for &position in positions.iter() {
                // If position was already merged, skip
                if bv.get(position as usize).unwrap() == false {
                    continue;
                }
                
                let t1_pos = position as usize;
                let t2_pos = bv.next_one(t1_pos).unwrap();
                let t0_pos = bv.prev_one(t1_pos);
                let t3_pos = bv.next_one(t2_pos);
    
                // Update (t0, t1) and (t0, next_id)  
                if t0_pos.is_some() && !end_positions_set.contains(&t1_pos) {
                    let t0 = token_ids[t0_pos.unwrap()];
                    // Update (t0, t1)
                    if (t0, t1) != (t1, t2) {
                        pair_pos.get_mut(&(t0, t1)).unwrap().remove(&(t0_pos.unwrap() as u32));
                        updated_pairs.insert((t0, t1));
                    }
                    // Update (t0, next_id)
                    pair_pos
                            .entry((t0, next_id))
                            .or_insert(FxHashSet::default())
                            .insert(t0_pos.unwrap() as u32);
                    updated_pairs.insert((t0, next_id));
                }

                // Update (t2, t3) and (next_id, t3)
                if t3_pos.is_some() && !end_positions_set.contains(&t3_pos.unwrap()){
                    let t3 = token_ids[t3_pos.unwrap()];
                    // Update (t2, t3)
                    if (t2, t3) != (t1, t2) {
                        pair_pos.get_mut(&(t2, t3)).unwrap().remove(&(t2_pos as u32));
                        updated_pairs.insert((t2, t3));
                    }
                    // Update (next_id, t3)
                    pair_pos
                            .entry((next_id, t3))
                            .or_insert(FxHashSet::default())
                            .insert(t1_pos as u32);
                    updated_pairs.insert((next_id, t3));
                }

                // set t2_pos to 0 to merge t1 and t2
                bv.set(t2_pos as usize, false);
    
                // Update the token_ids
                token_ids[t1_pos] = next_id;
            }
    
            // Update the max_freq heap with updated pairs
            for &pair in updated_pairs.iter() {
                let freq = pair_pos.get(&pair).unwrap().len() as u32;
                max_freq.push((freq, pair));
            }
    
            next_id += 1;
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

    fn sampling(data: &[u8], end_positions: &[usize], sample_size: usize) -> (Vec<u8>, Vec<usize>) {
        let n = end_positions.len() - 1;
        let mut sampled_indices: Vec<usize> = (0..n).collect();
        sampled_indices.shuffle(&mut thread_rng());

        let mut sampled_data = Vec::new();
        let mut sampled_end_positions = Vec::with_capacity(n + 1);
        sampled_end_positions.push(0);

        for &index in &sampled_indices {
            let start = end_positions[index];
            let end = end_positions[index + 1];
            let item = &data[start..end];

            if sampled_data.len() + item.len() > sample_size {
                break;
            }

            sampled_data.extend_from_slice(item);
            sampled_end_positions.push(sampled_data.len());
        }

        (sampled_data, sampled_end_positions)
    }
}