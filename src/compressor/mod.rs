pub mod raw;
pub mod bpe;
pub mod onpair;
pub mod onpair16;
pub mod onpair_bv;

pub trait Compressor {
    /// Creates a new compressor allocating memory for the specified data size and number of elements.
    fn new(data_size: usize, n_elements: usize) -> Self;

    /// Compresses the provided data and stores it internally.
    fn compress(&mut self, data: &[u8], end_positions: &[usize]);

    /// Decompresses the internally stored data and returns the number of decompressed bytes.
    fn decompress(&self, buffer: &mut [u8]) -> usize;

    /// Retrieves the item at the specified index and returns its size in bytes.
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize;

    /// Returns the amount of space used by the compressed data in bytes.
    fn space_used_bytes(&self) -> usize;

    /// Returns the name of the compressor.
    fn name(&self) -> &str;
}

#[allow(dead_code)]
const DEFAULT_BLOCK_SIZE: usize = 64 * 1024;  // 64 KB (a good range is from 4 KB to 128 KB)

pub struct BlockMetadata {
    pub end_position: usize,        // End position of this block in compressed data
    pub num_items_psum: usize,      // Cumulative number of items up to this block
    pub uncompressed_size: i32,     // Uncompressed size of this block
}

pub trait BlockCompressor: Compressor {
    /// Returns the block size (in bytes) used by this compressor.
    fn get_block_size(&self) -> usize;

    /// Get the slice of compressed data.
    fn get_compressed_data(&self) -> &[u8];

    /// Returns the metadata for all blocks.
    fn get_blocks_metadata(&self) -> &Vec<BlockMetadata>;

    /// Returns mutable metadata for all blocks.
    fn get_blocks_metadata_mut(&mut self) -> &mut Vec<BlockMetadata>;

    /// Get the slice of item end positions.
    fn get_item_end_positions(&self) -> &[usize];

    /// Returns mutable item end positions.
    fn get_item_end_positions_mut(&mut self) -> &mut Vec<usize>;

    /// Compresses a single block of data, returns the number of bytes of the compressed block.
    fn compress_block(&mut self, block: &[u8]) -> usize;

    /// Decompresses a single block of data into the provided buffer.
    fn decompress_block(&self, compressed_data: &[u8], uncompressed_size: usize, buffer: &mut [u8]);

    /// Decompresses a single block of data into the internal cache.
    fn decompress_block_to_cache(&mut self, block_index: usize);

    /// Get the cache for the last decompressed block.
    fn get_block_cache(&self) -> &[u8];

    /// Get the number of blocks.
    #[inline(always)]
    fn get_num_blocks(&self) -> usize {
        self.get_blocks_metadata().len()
    }
    
    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        // Copy end_positions to self.item_end_positions
        unsafe {
            let item_end_positions = self.get_item_end_positions_mut();
            item_end_positions.set_len(end_positions.len());
            let src = end_positions.as_ptr();
            let dst = item_end_positions.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, end_positions.len());
        }

        let block_size = self.get_block_size();
        let mut block_start = 0;            // Start of the current block
        let mut num_items_in_block = 0;     // Number of items in the current block
        let mut current_block_size = 0;     // Total size of the current block
        let mut item_start = 0;             // Start of the current item

        for &item_end in end_positions.iter().skip(1) {
            let item_size = item_end - item_start;
            
            if current_block_size + item_size > block_size {
                let block = &data[block_start..item_start];
                let compressed_block_size = self.compress_block(block);

                let end_position = self.get_blocks_metadata().last().map_or(0, |m| m.end_position) + compressed_block_size;
                let num_items_psum = self.get_blocks_metadata().last().map_or(0, |meta| meta.num_items_psum) + num_items_in_block;

                self.get_blocks_metadata_mut().push(BlockMetadata {
                    end_position,
                    num_items_psum,
                    uncompressed_size: block.len() as i32,
                });

                block_start = item_start;
                num_items_in_block = 0;
                current_block_size = 0;
            }

            current_block_size += item_size;
            num_items_in_block += 1;
            item_start = item_end;
        }

        if num_items_in_block > 0 {
            let block = &data[block_start..item_start];
            let compressed_block_size = self.compress_block(block);

            let end_position = self.get_blocks_metadata().last().map_or(0, |m| m.end_position) + compressed_block_size;
            let num_items_psum = self.get_blocks_metadata().last().map_or(0, |meta| meta.num_items_psum) + num_items_in_block;  // Cumulative number of items

            self.get_blocks_metadata_mut().push(BlockMetadata {
                end_position,
                num_items_psum,
                uncompressed_size: block.len() as i32,
            });
        }
    }

    /// Decompress all blocks.
    fn decompress(&self, buffer: &mut [u8]) -> usize {
        let mut total_size = 0;

        for (i, block_metadata) in self.get_blocks_metadata().iter().enumerate() {
            let start = if i == 0 { 0 } else { self.get_blocks_metadata()[i - 1].end_position };
            let end = block_metadata.end_position;

            let compressed_data = &self.get_compressed_data()[start..end];
            self.decompress_block(compressed_data, block_metadata.uncompressed_size as usize, buffer[total_size..].as_mut());
            total_size += block_metadata.uncompressed_size as usize;
        }

        total_size
    }

    #[inline(always)]
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        let block_index = self.get_block_index(index);
        self.decompress_block_to_cache(block_index);

        let (item_start, item_end) = self.get_item_delimiters(block_index, index);
        let item_size = item_end - item_start;
        let block_cache = self.get_block_cache();

        unsafe {
            let src = block_cache.as_ptr().add(item_start);
            let dst = buffer.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, item_size);
        }
        
        item_size
    }

    /// Returns the block index for a given item index.
    #[inline(always)]
    fn get_block_index(&self, item_index: usize) -> usize {        
        self.get_blocks_metadata()
            .binary_search_by(|block| {
                if item_index < block.num_items_psum {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            })
            .unwrap_or_else(|idx| idx)
    }

    /// Get the item delimiters (start and end offsets) in a block.
    #[inline(always)]
    fn get_item_delimiters(&self, block_index: usize, item_index: usize) -> (usize, usize) {
        debug_assert!(block_index < self.get_num_blocks());

        let blocks_metadata = self.get_blocks_metadata();
        let item_positions = self.get_item_end_positions();

        let first_item_index = if block_index == 0 {
            0
        } else {
            blocks_metadata[block_index - 1].num_items_psum
        };

        let start = item_positions[item_index];
        let end = item_positions[item_index+1];

        let adjustment = if first_item_index > 0 {
            item_positions[first_item_index]
        } else {
            0
        };
        
        (start - adjustment, end - adjustment)
    }
}
