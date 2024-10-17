use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct BitVector {
    data: Vec<u64>,
    position: usize,
}

impl BitVector {
    /// Creates a new empty binary vector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty binary vector with at least a capacity of ```n_bits```.
    pub fn with_capacity(n_bits: usize) -> Self {
        let capacity = (n_bits + 63) / 64;
        Self {
            data: Vec::with_capacity(capacity),
            ..Self::default()
        }
    }

    /// Creates a binary vector with ```n_bits``` set to 0.
    pub fn with_zeroes(n_bits: usize) -> Self {
        let mut bv = Self::with_capacity(n_bits);
        bv.extend_with_zeroes(n_bits);
        bv.shrink_to_fit();
        bv
    }

    #[inline]
    pub fn extend_with_zeroes(&mut self, n: usize) {
        self.position += n;
        let new_size = (self.position + 63) / 64;
        self.data.resize_with(new_size, Default::default);
    }

    /// Creates a binary vector with `n_bits` set to 1.
    pub fn with_ones(n_bits: usize) -> Self {
        let mut bv = Self::with_capacity(n_bits);
        bv.extend_with_ones(n_bits);
        bv.shrink_to_fit();
        bv
    }

    #[inline]
    pub fn extend_with_ones(&mut self, n: usize) {
        self.position += n;
        let new_size = (self.position + 63) / 64;
        self.data.resize_with(new_size, || u64::MAX);  // Fill with u64::MAX
        if self.position % 64 != 0 {
            let remaining_bits = self.position % 64;
            self.data[new_size - 1] = (1u64 << remaining_bits) - 1;  // Set only the last bits to 1
        }
    }

    #[inline]
    pub fn push(&mut self, bit: bool) {
        let pos_in_word = self.position % 64;
        if pos_in_word == 0 {
            self.data.push(0);
        }
        if bit {
            // push a 1
            if let Some(last) = self.data.last_mut() {
                *last |= (bit as u64) << pos_in_word;
            }
        }
        self.position += 1;
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.position {
            return None;
        }
        let word = index >> 6;
        let pos_in_word = index & 63;
        Some(self.data[word] >> pos_in_word & 1_u64 == 1)
    }

    /// Sets the to ```bit``` the given position ```index```.
    #[inline(always)]
    pub fn set(&mut self, index: usize, bit: bool) {
        let word = index >> 6;
        let pos_in_word = index & 63;
        self.data[word] &= !(1_u64 << pos_in_word);
        self.data[word] |= (bit as u64) << pos_in_word;
    }

    #[inline(always)]
    pub fn next_one(&self, pos: usize) -> Option<usize> {
        let mut next_pos = pos + 1;
        let mut word_pos = next_pos >> 6;
        let mut buffer = self.data[word_pos] >> (next_pos % 64);
        
        while buffer == 0 {
            next_pos += 64 - (next_pos % 64);
            word_pos = next_pos >> 6;
            if word_pos >= self.data.len() {
                return None;
            }
            buffer = self.data[word_pos];
        }
        let pos_in_word: usize = buffer.trailing_zeros() as usize;
        next_pos += pos_in_word;
        
        Some(next_pos)
    }

    #[inline(always)]
    pub fn prev_one(&self, pos: usize) -> Option<usize> {
        if pos == 0 {
            return None;
        }

        let mut prev_pos = pos - 1;
        let mut word_pos = prev_pos >> 6;
        let mut buffer = self.data[word_pos] << (63 - (prev_pos % 64));
        
        while buffer == 0 {
            if word_pos == 0 {
                return None;
            }
            word_pos -= 1;
            prev_pos = (word_pos + 1) * 64 - 1;
            buffer = self.data[word_pos];
        }
    
        let pos_in_word: usize = buffer.leading_zeros() as usize;
        prev_pos -= pos_in_word;
    
        Some(prev_pos)
    }

    /// Shrinks the underlying vector of 64bit words to fit.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Checks if the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.position == 0
    }

    /// Returns the number of bits in the bitvector.
    pub fn len(&self) -> usize {
        self.position
    }
}
