use std::collections::BinaryHeap;
use crate::bit_vector::BitVector;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct BPECompressor<'a> {
    pub data: &'a [u8],
    pub bv: BitVector,
    pub pair_pos: FxHashMap<(u32, u32), FxHashSet<u32>>,    // (token1, token2) -> bv positions
    pub pair_freq: FxHashMap<(u32, u32), u32>,              // (token1, token2) -> frequency
    pub max_freq: BinaryHeap<(u32, (u32, u32))>,            // (frequency, (token1, token2))
    bytes_to_token: FxHashMap<Vec<u8>, u32>,
}

impl<'a> BPECompressor<'a> {
    pub fn new(data: &'a [u8]) -> BPECompressor {
        // Initialize the dictionary with the first 256 ASCII characters
        let bytes_to_token: FxHashMap<Vec<u8>, u32> = (0..256)
            .map(|i| (vec![i as u8], i as u32))
            .collect();

            BPECompressor{
                data,
                bv: BitVector::with_zeroes(data.len()),
                pair_pos: FxHashMap::default(),
                pair_freq: FxHashMap::default(),
                max_freq: BinaryHeap::new(),
                bytes_to_token,
        }
    }

    pub fn tokenize(&mut self) {
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

    pub fn initialize(&mut self) {
        let mut t0_pos = 0;
        let mut t1_pos = self.bv.next_one(t0_pos).unwrap();
        let mut t2_pos_opt = self.bv.next_one(t1_pos);

        let mut t0 = self.get_or_insert_token(&self.data[t0_pos..t1_pos]);
        let mut t1;

        while let Some(t2_pos) = t2_pos_opt {
            t1 = self.get_or_insert_token(&self.data[t1_pos..t2_pos]);
            
            self.insert_pair((t0, t1), t0_pos as u32);
            
            t2_pos_opt = self.bv.next_one(t2_pos);
            t0 = t1;
            t0_pos = t1_pos;
            t1_pos = t2_pos;
        }

        // Process the last token if necessary
        t1 = self.get_or_insert_token(&self.data[t1_pos..]);
        self.insert_pair((t0, t1), t0_pos as u32);

        // Insert pair_freq into max_freq
        for (pair, freq) in self.pair_freq.iter() {
            self.max_freq.push((*freq, *pair));
        }
    }

    #[inline(always)]
    fn get_or_insert_token(&mut self, token: &[u8]) -> u32 {
        if token.len() == 1 {
            return token[0] as u32;
        }

        if let Some(&token_id) = self.bytes_to_token.get(token) {
            return token_id;
        }

        let token_id = self.bytes_to_token.len() as u32;
        self.bytes_to_token.insert(token.to_vec(), token_id);
        
        token_id
    }

    #[inline(always)]
    fn insert_pair(&mut self, (t1, t2): (u32, u32), pos: u32) {
        self.pair_pos
            .entry((t1, t2))
            .or_insert(FxHashSet::default())
            .insert(pos);
    
        *self.pair_freq
            .entry((t1, t2))
            .or_insert(0) += 1;
    }

    pub fn compress(&mut self, n_iterations: usize) {
        for _ in 0..n_iterations {
            let next_id = self.bytes_to_token.len() as u32;

            // Store updated pairs to minimize insertions in the max_freq heap
            let mut updated_pairs: FxHashSet<(u32, u32)> = FxHashSet::default();

            // Get the pair with the maximum frequency
            let (_, (t1, t2)) = loop {
                let (freq, (t1, t2)) = self.max_freq.pop().unwrap();
                let current_freq = *self.pair_freq.get(&(t1, t2)).unwrap();
                
                // Check if the frequency is up-to-date
                if freq == current_freq {
                    break (freq, (t1, t2));  // Exit loop with valid pair
                }
            };

            // Get the positions of the pair (t1, t2)
            let mut positions: Vec<u32> = self.pair_pos.remove(&(t1, t2)).unwrap().iter().copied().collect();
            positions.sort();

            // Update next token in the dictionary
            let slice_start = positions[0] as usize;
            let temp = self.bv.next_one(positions[0] as usize).unwrap();
            let slice_end = self.bv.next_one(temp).unwrap_or(self.bv.len());
            let new_bytes: Vec<u8> = (&self.data[slice_start..slice_end]).to_vec();

            println!("{next_id}: \"{}\"", String::from_utf8_lossy(&new_bytes));

            self.bytes_to_token.insert(new_bytes, next_id); 

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
                let t0 = t0_pos.and_then(|t0_pos| {
                    let slice = &self.data[t0_pos..t1_pos];
                    self.bytes_to_token.get(slice).copied()
                });

                // Get the next token (if it exists)
                let t3: Option<u32> = t3_pos.and_then(|t3_pos| {
                    let t4_pos = self.bv.next_one(t3_pos).unwrap_or(self.bv.len());
                    let slice = &self.data[t3_pos..t4_pos];
                    self.bytes_to_token.get(slice).copied()
                });

                // Update (t0, t1) and (t0, next_id)                
                if let Some(t0) = t0 {
                    updated_pairs.insert((t0, t1));
                    updated_pairs.insert((t0, next_id));
                    // Update the pair (t0, t1)
                    if let Some(pos_set) = self.pair_pos.get_mut(&(t0, t1)) {
                        pos_set.remove(&(t0_pos.unwrap() as u32));
                    }
                    *self.pair_freq
                        .get_mut(&(t0, t1))
                        .unwrap() -= 1;
                    // Update the pair (t0, next_id)
                    self.insert_pair((t0, next_id), t0_pos.unwrap() as u32);
                }

                // Update (t2, t3) and (next_id, t3)
                if let Some(t3) = t3 {
                    updated_pairs.insert((t2, t3));
                    updated_pairs.insert((next_id, t3));
                    // Update the pair (t2, t3)
                    if let Some(pos_set) = self.pair_pos.get_mut(&(t2, t3)) {
                        pos_set.remove(&(t2_pos as u32));
                    }
                    *self.pair_freq
                        .get_mut(&(t2, t3))
                        .unwrap() -= 1;
                    // Update the pair (next_id, t3)
                    self.insert_pair((next_id, t3), t1_pos as u32);
                }

                // set t2_pos to 0 to merge t1 and t2
                self.bv.set(t2_pos as usize, false);
            }
    
            // Remove the pair (t1, t2)
            self.pair_freq.remove(&(t1, t2));

            // Update the max_freq heap with updated pairs
            for &(ti, tj) in updated_pairs.iter() {
                if (ti, tj) != (t1, t2) {
                    let freq = self.pair_freq.get(&(ti, tj)).copied().unwrap();
                    self.max_freq.push((freq, (ti, tj)));
                }
            }
        }
    }

