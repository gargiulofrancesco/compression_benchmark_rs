use super::Compressor;
use crate::bit_vector::BitVector;
use rustc_hash::FxHashMap;
use std::{arch::x86_64::*, io, time::Instant};

const THRESHOLD: usize = 10;

const MASKS: [u128; 17] =[
    0x00000000000000000000000000000000, // 0 bytes
    0x000000000000000000000000000000FF, // 1 byte
    0x0000000000000000000000000000FFFF, // 2 bytes
    0x00000000000000000000000000FFFFFF, // 3 bytes
    0x000000000000000000000000FFFFFFFF, // 4 bytes
    0x0000000000000000000000FFFFFFFFFF, // 5 bytes
    0x00000000000000000000FFFFFFFFFFFF, // 6 bytes
    0x000000000000000000FFFFFFFFFFFFFF, // 7 bytes
    0x0000000000000000FFFFFFFFFFFFFFFF, // 8 bytes
    0x00000000000000FFFFFFFFFFFFFFFFFF, // 9 bytes
    0x000000000000FFFFFFFFFFFFFFFFFFFF, // 10 bytes
    0x0000000000FFFFFFFFFFFFFFFFFFFFFF, // 11 bytes
    0x00000000FFFFFFFFFFFFFFFFFFFFFFFF, // 12 bytes
    0x000000FFFFFFFFFFFFFFFFFFFFFFFFFF, // 13 bytes
    0x0000FFFFFFFFFFFFFFFFFFFFFFFFFFFF, // 14 bytes
    0x00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF, // 15 bytes
    0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF, // 16 bytes
];

pub struct OnPairCompressor {
    data: BitVector,                            // Store the compressed data as bytes
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
    bits_per_token: usize,                      // Number of bits required to represent a token 
}

impl Compressor for OnPairCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPairCompressor {
            data: BitVector::with_capacity(data_size * 8),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::new(),
            dictionary_end_positions: Vec::new(),
            bits_per_token: 0,
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let dictionary = OnPairCompressor::train(data, end_positions);
        self.bits_per_token = (f64::log2(dictionary.len() as f64)).ceil() as usize;
        self.parse(data, end_positions, &dictionary);   
    }

    fn decompress(&self, buffer: &mut Vec<u8>) {
        let mut pos = 0;
        let dict_ptr = self.dictionary.as_ptr();
        let end_positions_ptr = self.dictionary_end_positions.as_ptr();

        while pos < self.data.len() {
            let token_id = self.data.get_bits(pos, self.bits_per_token).unwrap() as usize;
            pos += self.bits_per_token;

            unsafe {
                // Access dictionary positions using raw pointers
                let dict_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dict_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dict_end - dict_start;

                // Use SIMD to copy 16 bytes (128 bits) at a time to the buffer
                let src_ptr = dict_ptr.add(dict_start) as *const __m128i;
                let dst_ptr = buffer.as_mut_ptr().add(buffer.len()) as *mut __m128i;

                // Load 16 bytes from dictionary and store into buffer
                let data = _mm_loadu_si128(src_ptr);
                _mm_storeu_si128(dst_ptr, data);

                // Update buffer length for each entry (assuming fixed 16 bytes here)
                buffer.set_len(buffer.len() + length);
            }
        }
    }

    #[inline]
    fn get_item_at(&mut self, index: usize, buffer: &mut Vec<u8>) {
        let start = self.item_end_positions[index];
        let end = self.item_end_positions[index + 1];
        let dict_ptr = self.dictionary.as_ptr();
        let end_positions_ptr = self.dictionary_end_positions.as_ptr();

        let mut pos = start;
        while pos < end {
            let token_id = self.data.get_bits(pos, self.bits_per_token).unwrap() as usize;
            pos += self.bits_per_token;

            unsafe {
                // Access dictionary positions using raw pointers
                let dict_start = *end_positions_ptr.add(token_id as usize) as usize;
                let dict_end = *end_positions_ptr.add(token_id as usize + 1) as usize;
                let length = dict_end - dict_start;

                // Use SIMD to copy 16 bytes (128 bits) at a time to the buffer
                let src_ptr = dict_ptr.add(dict_start) as *const __m128i;
                let dst_ptr = buffer.as_mut_ptr().add(buffer.len()) as *mut __m128i;

                // Load 16 bytes from dictionary and store into buffer
                let data = _mm_loadu_si128(src_ptr);
                _mm_storeu_si128(dst_ptr, data);

                // Update buffer length for each entry (assuming fixed 16 bytes here)
                buffer.set_len(buffer.len() + length);
            }
        }
    }

    fn space_used_bytes(&self) -> usize {
        (self.data.len() / 8) + self.dictionary.len() + self.dictionary_end_positions.len() * 4
    }

    fn name(&self) -> &str {
        "On-Pair"
    }
}

