use std::collections::BinaryHeap;
use crate::bit_vector::BitVector;
use rustc_hash::{FxHashMap, FxHashSet};


pub struct BPECompressor<'a> {
    data: &'a [u8],
    next_id: u32,
}

impl<'a> BPECompressor<'a> {
    pub fn new(data: &'a [u8]) -> BPECompressor {
        BPECompressor {
            data,
            next_id: 256,
        }
    }

    pub fn compress(&mut self, n_iterations: usize, tokenize_words: bool, verbose: bool) {
        let mut bv= self.tokenize(tokenize_words);
        let mut token_ids = self.initialize_token_ids(&bv);
        let (mut pair_pos, mut max_freq) = self.initialize_pair_positions(&bv, &token_ids);
        self.merge(n_iterations, verbose, &mut bv, &mut token_ids, &mut pair_pos, &mut max_freq);
        let (token_ids, dictionary, separators) = self.remap_by_frequency(&bv, &token_ids);    
    }

    fn tokenize(&mut self, tokenize_words: bool) -> BitVector {
        if tokenize_words {
            self.tokenize_words()
        }
        else {
            self.tokenize_bytes()
        }
    }

    fn tokenize_bytes(&mut self) -> BitVector {
        let mut bv = BitVector::with_ones(self.data.len());
        bv.shrink_to_fit();

        bv
    }

    fn tokenize_words(&mut self) -> BitVector {
        let mut bv = BitVector::with_zeroes(self.data.len());
        bv.set(0, true);
        let mut prev_not_alphanumeric = if self.data[0].is_ascii_alphanumeric() { false } else { true };

        for i in 1..bv.len() {
            if !self.data[i].is_ascii_alphanumeric() {
                bv.set(i, true);
                prev_not_alphanumeric = true;
            }
            else {
                if prev_not_alphanumeric {
                    bv.set(i, true);
                }
                prev_not_alphanumeric = false;
            }
        }

        bv.shrink_to_fit();

        bv
    }

    fn initialize_token_ids(&mut self, bv: &BitVector) -> Vec<u32> {
        let mut token_ids: Vec<u32> = vec![0; self.data.len()];
        let mut map: FxHashMap<Vec<u8>, u32> = FxHashMap::default();

        let mut iter = bv.ones(0);
        let mut current_pos = iter.next().unwrap();
        let mut next_pos_opt = iter.next();

        while let Some(next_pos) = next_pos_opt {
            let slice = &self.data[current_pos..next_pos];
            let token_id = self.get_or_insert_token(&mut map, slice);
            token_ids[current_pos] = token_id;

            current_pos = next_pos;
            next_pos_opt = iter.next();
        }

        // Add the last token
        let slice = &self.data[current_pos..];
        let token_id = self.get_or_insert_token(&mut map, slice);
        token_ids[current_pos] = token_id;

        token_ids.shrink_to_fit();

        token_ids
    }
        
    #[inline(always)]
    fn get_or_insert_token(&mut self, map: &mut FxHashMap<Vec<u8>, u32>, slice: &[u8]) -> u32 {
        if slice.len() == 1 {
            slice[0] as u32
        }
        else if let Some(&token_id) = map.get(slice) {
            token_id
        }
        else {
            map.insert(slice.to_vec(), self.next_id);
            self.next_id += 1;
            self.next_id - 1
        }
    }

    pub fn initialize_pair_positions(&mut self, bv: &BitVector, token_ids: &[u32]) -> (FxHashMap<(u32, u32), FxHashSet<u32>>, BinaryHeap<(u32, (u32, u32))>) {
        let mut pair_pos: FxHashMap<(u32, u32), FxHashSet<u32>> = FxHashMap::default();
        let mut max_freq: BinaryHeap<(u32, (u32, u32))> = BinaryHeap::new();
        
        let mut iter = bv.ones(0);
        let mut current_pos = iter.next().unwrap();
        let mut next_pos_opt = iter.next();

        while let Some(next_pos) = next_pos_opt {
            let t1 = token_ids[current_pos];
            let t2 = token_ids[next_pos];
            pair_pos
                .entry((t1, t2))
                .or_insert(FxHashSet::default())
                .insert(current_pos as u32);

            current_pos = next_pos;
            next_pos_opt = iter.next();
        }
    
        for (pair, pos_set) in pair_pos.iter() {
            max_freq.push((pos_set.len() as u32, *pair));
        }

        (pair_pos, max_freq)
    }

