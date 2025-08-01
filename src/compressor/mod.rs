//! Compression algorithm abstractions and implementations
//!
//! This module defines the core `Compressor` trait that standardizes the interface
//! for string compression algorithms evaluated in this benchmark suite. It provides
//! a uniform interface for all compression algorithms in the benchmark framework.

pub mod raw;
pub mod bpe;
pub mod onpair;
pub mod onpair16;
pub mod onpair_bv;

/// Core trait defining the compression algorithm interface
/// 
/// This trait provides a uniform interface for all compression algorithms
/// in the benchmark framework. All derived implementations must provide:
/// - Compression/decompression of string collections
/// - Access to individual strings by index
/// - Space usage reporting for compression ratio calculation
pub trait Compressor {
    /// Creates a new compressor instance with pre-allocated buffers
    /// 
    /// Create instances of compression algorithms with appropriate buffer sizes based 
    /// on the dataset characteristics.
    /// 
    /// # Arguments
    /// - `data_size`: Total size of input data in bytes
    /// - `n_elements`: Number of individual strings in the dataset
    fn new(data_size: usize, n_elements: usize) -> Self;

    /// Compresses the input dataset using the algorithm implementation
    /// 
    /// # Arguments
    /// - `data`: Concatenated string data as byte array
    /// - `end_positions`: Boundary positions for individual strings (cumulative lengths)
    fn compress(&mut self, data: &[u8], end_positions: &[usize]);

    /// Decompresses the entire dataset to provided buffer
    /// 
    /// # Arguments
    /// - `buffer`: Output buffer for decompressed data (must be pre-allocated)
    /// 
    /// # Returns
    /// Number of bytes written to the output buffer
    fn decompress(&self, buffer: &mut [u8]) -> usize;

    /// Retrieves a single string by index
    /// 
    /// Core operation for access latency measurement. Provides direct access
    /// to individual strings without decompressing the entire dataset.
    /// Writes the requested string to the provided buffer.
    /// 
    /// # Arguments
    /// - `index`: Zero-based index of the string to retrieve
    /// - `buffer`: Output buffer for the decompressed string
    /// 
    /// # Returns
    /// Number of bytes written to the buffer
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize;

    /// Reports total memory usage of the compressed representation
    /// 
    /// # Returns
    /// Total bytes used by compressed data and metadata structures
    fn space_used_bytes(&self) -> usize;

    /// Returns the human-readable name of the compression algorithm
    /// 
    /// # Returns
    /// Identifier for the algorithm (e.g., "lz4", "zstd")
    fn name(&self) -> &str;
}

#[allow(dead_code)]
/// Default block size for block-based compression algorithms
/// Set to 64 KB as a reasonable balance between compression efficiency and memory usage.
const DEFAULT_BLOCK_SIZE: usize = 64 * 1024; 

/// Metadata structure for individual compressed blocks
/// 
/// Stores essential information needed for block boundary management
/// and random access within compressed datasets divided into fixed-size blocks.
pub struct BlockMetadata {
    pub end_position: usize,    // End position of this block in compressed data
    pub num_items_psum: usize,  // Cumulative number of items up to this block
    pub uncompressed_size: i32, // Uncompressed size of this block
}

/// Extended trait for block-based compression algorithms
/// 
/// Provides infrastructure for compressors that divide input data into fixed-size blocks
/// for independent compression. Enables efficient random access by maintaining block
/// metadata and implementing block-level caching for repeated accesses.
pub trait BlockCompressor: Compressor {
    /// Returns the block size (in bytes) used by this compressor
    /// 
    /// # Returns
    /// The size in bytes of each data block used for compression
    fn get_block_size(&self) -> usize;

    /// Provides access to the entire compressed data
    /// 
    /// # Returns
    /// Byte slice containing all compressed blocks concatenated together
    fn get_compressed_data(&self) -> &[u8];

    /// Returns the metadata for all compressed blocks
    ///
    /// # Returns
    /// Vector containing metadata for each compressed block
    fn get_blocks_metadata(&self) -> &Vec<BlockMetadata>;

