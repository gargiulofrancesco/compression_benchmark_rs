use crate::bit_vector::BitVector;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct Repair<'a> {
    data: &'a [u8],
    next_id: u32,
    threshold: u32,
}

impl<'a> Repair<'a> {
    pub fn new(data: &'a [u8]) -> Repair {
        Repair {
            data,
            next_id: 256,
            threshold: ((data.len() as f32).sqrt() / 3.0) as u32,
        }
    }

    pub fn compress (&mut self, n_iterations: usize, tokenize_words: bool) {
        let (mut token_ids, mut bv, mut pair_pos) = self.tokenize(tokenize_words);

        Self::cluster_positions(&mut pair_pos, &token_ids, &bv); 

        let mut hf_pairs = self.initialize_hf_queue(&pair_pos, &token_ids, &bv);

        self.merge(n_iterations, &mut token_ids, &mut bv, &mut pair_pos, &mut hf_pairs);
    }

    fn tokenize(&mut self, tokenize_words: bool) -> (Vec<u32>, BitVector, Vec<u32>) {
        if tokenize_words {
            self.tokenize_words()
        }
        else {
            self.tokenize_bytes()
        }
    }

    fn tokenize_bytes(&self) -> (Vec<u32>, BitVector, Vec<u32>) {
        let mut bv = BitVector::with_ones(self.data.len());
        let mut pair_pos: Vec<u32> = (0..(self.data.len()-1) as u32).collect();
        let mut token_ids: Vec<u32> = self.data.iter().map(|&byte| byte as u32).collect();

        token_ids.shrink_to_fit();
        bv.shrink_to_fit();
        pair_pos.shrink_to_fit();
        
        (token_ids, bv, pair_pos)
    }

