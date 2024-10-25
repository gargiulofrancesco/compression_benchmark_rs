use serde::{Deserialize, Serialize};
use std::arch::x86_64::_popcnt64;

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
    pub fn append_bits(&mut self, bits: u64, len: usize) {
        assert!(len == 64 || (bits >> len) == 0);
        assert!(len <= 64);
        if len == 0 {
            return;
        }
        let pos_in_word: usize = self.position & 63;
        self.position += len;

        if pos_in_word == 0 {
            self.data.push(bits);
        } else if let Some(last) = self.data.last_mut() {
            *last |= bits << pos_in_word;
            if len > 64 - pos_in_word {
                self.data.push(bits >> (64 - pos_in_word));
            }
        }
    }

    #[inline(always)]
    pub fn get_bits(&self, index: usize, len: usize) -> Option<u64> {
        if (len > 64) | (index + len > self.position) {
            return None;
        }
        if len == 0 {
            return Some(0);
        }
        let block = index >> 6;
        let shift = index & 63;

        let mask = if len == 64 {
            std::u64::MAX
        } else {
            (1_u64 << len) - 1
        };

        if shift + len <= 64 {
            return Some(self.data[block] >> shift & mask);
        }
        Some((self.data[block] >> shift) | (self.data[block + 1] << (64 - shift) & mask))
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

    pub fn ones(&self, pos: usize) -> UnaryIterOnes {
        UnaryIterOnes::new(self, pos)
    }

    pub fn zeroes(&self, pos: usize) -> UnaryIterZeroes {
        UnaryIterZeroes::new(self, pos)
    }
}

// Iterator for enumerating bit positions
pub struct UnaryIter<'a> {
    bv: &'a BitVector,
    pos: usize,
    word_pos: usize,
    buffer: u64,
}

impl<'a> UnaryIter<'a> {
    // Creates the iterator from the given bit position
    pub fn new(bv: &'a BitVector, pos: usize) -> UnaryIter {
        let word_pos = pos >> 6;
        let buffer = if word_pos < bv.data.len() {
            bv.data[word_pos] >> (pos % 64)
        } else {
            0
        };

        UnaryIter {
            bv,
            pos,
            word_pos,
            buffer,
        }
    }

    // iterates over positions of set bits
    #[inline(always)]
    pub fn next1(&mut self) -> Option<usize> {
        while self.buffer == 0 {
            self.pos += 64 - (self.pos % 64);
            self.word_pos = self.pos >> 6;
            if self.word_pos >= self.bv.data.len() {
                return None;
            }
            self.buffer = self.bv.data[self.word_pos];
        }
        let pos_in_word: usize = self.buffer.trailing_zeros() as usize;
        self.pos += pos_in_word + 1;
        self.word_pos = self.pos >> 6;
        self.buffer = if self.word_pos < self.bv.data.len() {
            self.bv.data[self.word_pos] >> (self.pos % 64)
        } else {
            0
        };
        Some(self.pos - 1)
    }

    // iterates over positions of bits set to zero
    #[inline(always)]
    pub fn next0(&mut self) -> Option<usize> {
        let mut buffer = !self.buffer;
        loop {
            let w;
            unsafe {
                w = _popcnt64(buffer as i64) as usize - (self.pos % 64);
            }
            if w != 0 {
                break;
            }
            self.pos += 64 - (self.pos % 64);
            self.word_pos = self.pos >> 6;
            if self.word_pos >= self.bv.data.len() {
                return None;
            }
            buffer = !self.bv.data[self.word_pos];
        }
        let pos_in_word: usize = buffer.trailing_zeros() as usize;
        self.pos += pos_in_word + 1;
        self.word_pos = self.pos >> 6;
        self.buffer = if self.word_pos < self.bv.data.len() {
            self.bv.data[self.word_pos] >> (self.pos % 64)
        } else {
            0
        };

        Some(self.pos - 1).filter(|&x| x < self.bv.position)
    }
    

    #[inline(always)]
    pub fn pos(&self) -> usize {
        self.pos
    }
}

pub struct UnaryIterOnes<'a> {
    iter: UnaryIter<'a>,
}
impl<'a> UnaryIterOnes<'a> {
    pub fn new(bv: &'a BitVector, pos: usize) -> UnaryIterOnes {
        let iter = UnaryIter::new(bv, pos);
        UnaryIterOnes { iter }
    }
}
impl<'a> Iterator for UnaryIterOnes<'a> {
    type Item = usize;

    // iterates over positions of set bits
    #[inline(always)]
    fn next(&mut self) -> Option<usize> {
        self.iter.next1()
    }
}

pub struct UnaryIterZeroes<'a> {
    iter: UnaryIter<'a>,
}
impl<'a> UnaryIterZeroes<'a> {
    pub fn new(bv: &'a BitVector, pos: usize) -> UnaryIterZeroes {
        let iter = UnaryIter::new(bv, pos);
        UnaryIterZeroes { iter }
    }
}
impl<'a> Iterator for UnaryIterZeroes<'a> {
    type Item = usize;

    // iterates over positions of set bits
    #[inline(always)]
    fn next(&mut self) -> Option<usize> {
        self.iter.next0()
    }
}