impl OnPairCompressor {
    fn train(data: &[u8], end_positions: &[usize]) -> FxHashMap<u128, usize> {
        let mut dictionary: FxHashMap<u128, usize> = FxHashMap::default();
        let mut frequency: FxHashMap<(usize, usize), usize> = FxHashMap::default();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = i as u128;
            dictionary.insert(token, i);
        }
    
        let mut previous_token_id: Option<usize> = None;
        let mut previous_token: u128 = 0;
        let mut previous_length: usize = 0;
    
        let mut start = 0;
        for &end in end_positions.iter() {
            previous_token_id = None;
            let mut pos = start;
    
            while pos < end {
                // Find the longest match
                let mut match_token_id = 0;
                let mut match_token: u128 = 0;
                let mut match_length = 0;
    
                let max_len = (end - pos).min(16);
    
                unsafe {
                   // Load 16 bytes from dictionary and store into buffer
                   let simd_str = _mm_loadu_si128(data.as_ptr().add(pos) as *const _);
                   _mm_storeu_si128((&mut match_token as *mut u128) as *mut _, simd_str);
                }
    
                for length in (1..=max_len).rev() {
                    match_token &= MASKS[length];
    
                    if let Some(&id) = dictionary.get(&match_token) {
                        match_token_id = id;
                        match_length = length;
                        break;
                    }
                }
    
                // Update token frequency and possibly merge tokens
                if let Some(prev_id) = previous_token_id {
                    *frequency.entry((prev_id, match_token_id)).or_insert(0) += 1;
                
                    if frequency[&(prev_id, match_token_id)] > THRESHOLD && match_length + previous_length <= 16 {
                        let merged_token = (match_token << (previous_length << 3)) | previous_token;
                        dictionary.insert(merged_token, next_token_id);
                        next_token_id += 1;
                        frequency.remove(&(prev_id, match_token_id));
                    }
                }
    
                previous_token_id = Some(match_token_id);
                previous_length = match_length;
                previous_token = match_token;
    
                pos += match_length;
            }
    
            start = end;
        }
    