    /// Returns mutable access to block metadata
    /// 
    /// # Returns
    /// Mutable vector containing metadata for each compressed block
    fn get_blocks_metadata_mut(&mut self) -> &mut Vec<BlockMetadata>;

    /// Provides access to item end positions
    /// 
    /// # Returns
    /// Slice containing cumulative end positions for each item
    fn get_item_end_positions(&self) -> &[usize];

    /// Returns mutable access to item end positions
    /// 
    /// # Returns
    /// Mutable vector containing cumulative end positions for each item
    fn get_item_end_positions_mut(&mut self) -> &mut Vec<usize>;

    /// Compresses a single block using the algorithm-specific method
    /// 
    /// Compresses the provided block of data and appends the result
    /// to the internal compressed data storage.
    /// 
    /// # Arguments
    /// - `block`: The uncompressed data block to compress
    /// 
    /// # Returns
    /// The number of bytes in the compressed block
    fn compress_block(&mut self, block: &[u8]) -> usize;

    /// Decompresses a single block into the provided buffer
    ///
    /// # Arguments
    /// - `compressed_data`: The compressed block data
    /// - `uncompressed_size`: Size of the decompressed data
    /// - `buffer`: Output buffer for the decompressed data
    fn decompress_block(&self, compressed_data: &[u8], uncompressed_size: usize, buffer: &mut [u8]);

    /// Decompresses a block to the internal cache for efficient repeated access
    /// 
    /// Decompresses the specified block and stores it in an internal cache
    /// for efficient repeated access to items within the block. Implements
    /// block-level caching to amortize decompression costs during sequential
    /// or clustered random access patterns.
    /// 
    /// # Arguments
    /// - `block_index`: Index of the block to decompress and cache
    fn decompress_block_to_cache(&mut self, block_index: usize);

    /// Provides access to the cached decompressed block data
    /// 
    /// Returns the cached decompressed data from the most recently accessed
    /// block. Used for efficient item extraction after block decompression.
    /// 
    /// # Returns
    /// Byte slice containing the cached decompressed block data
    fn get_block_cache(&self) -> &[u8];

    /// Returns the total number of compressed blocks
    /// 
    /// # Returns
    /// Total number of blocks in the compressed representation
    #[inline(always)]
    fn get_num_blocks(&self) -> usize {
        self.get_blocks_metadata().len()
    }
    
    /// Default implementation of compression for block-based algorithms
    /// 
    /// Divides the input data into blocks and compresses each block independently.
    /// Automatically handles block boundaries and maintains metadata for efficient
    /// random access.
    /// 
    /// # Arguments
    /// - `data`: Raw byte array containing concatenated strings
    /// - `end_positions`: Boundary positions for individual strings (cumulative lengths)
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

    /// Decompresses all blocks to reconstruct the original dataset
    /// 
    /// Decompresses all blocks sequentially and concatenates the results into
    /// the provided buffer.
    /// 
    /// # Arguments
    /// - `buffer`: Pre-allocated output buffer for decompressed data
    /// 
    /// # Returns
    /// Total number of bytes written to the output buffer
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

    /// Retrieves a single string by index with optimized random access
    /// 
    /// Locates the block containing the requested string, decompresses only
    /// that block (with caching), and extracts the specific string.
    /// Implements block-level caching to amortize decompression costs.
    ///
    /// # Arguments
    /// - `index`: Zero-based index of the string to retrieve
    /// - `buffer`: Output buffer for the decompressed string
    /// 
    /// # Returns
    /// Number of bytes written to the buffer
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

    /// Finds the block index containing the specified string
    /// 
    /// Uses binary search on cumulative item counts to efficiently locate
    /// the target block for random access operations.
    /// 
    /// # Arguments
    /// * `item_index` - Zero-based index of the target string
    /// 
    /// # Returns
    /// Index of the block containing the string
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

    /// Calculates start and end positions of a string within its block
    /// 
    /// Translates global string positions to block-relative coordinates for
    /// efficient extraction from decompressed block data.
    ///
    /// # Arguments
    /// * `block_index` - Index of the block containing the string
    /// * `item_index` - Global index of the target string
    /// 
    /// # Returns
    /// Tuple of (start_offset, end_offset) within the block's decompressed data
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