    pub fn merge(&mut self, n_iterations: usize, verbose: bool, bv: &mut BitVector, token_ids: &mut [u32], pair_pos: &mut FxHashMap<(u32, u32), FxHashSet<u32>>, max_freq: &mut BinaryHeap<(u32, (u32, u32))>) {
        for _ in 0..n_iterations {
            // Store updated pairs to minimize insertions in the max_freq heap
            let mut updated_pairs: FxHashSet<(u32, u32)> = FxHashSet::default();

            // Get the pair with the maximum frequency
            let (freq, (t1, t2)) = loop {
                let (freq, (t1, t2)) = max_freq.pop().unwrap();
                let current_freq = pair_pos.get(&(t1, t2)).unwrap().len() as u32;
                
                // Check if the frequency is up-to-date
                if freq == current_freq {
                    break (freq, (t1, t2));  // Exit loop with valid pair
                }
            };

            // Get the positions of the pair (t1, t2)
            let mut positions= pair_pos.remove(&(t1, t2)).unwrap().into_iter().collect::<Vec<u32>>();
            positions.sort();

            // Print the top pair (t1, t2) if verbose
            if verbose {
                let temp_start_slice = positions[0] as usize;
                let temp_middle_slice = bv.next_one(positions[0] as usize).unwrap();
                let temp_end_slice = bv.next_one(temp_middle_slice).unwrap_or(self.data.len());
                println!("{}: \"{}\", freq: {}", self.next_id, String::from_utf8_lossy(&self.data[temp_start_slice..temp_end_slice]), freq);    
            }

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

                // Get the previous token (if it exists)
                let t0 = t0_pos.map(|t0_pos| token_ids[t0_pos]);

                // Get the next token (if it exists)
                let t3 = t3_pos.map(|t3_pos| token_ids[t3_pos]);

                // Update (t0, t1) and (t0, next_id)                
                if let Some(t0) = t0 {
                    updated_pairs.insert((t0, t1));
                    updated_pairs.insert((t0, self.next_id));
                    // Update the pair (t0, t1)
                    if let Some(pos_set) = pair_pos.get_mut(&(t0, t1)) {
                        pos_set.remove(&(t0_pos.unwrap() as u32));
                    }
                    // Update the pair (t0, next_id)
                    pair_pos
                        .entry((t0, self.next_id))
                        .or_insert(FxHashSet::default())
                        .insert(t0_pos.unwrap() as u32);
                }

                // Update (t2, t3) and (next_id, t3)
                if let Some(t3) = t3 {
                    updated_pairs.insert((t2, t3));
                    updated_pairs.insert((self.next_id, t3));
                    // Update the pair (t2, t3)
                    if let Some(pos_set) = pair_pos.get_mut(&(t2, t3)) {
                        pos_set.remove(&(t2_pos as u32));
                    }
                    // Update the pair (next_id, t3)
                    pair_pos
                        .entry((self.next_id, t3))
                        .or_insert(FxHashSet::default())
                        .insert(t1_pos as u32);
                }

                // set t2_pos to 0 to merge t1 and t2
                bv.set(t2_pos as usize, false);

                // Update the token_ids
                token_ids[t1_pos] = self.next_id;
            }

            // Update the max_freq heap with updated pairs
            for &(ti, tj) in updated_pairs.iter() {
                if (ti, tj) != (t1, t2) {
                    let freq = pair_pos.get(&(ti, tj)).unwrap().len() as u32;
                    max_freq.push((freq, (ti, tj)));
                }
            }

            self.next_id += 1;
        }

    }

    fn remap_by_frequency(&self, bv: &BitVector, token_ids: &[u32]) -> (Vec<u32>, Vec<u8>, Vec<usize>) {
        let mut frequency_map: FxHashMap<u32, u32> = FxHashMap::default();
        let mut dictionary_map: FxHashMap<u32, Vec<u8>> = FxHashMap::default();
        let mut remapped_token_ids = Vec::new();
        
        let mut iter = bv.ones(0);
        let mut current_pos = iter.next().unwrap();
        let mut next_pos_opt = iter.next();
    
        while let Some(next_pos) = next_pos_opt {
            let slice = &self.data[current_pos..next_pos];
            let token_id = token_ids[current_pos];
            *frequency_map.entry(token_id).or_insert(0) += 1;
            dictionary_map.entry(token_id).or_insert(slice.to_vec());
    
            remapped_token_ids.push(token_id);
            current_pos = next_pos;
            next_pos_opt = iter.next();
        }
    
        // Add the last token
        let slice = &self.data[current_pos..];
        let token_id = token_ids[current_pos];
        remapped_token_ids.push(token_id);
        *frequency_map.entry(token_id).or_insert(0) += 1;
        dictionary_map.entry(token_id).or_insert(slice.to_vec());


        // Step 2: Collect (element, frequency) pairs and sort by frequency
        let mut frequency_vec: Vec<(u32, u32)> = frequency_map.into_iter().collect();
        frequency_vec.sort_by(|a, b| b.1.cmp(&a.1)); 
        
        let mut dictionary = Vec::new();
        let mut separators  = vec![0];

        for &(token_id, freq) in frequency_vec.iter() {
            let bytes = dictionary_map.get(&token_id).unwrap();
            dictionary.extend_from_slice(bytes);
            separators.push(separators.last().unwrap() + bytes.len());
        }

        remapped_token_ids.shrink_to_fit();
        dictionary.shrink_to_fit();
        separators.shrink_to_fit();

        (remapped_token_ids, dictionary, separators)
    }

}