use super::Compressor;
use crate::longest_prefix_matcher::lpm16::LongestPrefixMatcher16;
use crate::longest_prefix_matcher::lpm16::StaticLongestPrefixMatcher16;
use crate::bit_vector::BitVector;
use std::collections::BinaryHeap;
use rustc_hash::{FxHashMap, FxHashSet};
use rand::seq::SliceRandom;
use rand::thread_rng;

const MAX_LENGTH: usize = 16;
const SAMPLE_SIZE_PERCENTAGE: f32 = 0.1; // 10% of the data size

type Pair = (u16, u16);

pub struct SampledBPE16Compressor {
    compressed_data: Vec<u16>,                  // Store the compressed data as bytes
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
}

impl Compressor for SampledBPE16Compressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        SampledBPE16Compressor {
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
        "Sampled BPE 16"
    }
}

impl SampledBPE16Compressor {
    fn train(&mut self, data: &[u8], end_positions: &[usize]) -> LongestPrefixMatcher16 {
        let mut lpm = LongestPrefixMatcher16::new();

        // Initialize the dictionary with single-byte tokens
        self.dictionary_end_positions.push(0);
        for i in 0..256 {
            let token = vec![i as u8];
            lpm.insert(&token, i as u16);
            self.dictionary.extend(&token);
            self.dictionary_end_positions.push(self.dictionary.len() as u32);
        }

        // Initialize Token IDs
        let mut token_ids: Vec<u16> = data.iter().map(|&b| b as u16).collect();

        // A bitvector indicates with zeroes the positions of merged bytes.
        let mut bv = BitVector::with_ones(data.len());

        // Strings end positions are used to avoid merging pairs across different strings
        let end_positions_set: FxHashSet<usize> = end_positions.iter().skip(1).copied().collect();

        // Initialize pair positions  
        let mut pair_pos: FxHashMap<Pair, FxHashSet<u32>> = FxHashMap::default();
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
        let mut top_pairs: BinaryHeap<(u32, Pair)> = BinaryHeap::new();
        for (pair, pos_set) in pair_pos.iter() {
            top_pairs.push((pos_set.len() as u32, *pair));
        }

        // Merge pairs
        let mut next_id = 256;
        while !top_pairs.is_empty(){
            // Get the most frequent pair
            let (freq, top_pair) = top_pairs.pop().unwrap();
            let current_freq = pair_pos[&top_pair].len() as u32;
            
            // Check if the frequency is up-to-date
            if freq != current_freq {
                top_pairs.push((current_freq, top_pair));
                continue; 
            }

            // Stop if the most frequent pair has frequency 0
            if current_freq == 0 {
                break;
            }

            // Get the positions of the top pair
            let mut positions= pair_pos.remove(&top_pair).unwrap().into_iter().collect::<Vec<u32>>();
            positions.sort();

            // Let t1 and t2 be the tokens to merge
            let (t1, t2) = top_pair;

            // Add the new token to the dictionary
            let mut merged_token = Vec::new();
            merged_token.extend_from_slice(&self.dictionary[
                self.dictionary_end_positions[t1 as usize] as usize
                ..
                self.dictionary_end_positions[t1 as usize + 1] as usize
            ]);
            merged_token.extend_from_slice(&self.dictionary[
                self.dictionary_end_positions[t2 as usize] as usize
                ..
                self.dictionary_end_positions[t2 as usize + 1] as usize
            ]);
            if !lpm.insert(&merged_token, next_id){
                continue; // If insertion failed, skip this pair
            }
            self.dictionary.extend(&merged_token);
            self.dictionary_end_positions.push(self.dictionary.len() as u32);

            // Keep track of new pairs that will form after merging
            let mut new_pairs: FxHashSet<Pair> = FxHashSet::default();

            // Update occurrences of the top pair
            for &position in positions.iter() {
                // If position was already merged, skip
                if unsafe { !bv.get_unchecked(position as usize) } {
                    continue;
                }

                // We indicate with t0 and t3 the tokens before and after the top pair
                let t1_pos = position as usize;
                let t2_pos = bv.next_one(t1_pos).unwrap();
                let t0_pos = bv.prev_one(t1_pos); // t0_pos is None if t1 is the first token
                let t3_pos = bv.next_one(t2_pos); // t3_pos is None if t2 is the last token

                // Update (t0, t1) and (t0, next_id)  
                if t0_pos.is_some() && !end_positions_set.contains(&t1_pos) {
                    let t0 = token_ids[t0_pos.unwrap()];
                    // Update (t0, t1)
                    if (t0, t1) != top_pair {
                        if let Some(pos_set) = pair_pos.get_mut(&(t0, t1)) {
                            pos_set.remove(&(t0_pos.unwrap() as u32));
                        }
                    }
                    // Update (t0, next_id) if the merged token fits in the 16-byte limit
                    let t0_len = self.dictionary_end_positions[t0 as usize + 1] as usize - self.dictionary_end_positions[t0 as usize] as usize;
                    if t0_len + merged_token.len() <= MAX_LENGTH {
                        new_pairs.insert((t0, next_id));
                        pair_pos
                            .entry((t0, next_id))
                            .or_insert(FxHashSet::default())
                            .insert(t0_pos.unwrap() as u32);
                    }
                }

                // Update (t2, t3) and (next_id, t3)
                if t3_pos.is_some() && !end_positions_set.contains(&t3_pos.unwrap()){
                    let t3 = token_ids[t3_pos.unwrap()];
                    // Update (t2, t3)
                    if (t2, t3) != top_pair {
                        if let Some(pos_set) = pair_pos.get_mut(&(t2, t3)) {
                            pos_set.remove(&(t2_pos as u32));
                        }
                    }
                    // Update (next_id, t3) if the merged token fits in the MAX_LENGTH limit
                    let t3_len = self.dictionary_end_positions[t3 as usize + 1] as usize - self.dictionary_end_positions[t3 as usize] as usize;
                    if t3_len + merged_token.len() <= MAX_LENGTH {
                        new_pairs.insert((next_id, t3));
                        pair_pos
                                .entry((next_id, t3))
                                .or_insert(FxHashSet::default())
                                .insert(t1_pos as u32);
                    }
                }
    
                // set t2_pos to 0 to merge t1 and t2
                bv.set(t2_pos as usize, false);
    
                // Update the token_ids
                token_ids[t1_pos] = next_id;
            }

            // Update the top_pairs heap with new pairs.
            // We don't need to update old pairs because they are already in the heap and their frequency can only decrease; 
            // the check at the beginning of the merge loop ensures we operate with up-to-date frequencies.
            for &new_pair in new_pairs.iter() {
                let freq = pair_pos[&new_pair].len() as u32;
                top_pairs.push((freq, new_pair));
            }
    
            // If the dictionary is full, stop merging
            if next_id == u16::MAX {
                break; 
            }

            // Update the next token ID
            next_id += 1;
        }

        lpm
    }

    fn parse(&mut self, data: &[u8], end_positions: &[usize], lpm: &StaticLongestPrefixMatcher16) {
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