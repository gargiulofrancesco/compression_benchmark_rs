use crate::compressor::Compressor;
use super::{BlockCompressor, BlockMetadata, DEFAULT_BLOCK_SIZE};
use std::mem;

pub struct ZstdCompressor {
    block_size: usize,                      // Maximum size of each block (in bytes)
    data: Vec<u8>,                          // Store compressed blocks
    blocks_metadata: Vec<BlockMetadata>,    // Metadata for each block
    item_end_positions: Vec<usize>,         // End positions of each item in the original data
}

impl Compressor for ZstdCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        ZstdCompressor {
            block_size: DEFAULT_BLOCK_SIZE,
            data: Vec::with_capacity(data_size),
            blocks_metadata: Vec::with_capacity(data_size / DEFAULT_BLOCK_SIZE),
            item_end_positions: Vec::with_capacity(n_elements),
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
    fn get_item_at(&self, index: usize, buffer: &mut Vec<u8>) {
        BlockCompressor::get_item_at(self, index, buffer);
    }

    fn space_used_bytes(&self) -> usize {
        self.data.len() + self.blocks_metadata.len() * (mem::size_of::<usize>() + mem::size_of::<usize>() + mem::size_of::<i32>())
    }

    fn name(&self) -> &str {
        "Zstd"
    }
}

impl BlockCompressor for ZstdCompressor {
    fn set_block_size(&mut self, block_size: usize) {
        debug_assert!(
            self.data.is_empty() && self.blocks_metadata.is_empty() && self.item_end_positions.is_empty(),
            "Block size can only be set before compression starts"
        );
        self.block_size = block_size;
        self.blocks_metadata = Vec::with_capacity(self.data.capacity() / block_size);
    }

    #[inline(always)]
    fn compress_block(&mut self, block: &[u8]) -> usize {
        let current_size = self.data.len();
        let block_len = block.len();

        unsafe {
            // Temporarily extend the length of the vector
            self.data.set_len(current_size + block_len);

            // Create a mutable slice of `self.data` starting from the current size
            let buffer_slice = self.data.get_unchecked_mut(current_size..current_size + block_len);
            
            // Compress the block into the buffer slice
            let compressed_buffer_size = zstd::bulk::compress_to_buffer(block, buffer_slice, zstd::DEFAULT_COMPRESSION_LEVEL).expect("zstd compression failed");
            
            // Adjust the length of `self.data` to include only the actual compressed data
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
                
            // Create a mutable slice of `buffer` starting from the current size
            let buffer_slice = buffer.get_unchecked_mut(current_buffer_size..new_buffer_size);
    
            // Decompress into the provided buffer
            let _ = zstd::bulk::decompress_to_buffer(compressed_data, buffer_slice);
        }
    }

    fn get_block_size(&self) -> usize {
        self.block_size
    }

    fn get_compressed_data(&self) -> &[u8] {
        &self.data
    }

    fn get_blocks_metadata(&self) -> &Vec<BlockMetadata> {
        &self.blocks_metadata
    }

    fn get_blocks_metadata_mut(&mut self) -> &mut Vec<BlockMetadata> {
        &mut self.blocks_metadata
    }

    fn get_item_end_positions(&self) -> &[usize] {
        &self.item_end_positions
    }
}
