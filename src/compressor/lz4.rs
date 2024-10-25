use lz4::block;
use crate::compressor::Compressor;
use super::BlockCompressor;

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
        let default_block_size = 64 * 1024;  // 64 KB
        
        LZ4Compressor {
            block_size: default_block_size,
            data: Vec::with_capacity(data_size),
            blocks_metadata: Vec::with_capacity(data_size / default_block_size),
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
        self.data.len() + self.blocks_metadata.len() * (8 + 8 + 4)
    }

    fn name(&self) -> &str {
        "LZ4"
    }
    
}

impl BlockCompressor for LZ4Compressor {
    fn set_block_size(&mut self, block_size: usize) {
        self.block_size = block_size;
        self.blocks_metadata = Vec::with_capacity(self.data.capacity() / block_size);
    }

    #[inline(always)]
    fn compress_block(&mut self, block: &[u8], num_items: usize) {
        // Get current size of compressed data (append to this)
        let current_size = self.data.len();

        // Temporarily extend the length of the vector
        // This ensures we can safely create a slice for compression
        unsafe {
            self.data.set_len(current_size + block.len());
        }
    
        // Create a mutable slice of `self.data` starting from the current size
        let buffer_slice = &mut self.data[current_size..current_size + block.len()];

        // Compress the block into the buffer slice
        let compressed_buffer_size = block::compress_to_buffer(block, None, false, buffer_slice).unwrap();
        
        // Adjust the length of `self.data` to include only the actual compressed data
        unsafe {
            self.data.set_len(current_size + compressed_buffer_size);
        }

        // Update block metadata
        let end_position = self.data.len();
        let num_items_psum = num_items + self.blocks_metadata.last()
            .map_or(0, |meta| meta.num_items_psum);  // Cumulative number of items
        let uncompressed_size = block.len() as i32;    // Uncompressed size of the block
    
        // Push metadata for this block
        self.blocks_metadata.push(BlockMetadata {
            end_position,
            num_items_psum,
            uncompressed_size,
        });
    } 
        
    #[inline(always)]
    fn decompress_block(&self, block_index: usize, buffer: &mut Vec<u8>) {
        // Get the start and end positions of the block
        let start = if block_index == 0 {
            0
        } else {
            self.blocks_metadata[block_index - 1].end_position
        };
        let end = self.blocks_metadata[block_index].end_position;

        // Extract the compressed block from self.data
        let compressed_block = &self.data[start..end];

        // Get the uncompressed size of the block
        let uncompressed_block_size = self.blocks_metadata[block_index].uncompressed_size as usize;

        let current_buffer_size = buffer.len();
        let new_buffer_size = current_buffer_size + uncompressed_block_size;

        unsafe {
            buffer.set_len(new_buffer_size);
        }

        // Create a mutable slice of `self.data` starting from the current size
        let buffer_slice = &mut buffer[current_buffer_size..new_buffer_size];

        // Decompress into the provided buffer
        let _ = block::decompress_to_buffer(compressed_block, Some(uncompressed_block_size as i32), buffer_slice);
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

        let mut block_index = 0;
        for (i, block) in self.blocks_metadata.iter().enumerate() {
            if item_index < block.num_items_psum {
                block_index = i;
                break;
            }
        }

        block_index
    }

    #[inline(always)]
    fn get_item_delimiters(&self, block_index: usize, item_index: usize) -> (usize, usize) {
        debug_assert!(block_index < self.get_num_blocks());
        debug_assert!(item_index < self.blocks_metadata[block_index].num_items_psum);

        let first_item_index = self.blocks_metadata.get(block_index.wrapping_sub(1))
            .map_or(0, |meta| meta.num_items_psum);

        let start = self.item_end_positions.get(item_index.wrapping_sub(1)).copied().unwrap_or(0);
        let end = self.item_end_positions[item_index];

        // Adjust for the block, if needed (only for non-zero blocks)
        let adjustment = if block_index == 0 { 0 } else { self.item_end_positions[first_item_index - 1] };
        
        (start - adjustment, end - adjustment)
    }
}
