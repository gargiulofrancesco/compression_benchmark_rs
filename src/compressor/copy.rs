use crate::compressor::Compressor;

pub struct CopyCompressor {
    compressed_data: Vec<u8>,
    offsets: Vec<usize>,
}

impl Compressor for CopyCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        let mut compressed_data = Vec::with_capacity(data_size);
        compressed_data.resize(data_size, 0);
        
        let mut offsets = Vec::with_capacity(n_elements + 1);
        offsets.resize(n_elements + 1, 0);
        
        Self {
            compressed_data,
            offsets,
        }
    }

    /// Compresses the provided data by simply copying it to internal storage.
    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        // Copy end positions
        unsafe {
            self.offsets[0] = 0;
            let src = end_positions.as_ptr();
            let dst = self.offsets.as_mut_ptr().add(1);
            std::ptr::copy_nonoverlapping(src, dst, end_positions.len());
        }

        // Copy data
        unsafe {
            let src = data.as_ptr();
            let dst = self.compressed_data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, data.len());
        }
    }

    /// Decompresses the stored data by copying it into the provided buffer.
    fn decompress(&self, buffer: &mut [u8]) -> usize {
        unsafe {
            let src = self.compressed_data.as_ptr();
            let dst = buffer.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, self.compressed_data.len());
        }

        self.compressed_data.len()
    }

    /// Retrieves an item starting at the specified index.
    #[inline(always)]
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        unsafe {
            let start = self.offsets[index];
            let end = self.offsets[index + 1];
            let item_size = end - start;
            
            let src = self.compressed_data.as_ptr().add(start);
            let dst = buffer.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, item_size);

            item_size
        }
    }

    /// Returns the amount of space used by the compressed data.
    fn space_used_bytes(&self) -> usize {
        self.compressed_data.len()
    }
    
    /// Returns the name of this compressor, which is "Copy" in this case.
    fn name(&self) -> &str {
        "Copy"
    }
}
