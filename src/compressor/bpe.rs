use std::collections::BinaryHeap;
use crate::bit_vector::BitVector;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct BPECompressor<'a> {
    data: &'a [u8],
    bv: BitVector,
    token_ids: Vec<u32>,    // token_ids[i] = id of the token at position i
    next_id: u32,

    pair_pos: FxHashMap<(u32, u32), FxHashSet<u32>>,    // (token1, token2) -> bv positions
    max_freq: BinaryHeap<(u32, (u32, u32))>,            // (frequency, (token1, token2))
}

impl<'a> BPECompressor<'a> {
    pub fn new(data: &'a [u8]) -> BPECompressor {
        BPECompressor{
            data,
            bv: BitVector::with_zeroes(data.len()),
            token_ids: vec![0; data.len()],
            next_id: 256,

            pair_pos: FxHashMap::default(),
            max_freq: BinaryHeap::new(),
        }
    }

    pub fn tokenize_bytes(&mut self){
        self.bv = BitVector::with_ones(self.bv.len());
    }

    pub fn tokenize_words(&mut self) {
        self.bv.set(0, true);
        let mut prev_not_alphanumeric = if self.data[0].is_ascii_alphanumeric() { false } else { true };

        for i in 1..self.bv.len() {
            if !self.data[i].is_ascii_alphanumeric() {
                self.bv.set(i, true);
                prev_not_alphanumeric = true;
            }
            else {
                if prev_not_alphanumeric {
                    self.bv.set(i, true);
                }
                prev_not_alphanumeric = false;
            }
        }
    }

    pub fn initialize_token_ids(&mut self) {
        let mut map: FxHashMap<Vec<u8>, u32> = FxHashMap::default();

        let mut iter = self.bv.ones(0);
        let mut current_pos = iter.next().unwrap();
        let mut next_pos_opt = iter.next();

        while let Some(next_pos) = next_pos_opt {
            let slice = &self.data[current_pos..next_pos];
            if slice.len() == 1 {
                self.token_ids[current_pos] = slice[0] as u32;
            }
            else if let Some(&token_id) = map.get(slice) {
                self.token_ids[current_pos] = token_id;
            }
            else {
                map.insert(slice.to_vec(), self.next_id);
                self.token_ids[current_pos] = self.next_id;
                self.next_id += 1;
            }

            current_pos = next_pos;
            next_pos_opt = iter.next();
        }

        // Add the last token
        let slice = &self.data[current_pos..];
        if slice.len() == 1 {
            self.token_ids[current_pos] = slice[0] as u32;
        }
        else if let Some(&token_id) = map.get(slice) {
            self.token_ids[current_pos] = token_id;
        }
        else {
            map.insert(slice.to_vec(), self.next_id);
            self.token_ids[current_pos] = self.next_id;
            self.next_id += 1;
        }
    }

    pub fn initialize_pair_positions(&mut self) {
        let mut iter = self.bv.ones(0);
        let mut current_pos = iter.next().unwrap();
        let mut next_pos_opt = iter.next();

        while let Some(next_pos) = next_pos_opt {
            let t1 = self.token_ids[current_pos];
            let t2 = self.token_ids[next_pos];
            self.pair_pos
                .entry((t1, t2))
                .or_insert(FxHashSet::default())
                .insert(current_pos as u32);

            current_pos = next_pos;
            next_pos_opt = iter.next();
        }
    
        for (pair, pos_set) in self.pair_pos.iter() {
            self.max_freq.push((pos_set.len() as u32, *pair));
        }
    }

    pub fn merge(&mut self, n_iterations: usize, verbose: bool) {
        for _ in 0..n_iterations {
            // Store updated pairs to minimize insertions in the max_freq heap
            let mut updated_pairs: FxHashSet<(u32, u32)> = FxHashSet::default();

            // Get the pair with the maximum frequency
            let (_, (t1, t2)) = loop {
                let (freq, (t1, t2)) = self.max_freq.pop().unwrap();
                let current_freq = self.pair_pos.get(&(t1, t2)).unwrap().len() as u32;
                
                // Check if the frequency is up-to-date
                if freq == current_freq {
                    break (freq, (t1, t2));  // Exit loop with valid pair
                }
            };

            // Get the positions of the pair (t1, t2)
            let mut positions: Vec<u32> = self.pair_pos.remove(&(t1, t2)).unwrap().iter().copied().collect();
            positions.sort();

            // Print the top pair (t1, t2) if verbose
            if verbose {
                let temp_start_slice = positions[0] as usize;
                let temp_middle_slice = self.bv.next_one(positions[0] as usize).unwrap();
                let temp_end_slice = self.bv.next_one(temp_middle_slice).unwrap_or(self.data.len());
                println!("{}: \"{}\"", self.next_id, String::from_utf8_lossy(&self.data[temp_start_slice..temp_end_slice]));    
            }

            // Update occurrences of (t1, t2)
            for &position in positions.iter() {
                // If position was already merged, skip
                if self.bv.get(position as usize).unwrap() == false {
                    continue;
                }
                
                let t1_pos = position as usize;
                let t2_pos = self.bv.next_one(t1_pos).unwrap();
                let t0_pos = self.bv.prev_one(t1_pos);
                let t3_pos = self.bv.next_one(t2_pos);

                // Get the previous token (if it exists)
                let t0 = t0_pos.map(|t0_pos| self.token_ids[t0_pos]);

                // Get the next token (if it exists)
                let t3 = t3_pos.map(|t3_pos| self.token_ids[t3_pos]);

                // Update (t0, t1) and (t0, next_id)                
                if let Some(t0) = t0 {
                    updated_pairs.insert((t0, t1));
                    updated_pairs.insert((t0, self.next_id));
                    // Update the pair (t0, t1)
                    if let Some(pos_set) = self.pair_pos.get_mut(&(t0, t1)) {
                        pos_set.remove(&(t0_pos.unwrap() as u32));
                    }
                    // Update the pair (t0, next_id)
                    self.pair_pos
                        .entry((t0, self.next_id))
                        .or_insert(FxHashSet::default())
                        .insert(t0_pos.unwrap() as u32);
                }

                // Update (t2, t3) and (next_id, t3)
                if let Some(t3) = t3 {
                    updated_pairs.insert((t2, t3));
                    updated_pairs.insert((self.next_id, t3));
                    // Update the pair (t2, t3)
                    if let Some(pos_set) = self.pair_pos.get_mut(&(t2, t3)) {
                        pos_set.remove(&(t2_pos as u32));
                    }
                    // Update the pair (next_id, t3)
                    self.pair_pos
                        .entry((self.next_id, t3))
                        .or_insert(FxHashSet::default())
                        .insert(t1_pos as u32);
                }

                // set t2_pos to 0 to merge t1 and t2
                self.bv.set(t2_pos as usize, false);

                // Update the token_ids
                self.token_ids[t1_pos] = self.next_id;
            }

            // Update the max_freq heap with updated pairs
            for &(ti, tj) in updated_pairs.iter() {
                if (ti, tj) != (t1, t2) {
                    let freq = self.pair_pos.get(&(ti, tj)).unwrap().len() as u32;
                    self.max_freq.push((freq, (ti, tj)));
                }
            }

            self.next_id += 1;
        }
    }
}