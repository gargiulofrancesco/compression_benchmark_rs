use std::collections::{HashMap, HashSet, BinaryHeap};
use crate::bit_vector::BitVector;

pub struct RePair<'a> {
    data: &'a [u8],
    bv: BitVector,
    pair_pos: HashMap<(u32, u32), HashSet<u32>>,    // (token1, token2) -> bv positions
    pair_freq: HashMap<(u32, u32), u32>,            // (token1, token2) -> frequency
    max_freq: BinaryHeap<(u32, (u32, u32))>,        // (frequency, (token1, token2))
    bytes_to_token: HashMap<Vec<u8>, u32>,
}

impl<'a> RePair<'a> {
    pub fn new(data: &'a [u8]) -> RePair {
        // Initialize the dictionary with the first 256 ASCII characters
        let bytes_to_token: HashMap<Vec<u8>, u32> = (0..256)
            .map(|i| (vec![i as u8], i as u32))
            .collect();

        RePair{
            data,
            bv: BitVector::with_ones(data.len()),
            pair_pos: HashMap::new(),
            pair_freq: HashMap::new(),
            max_freq: BinaryHeap::new(),
            bytes_to_token,
        }
    }

    pub fn initialize(&mut self) {
        for i in 0..self.data.len()-1 {
            let pair = (self.data[i] as u32, self.data[i + 1] as u32);

            self.pair_pos
                .entry(pair)
                .or_insert(HashSet::new())
                .insert(i as u32);

            *self.pair_freq
                .entry(pair)
                .or_insert(0) += 1;
        }

        // Insert pair_freq into max_freq
        for (pair, freq) in self.pair_freq.iter() {
            self.max_freq.push((*freq, *pair));
        }
    }

    pub fn compress(&mut self, n_iterations: usize) {
        for iter in 0..n_iterations {
            let next_id = 256 + iter as u32;

            // Store updated pairs to minimize insertions in the max_freq heap
            let mut updated_pairs: HashSet<(u32, u32)> = HashSet::new();

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
                    self.pair_pos
                        .entry((t0, next_id))
                        .or_insert(HashSet::new())
                        .insert(t0_pos.unwrap() as u32);
                    *self.pair_freq
                        .entry((t0, next_id))
                        .or_insert(0) += 1;      
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
                    self.pair_pos
                        .entry((next_id, t3))
                        .or_insert(HashSet::new())
                        .insert(t1_pos as u32);
                    *self.pair_freq
                        .entry((next_id, t3))
                        .or_insert(0) += 1;
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
}
