use std::mem;
use lz4::block;
use crate::compressor::Compressor;
use super::BlockCompressor;

const DEFAULT_BLOCK_SIZE: usize = 64 * 1024;  // 64 KB

pub struct LZ4Compressor {
    block_size: usize,                      // Maximum size of each block (in bytes)
    data: Vec<u8>,                          // Store compressed blocks
    blocks_metadata: Vec<BlockMetadata>,    // Metadata for each block
    item_end_positions: Vec<usize>,         // End positions of each item in the original data
}

pub struct BlockMetadata {
    pub end_position: usize,        // End position of this block in compressed data
    pub num_items_psum: usize,      // Cumulative number of items up to this block
    pub uncompressed_size: i32,     // Uncompressed size of this block
}

impl Compressor for LZ4Compressor {
    /// Create a new LZ4Compressor preallocating the amount of memory needed.
    fn new(data_size: usize, n_elements: usize) -> Self {
        
        LZ4Compressor {
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
        "LZ4"
    }
    
}

impl BlockCompressor for LZ4Compressor {
    fn set_block_size(&mut self, block_size: usize) {
        // Only allow setting the block size before compressing data
        debug_assert!(self.data.is_empty() && self.blocks_metadata.is_empty() && self.item_end_positions.is_empty()); 

        self.block_size = block_size;
        self.blocks_metadata = Vec::with_capacity(self.data.capacity() / block_size);
    }

    #[inline(always)]
    fn compress_block(&mut self, block: &[u8], num_items: usize) {
        // Get current size of compressed data (append to this)
        let current_size = self.data.len();
        let block_len = block.len();

        unsafe {
            // Temporarily extend the length of the vector
            // This ensures we can safely create a slice for compression
            self.data.set_len(current_size + block_len);
    
            // Create a mutable slice of `self.data` starting from the current size
            let buffer_slice = self.data.get_unchecked_mut(current_size..current_size + block_len);

            // Compress the block into the buffer slice
            let compressed_buffer_size = block::compress_to_buffer(block, None, false, buffer_slice).unwrap();
            
            // Adjust the length of `self.data` to include only the actual compressed data
            self.data.set_len(current_size + compressed_buffer_size);
        }

        // Update block metadata
        let end_position = self.data.len();
        let num_items_psum = num_items + self.blocks_metadata.last()
            .map_or(0, |meta| meta.num_items_psum);  // Cumulative number of items
        let uncompressed_size = block_len as i32;    // Uncompressed size of the block
    
        // Push metadata for this block
        self.blocks_metadata.push(BlockMetadata {
            end_position,
            num_items_psum,
            uncompressed_size,
        });
    } 
        
    #[inline(always)]
    fn decompress_block(&self, block_index: usize, buffer: &mut Vec<u8>) {
        unsafe {
            // Get the start and end positions of the block
            let start = if block_index == 0 {
                0
            } else {
                self.blocks_metadata.get_unchecked(block_index - 1).end_position
            };
            let end = self.blocks_metadata.get_unchecked(block_index).end_position;

            // Extract the compressed block from self.data
            let compressed_block = &self.data.get_unchecked(start..end);

            // Get the uncompressed size of the block
            let uncompressed_block_size = self.blocks_metadata.get_unchecked(block_index).uncompressed_size as usize;
            
            let current_buffer_size = buffer.len();
            let new_buffer_size = current_buffer_size + uncompressed_block_size;

            buffer.set_len(new_buffer_size);
            
            // Create a mutable slice of `self.data` starting from the current size
            let buffer_slice = buffer.get_unchecked_mut(current_buffer_size..new_buffer_size);

            // Decompress into the provided buffer
            let _ = block::decompress_to_buffer(compressed_block, Some(uncompressed_block_size as i32), buffer_slice);
        }
    }

    #[inline(always)]
    fn get_block_size(&self) -> usize {
        self.block_size
    }

    #[inline(always)]
    fn get_num_blocks(&self) -> usize {
        self.blocks_metadata.len()
    }

    #[inline(always)]
    fn get_block_index(&self, item_index: usize) -> usize {
        debug_assert!(item_index < self.item_end_positions.len());
        
        self.blocks_metadata
            .binary_search_by(|block| {
                if item_index < block.num_items_psum {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            })
            .unwrap_or_else(|idx| idx)
    }

    #[inline(always)]
    fn get_block_buffer(&self) -> Vec<u8> {
        let buffer = Vec::with_capacity(self.block_size);
        buffer
    }

    #[inline(always)]
    fn get_item_delimiters(&self, block_index: usize, item_index: usize) -> (usize, usize) {
        debug_assert!(block_index < self.get_num_blocks());
        debug_assert!(item_index < self.blocks_metadata[block_index].num_items_psum);

        unsafe{
            // Get the index of the first item in the block
            let first_item_index = if block_index == 0 {
                0
            } else {
                self.blocks_metadata.get_unchecked(block_index - 1).num_items_psum
            };

            // Start and end positions of the item
            let start = if item_index > 0 {
                *self.item_end_positions.get_unchecked(item_index - 1)
            } else {
                0
            };
            let end = *self.item_end_positions.get_unchecked(item_index);

            // Adjust for the block, if needed (only for non-zero blocks)
            let adjustment = if first_item_index > 0 {
                *self.item_end_positions.get_unchecked(first_item_index - 1)
            } else {
                0
            };
            
            (start - adjustment, end - adjustment)            
        }
    }
}
