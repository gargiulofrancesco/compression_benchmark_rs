use super::Compressor;
use crate::bit_vector::BitVector;
use std::collections::BinaryHeap;
use std::mem;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct BPECompressor {
    data: Vec<u8>,                          // Store the compressed data as bytes
    item_end_positions: Vec<usize>,         // Store the end positions of each compressed item
    dictionary: Vec<u8>,                    // Store the dictionary
    dictionary_end_positions: Vec<u32>,     // Store the end positions of each element in the dictionary
}

impl Compressor for BPECompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        BPECompressor {
            data: Vec::with_capacity(data_size),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::new(),
            dictionary_end_positions: Vec::new(),
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let end_positions_set: FxHashSet<usize> = end_positions.iter().copied().collect();
        let mut next_id: u16 = 256;

        let mut bv = BitVector::with_ones(data.len());
        let mut token_ids = initialize_token_ids(data, &bv, &mut next_id);
        let (mut pair_pos, mut max_freq) = initialize_pair_positions(&bv, &token_ids, &end_positions_set);
        merge(&mut bv, &mut token_ids, &end_positions_set, &mut pair_pos, &mut max_freq, &mut next_id);

        let (compressed_strings, compressed_strings_separators, dictionary, dictionary_separators) = remap_by_frequency(data, &bv, &token_ids, end_positions);
        self.dictionary = dictionary;
        self.dictionary_end_positions = dictionary_separators;

        let mut start = 0;
        self.item_end_positions.push(0);
        for end in compressed_strings_separators {
            for &token_id in compressed_strings[start..end].iter() {  
                assert!(token_id <= (u16::MAX >> 1));
                if token_id < 128 {
                    self.data.push(token_id as u8);
                }
                else {
                    let top_bits = ((token_id >> 8) as u8) | 0x80;
                    let bottom_bits = (token_id & 0xFF) as u8;
                    self.data.push(top_bits);
                    self.data.push(bottom_bits);
                }
            }
            
            self.item_end_positions.push(self.data.len());
            start = end;
        }
    }

    fn decompress(&self, buffer: &mut Vec<u8>) {
        let mut pos = 0;
        
        // Assume the buffer is preallocated to the necessary size.
        unsafe {
            // Use raw pointers to avoid bounds checking on `self.data` and `self.dictionary`
            let data_ptr = self.data.as_ptr();
            let dict_ptr = self.dictionary.as_ptr();
            let end_positions_ptr = self.dictionary_end_positions.as_ptr();
            
            while pos < self.data.len() {
                let mut token_id = *data_ptr.add(pos) as u16;
                pos += 1;
    
                if token_id > 127 {
                    token_id = (token_id & 0x7F) << 8 | *data_ptr.add(pos) as u16;
                    pos += 1;
                }
    
                // Access dictionary positions using raw pointers
                let dic_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dic_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dic_end - dic_start;
    
                // Directly copy data from dictionary to buffer
                let src_ptr = dict_ptr.add(dic_start);
                let dst_ptr = buffer.as_mut_ptr().add(buffer.len());
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, length);
    
                // Update buffer length manually
                buffer.set_len(buffer.len() + length);
            }
        }
    }

    fn get_item_at(&self, index: usize, buffer: &mut Vec<u8>) {
        let mut pos = self.item_end_positions[index];
        let end = self.item_end_positions[index + 1];
    
        unsafe {
            // Set up raw pointers to data structures to avoid bounds checking
            let data_ptr = self.data.as_ptr();
            let dict_ptr = self.dictionary.as_ptr();
            let end_positions_ptr = self.dictionary_end_positions.as_ptr();
    
            while pos < end {
                // Retrieve token ID, handling potential multi-byte encoding
                let mut token_id = *data_ptr.add(pos) as u16;
                pos += 1;
    
                if token_id > 127 {
                    token_id = (token_id & 0x7F) << 8 | *data_ptr.add(pos) as u16;
                    pos += 1;
                }
    
                // Access dictionary start and end positions
                let dic_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dic_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dic_end - dic_start;
    
                // Copy dictionary data directly into the buffer
                let src_ptr = dict_ptr.add(dic_start);
                let dest_ptr = buffer.as_mut_ptr().add(buffer.len());
                std::ptr::copy_nonoverlapping(src_ptr, dest_ptr, length);
    
                // Update buffer length manually to include new data
                buffer.set_len(buffer.len() + length);
            }
        }
    }

    fn space_used_bytes(&self) -> usize {
        self.data.len() + self.dictionary.len() + (self.dictionary_end_positions.len() * mem::size_of::<u32>())
    }

    fn name(&self) -> &str {
        "BPE"
    }
}

