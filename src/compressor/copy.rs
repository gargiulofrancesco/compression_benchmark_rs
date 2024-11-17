use crate::compressor::Compressor;

pub struct CopyCompressor {
    data: Vec<u8>,                  // Store the "compressed" data as bytes
    end_positions: Vec<usize>,      // Store the end positions of each element
}

impl Compressor for CopyCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        Self {
            data: Vec::with_capacity(data_size),
            end_positions: Vec::with_capacity(n_elements + 1),
        }
    }

    /// Compresses the provided data by simply copying it to internal storage.
    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        self.end_positions.push(0);
        self.end_positions.extend_from_slice(end_positions);
        self.data.extend_from_slice(data);
    }

    /// Decompresses the stored data by copying it into the provided buffer.
    fn decompress(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.data);
    }

    /// Retrieves an item starting at the specified index.
    #[inline(always)]
    fn get_item_at(&self, index: usize, buffer: &mut Vec<u8>) {
        unsafe {
            let start = *self.end_positions.get_unchecked(index);
            let end = *self.end_positions.get_unchecked(index + 1);
        
            let slice = self.data.get_unchecked(start..end);
            buffer.extend_from_slice(slice);
        }
    }

    /// Returns the amount of space used by the compressed data.
    fn space_used_bytes(&self) -> usize {
        self.data.len()
    }
    
    /// Returns the name of this compressor, which is "Copy" in this case.
    fn name(&self) -> &str {
        "Copy"
    }
}
