use crate::compressor::Compressor;
use super::{BlockCompressor, BlockMetadata, DEFAULT_BLOCK_SIZE};
use std::mem;
use snap::raw::{Decoder, Encoder};

pub struct SnappyCompressor {
    block_size: usize,                      // Maximum size of each block (in bytes)
    data: Vec<u8>,                          // Store compressed blocks
    blocks_metadata: Vec<BlockMetadata>,    // Metadata for each block
    item_end_positions: Vec<usize>,         // End positions of each item in the original data
    cache_index: Option<usize>,             // Index of the block in cache
    cache: Vec<u8>,                         // Cache for the last decompressed block
}

impl Compressor for SnappyCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        SnappyCompressor {
            block_size: DEFAULT_BLOCK_SIZE,
            data: Vec::with_capacity(data_size + 2048),
            blocks_metadata: Vec::with_capacity(data_size / DEFAULT_BLOCK_SIZE),
            item_end_positions: Vec::with_capacity(n_elements),
            cache_index: None,
            cache: Vec::with_capacity(DEFAULT_BLOCK_SIZE),
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        self.item_end_positions.extend_from_slice(end_positions);
        BlockCompressor::compress(self, data, end_positions);
    }
    
    fn decompress(&self, buffer: &mut Vec<u8>) {
        BlockCompressor::decompress(self, buffer);
    }
    
    #[inline(always)]
    fn get_item_at(&mut self, index: usize, buffer: &mut Vec<u8>) {
        BlockCompressor::get_item_at(self, index, buffer);
    }

    fn space_used_bytes(&self) -> usize {
        self.data.len() + self.blocks_metadata.len() * (mem::size_of::<usize>() + mem::size_of::<usize>() + mem::size_of::<i32>())
    }

    fn name(&self) -> &str {
        "Snappy"
    }
}

impl BlockCompressor for SnappyCompressor {
    fn set_block_size(&mut self, block_size: usize) {
        // Only allow setting the block size before compressing data
        debug_assert!(self.data.is_empty() && self.blocks_metadata.is_empty() && self.item_end_positions.is_empty());

        self.block_size = block_size;
        self.blocks_metadata = Vec::with_capacity(self.data.capacity() / block_size);
    }

    #[inline(always)]
    fn compress_block(&mut self, block: &[u8]) -> usize {
        let current_size = self.data.len();
        let block_len = block.len();
        let max_compressed_len = snap::raw::max_compress_len(block_len);

        unsafe {
            // Temporarily extend the length of the vector
            // This ensures we can safely create a slice for compression
            self.data.set_len(current_size + max_compressed_len);

            // Create a mutable slice of `self.data` starting from the current size
            let buffer_slice = self.data.get_unchecked_mut(current_size..current_size + max_compressed_len);
            let mut encoder = Encoder::new();
            let compressed_buffer_size = encoder.compress(block, buffer_slice).expect("Snappy compression failed");
            self.data.set_len(current_size + compressed_buffer_size);

            compressed_buffer_size
        }
    }
    
    #[inline(always)]
    fn decompress_block(&self, compressed_data: &[u8], uncompressed_size: usize, buffer: &mut Vec<u8>) {
        unsafe {
            let current_buffer_size = buffer.len();
            let new_buffer_size = current_buffer_size + uncompressed_size;
            buffer.set_len(new_buffer_size);
    
            // Create a mutable slice of `self.data` starting from the current size
            let buffer_slice = buffer.get_unchecked_mut(current_buffer_size..new_buffer_size);
    
            let mut decoder = Decoder::new();
            decoder.decompress(compressed_data, buffer_slice).expect("Snappy decompression failed");
        }
    }

    #[inline(always)]
    fn decompress_block_to_cache(&mut self, block_index: usize) {
        if Some(block_index) == self.cache_index {
            return;
        }

        let block_metadata = &self.blocks_metadata[block_index];
        let block_start = if block_index == 0 {
            0
        } else {
            self.blocks_metadata[block_index - 1].end_position
        };
        let block_end = block_metadata.end_position;

        self.cache.clear();
        let uncompressed_size = block_metadata.uncompressed_size as usize;
        let compressed_data = &self.data[block_start..block_end];

        unsafe {
            self.cache.set_len(uncompressed_size);
                
            // Create a mutable slice of `self.data` starting from the current size
            let cache_slice = self.cache.get_unchecked_mut(0..uncompressed_size);

            let mut decoder = Decoder::new();
            decoder.decompress(compressed_data, cache_slice).expect("Snappy decompression failed");
        }

        self.cache_index = Some(block_index);
    }
    
    #[inline(always)]
    fn get_block_cache(&self) -> &[u8] {
        &self.cache
    }

    #[inline(always)]
    fn get_block_size(&self) -> usize {
        self.block_size
    }

    #[inline(always)]
    fn get_compressed_data(&self) -> &[u8] {
        &self.data
    }

    #[inline(always)]
    fn get_blocks_metadata(&self) -> &Vec<BlockMetadata> {
        &self.blocks_metadata
    }

    #[inline(always)]
    fn get_blocks_metadata_mut(&mut self) -> &mut Vec<BlockMetadata> {
        &mut self.blocks_metadata
    }

    #[inline(always)]
    fn get_item_end_positions(&self) -> &[usize] {
        &self.item_end_positions
    }
}