/// Initializes token IDs for the given data based on token boundaries defined by a `BitVector`
fn initialize_token_ids(data: &[u8], bv: &BitVector, next_id: &mut u16) -> Vec<u16> {
    let mut token_ids: Vec<u16> = vec![0; data.len()];
    let mut map: FxHashMap<Vec<u8>, u16> = FxHashMap::default();

    let mut iter = bv.ones(0);
    let mut current_pos = iter.next().unwrap();
    let mut next_pos_opt = iter.next();

    while let Some(next_pos) = next_pos_opt {
        let slice = &data[current_pos..next_pos];
        let token_id = get_or_insert_token(&mut map, slice, next_id);
        token_ids[current_pos] = token_id;

        current_pos = next_pos;
        next_pos_opt = iter.next();
    }

    // Add the last token
    let slice = &data[current_pos..];
    let token_id = get_or_insert_token(&mut map, slice, next_id);
    token_ids[current_pos] = token_id;

    token_ids.shrink_to_fit();

    token_ids
}

/// Retrieves an existing token ID for a token or inserts a new one if it's not in the map.
#[inline(always)]
fn get_or_insert_token(map: &mut FxHashMap<Vec<u8>, u16>, slice: &[u8], next_id: &mut u16) -> u16 {
    if slice.len() == 1 {
        slice[0] as u16
    }
    else if let Some(&token_id) = map.get(slice) {
        token_id
    }
    else {
        map.insert(slice.to_vec(), *next_id);
        *next_id += 1;
        *next_id - 1
    }
}

/// Initializes pair positions based on the tokenization provided by a `BitVector`
pub fn initialize_pair_positions(bv: &BitVector, token_ids: &[u16], end_positions_set: &FxHashSet<usize>) -> (FxHashMap<(u16, u16), FxHashSet<u32>>, BinaryHeap<(u32, (u16, u16))>) {
    let mut pair_pos: FxHashMap<(u16, u16), FxHashSet<u32>> = FxHashMap::default();
    let mut max_freq: BinaryHeap<(u32, (u16, u16))> = BinaryHeap::new();
    
    let mut iter = bv.ones(0);
    let mut current_pos = iter.next().unwrap();
    let mut next_pos_opt = iter.next();

    while let Some(next_pos) = next_pos_opt {
        // Skip pairs (a, b) where b is the first token of a string to avoid merging strings
        if !end_positions_set.contains(&next_pos) {
            let t1 = token_ids[current_pos];
            let t2 = token_ids[next_pos];
            pair_pos
                .entry((t1, t2))
                .or_insert(FxHashSet::default())
                .insert(current_pos as u32);
        }

        current_pos = next_pos;
        next_pos_opt = iter.next();
    }

    for (pair, pos_set) in pair_pos.iter() {
        max_freq.push((pos_set.len() as u32, *pair));
    }

    (pair_pos, max_freq)
}

pub fn merge (
    bv: &mut BitVector, 
    token_ids: &mut [u16], 
    end_positions_set: &FxHashSet<usize>,
    pair_pos: &mut FxHashMap<(u16, u16), FxHashSet<u32>>, 
    max_freq: &mut BinaryHeap<(u32, (u16, u16))>,
    next_id: &mut u16,
) {
    while *next_id <= (u16::MAX >> 1) {
        // Store updated pairs to minimize insertions in the max_freq heap
        let mut updated_pairs: FxHashSet<(u16, u16)> = FxHashSet::default();

        // Get the pair with the maximum frequency
        let (freq, (t1, t2)) = loop {
            let (freq, (t1, t2)) = max_freq.pop().unwrap();
            let current_freq = pair_pos.get(&(t1, t2)).unwrap().len() as u32;
            
            // Check if the frequency is up-to-date
            if freq == current_freq {
                break (freq, (t1, t2));  // Exit loop with valid pair
            }
        };

        if freq < 10 {
            break;
        }

        // Get the positions of the pair (t1, t2)
        let mut positions= pair_pos.remove(&(t1, t2)).unwrap().into_iter().collect::<Vec<u32>>();
        positions.sort();

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
                // If t1 is a separator, don't update the pair (t0, t1)
                if !end_positions_set.contains(&t1_pos) {
                    updated_pairs.insert((t0, t1));
                    updated_pairs.insert((t0, *next_id));
                    // Update the pair (t0, t1)
                    if let Some(pos_set) = pair_pos.get_mut(&(t0, t1)) {
                        pos_set.remove(&(t0_pos.unwrap() as u32));
                    }
                    // Update the pair (t0, next_id)
                    pair_pos
                        .entry((t0, *next_id))
                        .or_insert(FxHashSet::default())
                        .insert(t0_pos.unwrap() as u32);
                }
            }

            // Update (t2, t3) and (next_id, t3)
            if let Some(t3) = t3 {
                if !end_positions_set.contains(&t3_pos.unwrap()) {
                    updated_pairs.insert((t2, t3));
                    updated_pairs.insert((*next_id, t3));
                    // Update the pair (t2, t3)
                    if let Some(pos_set) = pair_pos.get_mut(&(t2, t3)) {
                        pos_set.remove(&(t2_pos as u32));
                    }
                    // Update the pair (next_id, t3)
                    pair_pos
                        .entry((*next_id, t3))
                        .or_insert(FxHashSet::default())
                        .insert(t1_pos as u32);
                }
            }

            // set t2_pos to 0 to merge t1 and t2
            bv.set(t2_pos as usize, false);

            // Update the token_ids
            token_ids[t1_pos] = *next_id;
        }

        // Update the max_freq heap with updated pairs
        for &(ti, tj) in updated_pairs.iter() {
            if (ti, tj) != (t1, t2) {
                let freq = pair_pos.get(&(ti, tj)).unwrap().len() as u32;
                max_freq.push((freq, (ti, tj)));
            }
        }

        *next_id += 1;
    }
}