    fn tokenize_words(&mut self) -> (Vec<u32>, BitVector, Vec<u32>) {
        // Process bv and token ids
        let mut token_ids: Vec<u32> = vec![u32::MAX; self.data.len()];
        let mut bv = BitVector::with_zeroes(self.data.len());
        let mut pair_pos: Vec<u32> = Vec::with_capacity(self.data.len());

        let mut map: FxHashMap<Vec<u8>, u32> = FxHashMap::default();
        let mut prev_not_alphanumeric = if self.data[0].is_ascii_alphanumeric() { false } else { true };
        let mut prev_token_pos = 0;
        
        bv.set(0, true);
        pair_pos.push(0);

        for i in 1..bv.len() {
            if !self.data[i].is_ascii_alphanumeric() {
                bv.set(i, true);
                pair_pos.push(i as u32);
                prev_not_alphanumeric = true;

                let slice = &self.data[prev_token_pos..i];
                let token_id = self.get_or_insert_token(&mut map, slice);
                token_ids[prev_token_pos] = token_id;
        
                prev_token_pos = i;
            }
            else {
                if prev_not_alphanumeric {
                    bv.set(i, true);
                    pair_pos.push(i as u32);

                    let slice = &self.data[prev_token_pos..i];
                    let token_id = self.get_or_insert_token(&mut map, slice);
                    token_ids[prev_token_pos] = token_id;
            
                    prev_token_pos = i;
                }

                prev_not_alphanumeric = false;
            }
        }

        // Remove the last pair position
        pair_pos.pop(); 

        // Add the last token
        let slice = &self.data[prev_token_pos..]; 
        let token_id = self.get_or_insert_token(&mut map, slice);
        token_ids[prev_token_pos] = token_id;

        // Return the token ids, bit vector, and pair positions
        token_ids.shrink_to_fit();
        bv.shrink_to_fit();
        pair_pos.shrink_to_fit();

        (token_ids, bv, pair_pos)
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
    
    fn cluster_positions(pair_pos: &mut [u32], token_ids: &[u32], bv: &BitVector) {
        let mut C1: FxHashMap<u64, usize> = FxHashMap::default();
        let mut C2: FxHashMap<u64, usize> = FxHashMap::default();
    
        for &pos in pair_pos.iter() {
            let ab = Self::get_pair_hash(pos, token_ids, bv);
            *C1.entry(ab).or_insert(0) += 1;
        }
    
        let mut j = 0;
        for &pos in pair_pos.iter() {
            let ab = Self::get_pair_hash(pos, token_ids, bv);
            if !C2.contains_key(&ab) {
                let c1_ab = *C1.get(&ab).unwrap(); // Lookup once and reuse
                j += c1_ab;
                let temp = j - c1_ab;
                C1.insert(ab, temp);
                C2.insert(ab, temp);
            }
        }
    
        j = 0;
        while j < pair_pos.len() {
            let pos = pair_pos[j];
            let ab = Self::get_pair_hash(pos, token_ids, bv);
            let c1_ab = *C1.get(&ab).unwrap(); // Cache C1 and C2 values to reduce HashMap lookups
            let c2_ab = *C2.get(&ab).unwrap();
    
            if c1_ab <= j && j < c2_ab {
                j += 1;
            } else {
                pair_pos.swap(j, c2_ab as usize);
                if let Some(c2_value) = C2.get_mut(&ab) {
                    *c2_value += 1;
                }
            }
        }  
    }

    #[inline(always)]
    fn get_pair_hash(pos: u32, token_ids: &[u32], bv: &BitVector) -> u64 {
        let t1 = token_ids[pos as usize] as u64;
        let t2 = token_ids[bv.next_one(pos as usize).unwrap()] as u64;
        (t1 << 32) | t2
    }

    fn initialize_hf_queue(&self, pair_pos: &[u32], token_ids: &[u32], bv: &BitVector) -> FxHashMap<u64, (u32, u32, u32)> {
        let mut hf_pairs: FxHashMap<u64, (u32, u32, u32)> = FxHashMap::default();
        
        let mut start = 0;
        let mut curr_freq = 0;
        let mut curr_pair = Self::get_pair_hash(pair_pos[0], token_ids, bv);

        for (i, &pos) in pair_pos.iter().enumerate() {
            let pair = Self::get_pair_hash(pos, token_ids, bv);
            if pair == curr_pair {
                curr_freq += 1;
            }
            else {
                if curr_freq > self.threshold {
                    hf_pairs.insert(curr_pair, (curr_freq, start, curr_freq));
                }
                curr_pair = pair;
                curr_freq = 1; 
                start = i as u32;
            }
        }

        // Process the last pair
        if curr_freq > self.threshold {
            hf_pairs.insert(curr_pair, (curr_freq, start, curr_freq));
        }

        hf_pairs
    }

    fn merge(&mut self, n_iterations: usize, token_ids: &mut [u32], bv: &mut BitVector, pair_pos: &mut [u32], hf_pairs: &mut FxHashMap<u64, (u32, u32, u32)>) {
        for _ in 0..n_iterations {     
            let (&pair, &(freq, start, length)) = hf_pairs.iter().max_by_key(|(_, &(freq, _, _))| freq).unwrap();
            let end = (start + length) as usize;
            let t1 = (pair >> 32) as u32;
            let t2 = pair as u32;
            
            for &pos in pair_pos[start as usize..end].iter() {
                let t1_pos = pos as usize;
                let t2_pos = bv.next_one(t1_pos).unwrap();
                if token_ids[t1_pos] != t1 || token_ids[t2_pos] != t2 {
                    continue;
                }

                // set t2_pos to 0 to merge t1 and t2
                bv.set(t2_pos, false);

                // Update the token_ids
                token_ids[t1_pos] = self.next_id;
                token_ids[t2_pos] = u32::MAX;

                // Update context
                let t0_pos = bv.prev_one(t1_pos);
                let t3_pos = bv.next_one(t2_pos);
                let t0 = t0_pos.map(|t0_pos| token_ids[t0_pos]);
                let t3 = t3_pos.map(|t3_pos| token_ids[t3_pos]);

                // Update (t0, t1)
                if let Some(t0) = t0 {
                    let t0_t1 = (t0 as u64) << 32 | t1 as u64;
                    if let Some(pair) = hf_pairs.get_mut(&t0_t1) {
                        pair.0 -= 1; // Decrement the frequency if the pair is present
                    }
                }
                
                // Update (t2, t3)
                if let Some(t3) = t3 {
                    let t2_t3 = (t2 as u64) << 32 | t3 as u64;
                    if let Some(pair) = hf_pairs.get_mut(&t2_t3) {
                        pair.0 -= 1; // Decrement the frequency if the pair is present
                    }
                }
            }

            // Update left context
            let mut processed_pairs: FxHashSet<u64> = FxHashSet::default();
            for i in start as usize..end {
                let pos = pair_pos[i];
                if token_ids[pos as usize] != self.next_id {
                    continue;
                }

                let t0_pos = bv.prev_one(pos as usize);
                if let Some(t0) = t0_pos.map(|t0_pos| token_ids[t0_pos]) {
                    let t0_t1 = (t0 as u64) << 32 | t1 as u64;
                    if processed_pairs.contains(&t0_t1) {
                        continue;
                    }
                    processed_pairs.insert(t0_t1);

                    if let Some(pair) = hf_pairs.get_mut(&t0_t1) {
                        if pair.0 <= pair.2 / 2 {
                            // Synchronize left context
                            self.synchronize(t0, t1, token_ids, bv, pair_pos, hf_pairs);
                        }
                    }
                }
            }

            self.synchronize(t1, t2, token_ids, bv, pair_pos, hf_pairs);
            hf_pairs.remove(&pair);
            self.next_id += 1; 
        }
    }

    fn synchronize(&self, t1: u32, t2: u32, token_ids: &mut [u32], bv: &mut BitVector, pair_pos: &mut [u32], hf_pairs: &mut FxHashMap<u64, (u32, u32, u32)>) {
        let t1_t2 = (t1 as u64) << 32 | t2 as u64;
        if let Some(pair) = hf_pairs.get_mut(&t1_t2) {
            let start = pair.1 as usize;
            let end = (pair.1 + pair.2) as usize;
            
            Self::cluster_positions(&mut pair_pos[start..end], token_ids, bv);
            
            let mut subrange_start: Option<usize> = None;
            let mut curr_freq = 0;
            let mut curr_pair: Option<u64> = None;

            for i in start..end {
                let pos = pair_pos[i];
                if !bv.get(pos as usize).unwrap() {
                    // Process previous pair
                    if let Some(pair) = curr_pair {
                        if curr_freq > self.threshold {
                            hf_pairs.insert(pair, (curr_freq, subrange_start.unwrap() as u32, curr_freq as u32));
                        }
                    }

                    curr_pair = None;
                    curr_freq = 0;
                    subrange_start = None;
                    
                    continue;
                }

                let next_pair = Self::get_pair_hash(pos, token_ids, bv);

                // If we are seeing the same pair again, increment frequency
                if Some(next_pair) == curr_pair {
                    curr_freq += 1;
                }
                else {
                    // Process previous pair
                    if let Some(pair) = curr_pair {
                        if curr_freq > self.threshold {
                            hf_pairs.insert(pair, (curr_freq, subrange_start.unwrap() as u32, curr_freq as u32));
                        }
                    }

                    curr_pair = Some(next_pair);
                    curr_freq = 1;
                    subrange_start = Some(i);
                }
            }

            // Process the last pair
            if let Some(pair) = curr_pair {
                if curr_freq > self.threshold {
                    hf_pairs.insert(pair, (curr_freq, subrange_start.unwrap() as u32, curr_freq as u32));
                }
            }
        }
    }

}