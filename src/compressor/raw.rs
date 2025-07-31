//! Raw (uncompressed) baseline implementation
//!
//! Provides a no-compression baseline for performance comparison. Simply stores
//! data in its original form while maintaining the same interface as compressed
//! algorithms.

use crate::compressor::Compressor;

/// Baseline compressor that stores data without compression
/// 
/// Maintains original data layout while implementing the Compressor interface.
/// Used as a performance baseline to measure compression algorithm trade-offs.
pub struct RawCompressor {
    compressed_data: Vec<u8>,   // Original uncompressed data
    offsets: Vec<usize>,        // Boundary positions for random access
}

impl Compressor for RawCompressor {
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

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        // Copy boundary positions for random access
        unsafe {
            let src = end_positions.as_ptr();
            let dst = self.offsets.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, end_positions.len());
        }

        // Copy data unchanged
        unsafe {
            let src = data.as_ptr();
            let dst = self.compressed_data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, data.len());
        }
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        unsafe {
            let src = self.compressed_data.as_ptr();
            let dst = buffer.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, self.compressed_data.len());
        }

        self.compressed_data.len()
    }

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

    fn space_used_bytes(&self) -> usize {
        self.compressed_data.len()
    }
    
    fn name(&self) -> &str {
        "Raw"
    }
}