    pub fn get_average_token_length(&self) -> f64 {
        let mut current_pos = 0;
        let mut next_pos_opt = self.bv.next_one(current_pos);
        
        let mut result: f64 = 0.0;
        let mut n_tokens = 0;

        while let Some(next_pos) = next_pos_opt {
            result += (next_pos - current_pos) as f64;
            current_pos = next_pos;
            next_pos_opt = self.bv.next_one(next_pos);
            n_tokens += 1;
        }

        result / n_tokens as f64
    }

    /*
    pub fn tokenize_and_initialize(&mut self) {
        let mut start: Option<usize> = None;
        let mut current_token_pos = 0;
        let mut prev_token_id: Option<u32> = None;

        for (i, &c) in self.data.iter().enumerate() {
            if c.is_ascii_alphanumeric() {
                if start.is_none() {
                    start = Some(i);
                }
            }
            else {
                if let Some(start) = start.take() {
                    // Process the current alphanumeric token
                    let token = &self.data[start..i];
                    let token_id = self.get_or_insert_token(token);
                    self.bv.set(start, true);
                    if let Some(prev_token_id) = prev_token_id {
                        self.insert_pair((prev_token_id, token_id), current_token_pos);
                    }
                    prev_token_id = Some(token_id);
                    current_token_pos = start as u32;
                }
                // Process the current non-alphanumeric character as its own token
                self.bv.set(i, true);
                // Update pair_pos and pair_freq
                let next_token_id = c as u32;
                if let Some(prev_token_id) = prev_token_id {
                    self.insert_pair((prev_token_id, next_token_id), current_token_pos);
                }
                prev_token_id = Some(next_token_id);
                current_token_pos = i as u32;
            }
        }

        // Process the last token if necessary
        if let Some(start) = start.take() {
            let token = &self.data[start..];
            let token_id = self.get_or_insert_token(token);
            self.bv.set(start, true);
            if let Some(prev_token_id) = prev_token_id {
                self.insert_pair((prev_token_id, token_id), start as u32);
            }
        }

        // Insert pair_freq into max_freq
        for (pair, freq) in self.pair_freq.iter() {
            self.max_freq.push((*freq, *pair));
        }
    }
    */
}