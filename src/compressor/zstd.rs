use crate::compressor::Compressor;
use super::{BlockCompressor, BlockMetadata, DEFAULT_BLOCK_SIZE};
use std::mem;

pub struct ZstdCompressor {
    block_size: usize,                      // Maximum size of each block (in bytes)
    compressed_data: Vec<u8>,               // Store compressed blocks
    blocks_metadata: Vec<BlockMetadata>,    // Metadata for each block
    item_end_positions: Vec<usize>,         // End positions of each item in the original data
    cache_index: Option<usize>,             // Index of the block in cache
    cache: Vec<u8>,                         // Cache for the last decompressed block
}

impl Compressor for ZstdCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        ZstdCompressor {
            block_size: DEFAULT_BLOCK_SIZE,
            compressed_data: Vec::with_capacity(data_size),
            blocks_metadata: Vec::with_capacity(data_size / DEFAULT_BLOCK_SIZE),
            item_end_positions: Vec::with_capacity(n_elements),
            cache_index: None,
            cache: vec![0; DEFAULT_BLOCK_SIZE],
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        BlockCompressor::compress(self, data, end_positions);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        BlockCompressor::decompress(self, buffer)
    }

    #[inline(always)]
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        BlockCompressor::get_item_at(self, index, buffer)
    }

    fn space_used_bytes(&self) -> usize {
        self.compressed_data.len() + self.blocks_metadata.len() * (mem::size_of::<usize>() + mem::size_of::<usize>() + mem::size_of::<i32>())
    }

    fn name(&self) -> &str {
        "Zstd"
    }
}

impl BlockCompressor for ZstdCompressor {
    #[inline(always)]
    fn compress_block(&mut self, block: &[u8]) -> usize {
        let current_size = self.compressed_data.len();
        let block_len = block.len();

        unsafe {
            // Temporarily extend the length of the vector
            self.compressed_data.set_len(current_size + block_len);

            // Create a mutable slice of `self.compressed_data` starting from the current size
            let buffer_slice = self.compressed_data.get_unchecked_mut(current_size..current_size + block_len);
            
            // Compress the block into the buffer slice
            let compressed_buffer_size = zstd::bulk::compress_to_buffer(block, buffer_slice, zstd::DEFAULT_COMPRESSION_LEVEL).expect("zstd compression failed");
            
            // Adjust the length of `self.compressed_data` to include only the actual compressed data
            self.compressed_data.set_len(current_size + compressed_buffer_size);

            compressed_buffer_size
        }
    }

    #[inline(always)]
    fn decompress_block(&self, compressed_data: &[u8], _uncompressed_size: usize, buffer: &mut [u8]) {
        let _ = zstd::bulk::decompress_to_buffer(compressed_data, buffer);
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
        let compressed_data = &self.compressed_data[block_start..block_end];

        unsafe {
            self.cache.set_len(uncompressed_size);
                
            // Decompress into the provided buffer
            let _ = zstd::bulk::decompress_to_buffer(compressed_data, &mut self.cache);
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
        &self.compressed_data
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
    
    fn get_item_end_positions_mut(&mut self) -> &mut Vec<usize> {
        &mut self.item_end_positions
    }
}