fn remap_by_frequency(data: &[u8], bv: &BitVector, token_ids: &[u16], end_positions: &[usize]) -> (Vec<u16>, Vec<usize>, Vec<u8>, Vec<u32>) {
    let mut compressed_strings = get_new_token_ids(bv, token_ids);
    let mut compressed_strings_separators = get_new_strings_separators(bv, end_positions);

    // get the frequency of each token_id
    let mut frequency_map: FxHashMap<u16, u32> = FxHashMap::default();
    for &token_id in compressed_strings.iter() {
        *frequency_map.entry(token_id).or_insert(0) += 1;
    }

    // sort token ids by frequency
    let mut freq_vec: Vec<_> = frequency_map.iter().collect();
    freq_vec.sort_by(|a, b| b.1.cmp(a.1));

    // populate the dictionary
    let dictionary_map = get_dictionary_map(data, bv, token_ids);
    let mut dictionary = Vec::new();
    let mut dictionary_separators  = vec![0u32];
    for &(token_id, _) in freq_vec.iter() {
        let bytes = dictionary_map.get(&token_id).unwrap();
        dictionary.extend_from_slice(bytes);
        dictionary_separators.push(dictionary.len() as u32);
    }

    // create a map from token_id to frequency rank
    let mut rank_map: FxHashMap<u16, u16> = FxHashMap::default();
    for (rank, (&token_id, _)) in freq_vec.iter().enumerate() {
        rank_map.insert(token_id, rank as u16);
    }

    // remap the token_ids by frequency rank
    for token_id in compressed_strings.iter_mut() {
        *token_id = *rank_map.get(token_id).unwrap();
    }

    compressed_strings.shrink_to_fit();
    compressed_strings_separators.shrink_to_fit();
    dictionary.shrink_to_fit();
    dictionary_separators.shrink_to_fit();

    (compressed_strings, compressed_strings_separators, dictionary, dictionary_separators)
}

fn get_dictionary_map(data: &[u8], bv: &BitVector, token_ids: &[u16]) -> FxHashMap<u16, Vec<u8>> {
    let mut dictionary_map: FxHashMap<u16, Vec<u8>> = FxHashMap::default();
    
    let mut iter = bv.ones(0);
    let mut current_pos = iter.next().unwrap();
    let mut next_pos_opt = iter.next();

    while let Some(next_pos) = next_pos_opt {
        let slice = &data[current_pos..next_pos];
        let token_id = token_ids[current_pos];
        dictionary_map.entry(token_id).or_insert(slice.to_vec());

        current_pos = next_pos;
        next_pos_opt = iter.next();
    }

    let slice = &data[current_pos..];
    let token_id = token_ids[current_pos];
    dictionary_map.entry(token_id).or_insert(slice.to_vec());

    dictionary_map
}

fn get_new_token_ids(bv: &BitVector, token_ids: &[u16]) -> Vec<u16> {
    let mut new_token_ids: Vec<u16> = Vec::new();

    for pos in bv.ones(0) {
        new_token_ids.push(token_ids[pos]);
    }

    new_token_ids
}

fn get_new_strings_separators(bv: &BitVector, end_positions: &[usize]) -> Vec<usize> {
    let mut new_strings_separators = Vec::new();
    let mut current_size = 0;
    let mut start = 0;

    for &end in end_positions.iter(){
        // Handle empty strings
        if start == end {
            new_strings_separators.push(current_size);
            continue;
        }

        for pos in bv.ones(start) {
            if pos == end {
                new_strings_separators.push(current_size);
                break;
            }
            current_size += 1;
        }

        start = end;
    }

    new_strings_separators.push(current_size);
    new_strings_separators.shrink_to_fit();

    new_strings_separators
}