        dictionary
    }
    
    fn parse(&mut self, data: &[u8], end_positions: &[usize], dictionary: &FxHashMap<u128, usize>) {
        // Initialize dictionary metadata
        self.dictionary_end_positions.push(0);
        self.item_end_positions.push(0);
    
        let mut dictionary_map: FxHashMap<usize, usize> = FxHashMap::default();
        let mut next_token_id = 0;
    
        let mut start = 0;
        for &end in end_positions.iter() {
            if start == end {
                self.item_end_positions.push(self.data.len());
                continue;
            }
    
            let mut pos = start;
            while pos < end {
                // Find the longest match
                let mut match_token_id = 0;
                let mut match_token: u128 = 0;
                let mut match_length = 0;
    
                let max_len = (end - pos).min(16);
    
                unsafe {
                   // Load 16 bytes from dictionary and store into buffer
                   let simd_str = _mm_loadu_si128(data.as_ptr().add(pos) as *const _);
                   _mm_storeu_si128((&mut match_token as *mut u128) as *mut _, simd_str);
                }
    
                for length in (1..=max_len).rev() {
                    match_token &= MASKS[length];
    
                    if let Some(&id) = dictionary.get(&match_token) {
                        match_token_id = id;
                        match_length = length;
                        break;
                    }
                }
    
                if let Some(&existing_token_id) = dictionary_map.get(&match_token_id) {
                    self.data.append_bits(existing_token_id as u64, self.bits_per_token);
                } else {
                    self.data.append_bits(next_token_id as u64, self.bits_per_token);
                    dictionary_map.insert(match_token_id, next_token_id);
    
                    self.dictionary.extend(&data[pos..pos + match_length]);
                    self.dictionary_end_positions.push(self.dictionary.len() as u32);
    
                    next_token_id += 1;
                }
    
                pos += match_length;
            }

            self.item_end_positions.push(self.data.len());
            start = end;
        }
    }

    fn train_sample(data: &[u8], end_positions: &[usize]) -> FxHashMap<u128, usize> {
        let mut dictionary: FxHashMap<u128, usize> = FxHashMap::default();
        let mut frequency: FxHashMap<(usize, usize), usize> = FxHashMap::default();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = i as u128;
            dictionary.insert(token, i);
        }
    
        let mut previous_token_id: Option<usize> = None;
        let mut previous_token: u128 = 0;
        let mut previous_length: usize = 0;

        const SAMPLE_PERCENTAGE: f64 = 0.2;
        let num_strings = end_positions.len() as f64;
        let sample_size = num_strings * SAMPLE_PERCENTAGE;
        let step_size = (num_strings / sample_size) as usize;
    
        for idx in (0..end_positions.len()).step_by(step_size) {
            let start = if idx == 0 { 0 } else { end_positions[idx - 1] };
            let end = end_positions[idx];
            
            previous_token_id = None;
            let mut pos = start;
    
            while pos < end {
                // Find the longest match
                let mut match_token_id = 0;
                let mut match_token: u128 = 0;
                let mut match_length = 0;
    
                let max_len = (end - pos).min(16);
    
                unsafe {
                   // Load 16 bytes from dictionary and store into buffer
                   let simd_str = _mm_loadu_si128(data.as_ptr().add(pos) as *const _);
                   _mm_storeu_si128((&mut match_token as *mut u128) as *mut _, simd_str);
                }
    
                for length in (1..=max_len).rev() {
                    match_token &= MASKS[length];
    
                    if let Some(&id) = dictionary.get(&match_token) {
                        match_token_id = id;
                        match_length = length;
                        break;
                    }
                }
    
                // Update token frequency and possibly merge tokens
                if let Some(prev_id) = previous_token_id {
                    *frequency.entry((prev_id, match_token_id)).or_insert(0) += 1;
                
                    if frequency[&(prev_id, match_token_id)] > THRESHOLD && match_length + previous_length <= 16 {
                        let merged_token = (match_token << (previous_length << 3)) | previous_token;
                        dictionary.insert(merged_token, next_token_id);
                        next_token_id += 1;
                        frequency.remove(&(prev_id, match_token_id));
                    }
                }
    
                previous_token_id = Some(match_token_id);
                previous_length = match_length;
                previous_token = match_token;
    
                pos += match_length;
            }    
        }
    
        dictionary
    }

    fn train_multi(data: &[u8], end_positions: &[usize]) -> FxHashMap<u128, usize> {
        let mut dictionary: FxHashMap<u128, usize> = FxHashMap::default();
        let mut frequency: FxHashMap<(usize, usize), usize> = FxHashMap::default();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = i as u128;
            dictionary.insert(token, i);
        }
    
        let mut previous_token_id: Option<usize> = None;
        let mut previous_token: u128 = 0;
        let mut previous_length: usize = 0;

        let mut pass_number = 0;

        while next_token_id < 65535 {
            pass_number += 1;
            if pass_number > 50 {
                break;
            }
            println!("pass: {}", pass_number);
            for window in end_positions.windows(2) {
                if next_token_id == 65535 {
                    break;
                }

                let start = window[0];
                let end = window[1];

                previous_token_id = None;
                let mut pos = start;

                while pos < end && next_token_id < 65535 {
                    // Find the longest match
                    let mut match_token_id = 0;
                    let mut match_token: u128 = 0;
                    let mut match_length = 0;
        
                    let max_len = (end - pos).min(16);
        
                    unsafe {
                       // Load 16 bytes from dictionary and store into buffer
                       let simd_str = _mm_loadu_si128(data.as_ptr().add(pos) as *const _);
                       _mm_storeu_si128((&mut match_token as *mut u128) as *mut _, simd_str);
                    }
        
                    for length in (1..=max_len).rev() {
                        match_token &= MASKS[length];
        
                        if let Some(&id) = dictionary.get(&match_token) {
                            match_token_id = id;
                            match_length = length;
                            break;
                        }
                    }
        
                    // Update token frequency and possibly merge tokens
                    if let Some(prev_id) = previous_token_id {
                        *frequency.entry((prev_id, match_token_id)).or_insert(0) += 1;
                    
                        if frequency[&(prev_id, match_token_id)] > THRESHOLD && match_length + previous_length <= 16 {
                            let merged_token = (match_token << (previous_length << 3)) | previous_token;
                            dictionary.insert(merged_token, next_token_id);
                            // println!("{}: \"{}\"", next_token_id, String::from_utf8_lossy(&data[pos - previous_length..pos + match_length]).into_owned());
                            next_token_id += 1;
                            frequency.remove(&(prev_id, match_token_id));
                        }
                    }
        
                    previous_token_id = Some(match_token_id);
                    previous_length = match_length;
                    previous_token = match_token;
        
                    pos += match_length;
                }    
            }
        }
    
        dictionary
    }
}
