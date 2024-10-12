use std::error::Error;
use lz4::block;
use crate::compressor::Compressor;

pub struct LZ4Compressor {
    compressed_blocks: Vec<Vec<u8>>,    // Store compressed blocks
    block_metadata: Vec<BlockMetadata>, // Track block offsets and boundaries
    block_size: i32,                    // Maximum size of each block (e.g., 64 KB)
}

/// Metadata for each block, helping to track where strings start and end within the block.
struct BlockMetadata {
    string_indices: Vec<(usize, usize)>,  // Tuples of (start_index, end_index) for each string in the block
}

impl LZ4Compressor {
    /// Create a new LZ4Compressor with a specified block size (in bytes).
    pub fn new(block_size: i32) -> Self {
        LZ4Compressor {
            compressed_blocks: Vec::new(),
            block_metadata: Vec::new(),
            block_size,
        }
    }

    /// Compress a block of data and store it
    fn compress_block(&mut self, block_data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let compressed = block::compress(block_data, None, false)?;
        Ok(compressed)
    }

    /// Decompress a block and return the decompressed data
    fn decompress_block(&self, block: &Vec<u8>, decompressed_size: i32) -> Result<Vec<u8>, Box<dyn Error>> {
        let decompressed = block::decompress(block, Some(decompressed_size))?;
        Ok(decompressed)
    }
}

impl Compressor for LZ4Compressor {
    /// Compresses the input data into blocks and stores it internally
    fn compress(&mut self, data: &[String]) -> Result<(), Box<dyn Error>> {
        let mut current_block = Vec::new();
        let mut current_metadata = BlockMetadata { string_indices: Vec::new() };

        let mut current_offset = 0;
        let mut total_data = Vec::new(); // Store the concatenated data to handle offsets correctly

        // Concatenate all strings for proper offsets
        for s in data {
            let s_bytes = s.as_bytes();
            total_data.extend_from_slice(s_bytes);

            if current_block.len() + s_bytes.len() > self.block_size as usize {
                // Compress the current block and store it
                let compressed_block = self.compress_block(&current_block)?;
                self.compressed_blocks.push(compressed_block);
                self.block_metadata.push(current_metadata);

                // Start a new block
                current_block = Vec::new();
                current_metadata = BlockMetadata { string_indices: Vec::new() };
                current_offset = 0;
            }

            // Add the string to the current block and update metadata
            current_block.extend_from_slice(s_bytes);
            current_metadata.string_indices.push((current_offset, current_offset + s_bytes.len()));
            current_offset += s_bytes.len();
        }

        // Compress the last block if it contains any data
        if !current_block.is_empty() {
            let compressed_block = self.compress_block(&current_block)?;
            self.compressed_blocks.push(compressed_block);
            self.block_metadata.push(current_metadata);
        }

        Ok(())
    }

    /// Decompresses all blocks and returns the full decompressed data as a Vec<String>
    fn decompress(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut decompressed_strings = Vec::new();

        // Iterate over each compressed block
        for (block, metadata) in self.compressed_blocks.iter().zip(self.block_metadata.iter()) {
            // Decompress the block
            let decompressed_block = self.decompress_block(block, self.block_size)?;

            // Use the metadata to extract individual strings
            let decompressed_block_str = String::from_utf8(decompressed_block)?;
            for (start, end) in &metadata.string_indices {
                decompressed_strings.push(decompressed_block_str[*start..*end].to_string());
            }
        }

        Ok(decompressed_strings)
    }

    /// Efficiently access the ith string using block-level decompression
    fn get_string_at(&self, index: usize) -> Result<String, Box<dyn Error>> {
        // Find the block that contains the ith string
        let mut total_strings = 0;
        for (block_idx, metadata) in self.block_metadata.iter().enumerate() {
            let num_strings_in_block = metadata.string_indices.len();
            if total_strings + num_strings_in_block > index {
                // This is the block that contains the ith string
                let string_index_in_block = index - total_strings;
                let (start, end) = metadata.string_indices[string_index_in_block];

                // Decompress only this block
                let decompressed_block = self.decompress_block(&self.compressed_blocks[block_idx], self.block_size)?;
                let decompressed_block_str = String::from_utf8(decompressed_block)?;

                return Ok(decompressed_block_str[start..end].to_string());
            }

            total_strings += num_strings_in_block;
        }

        Err("Index out of bounds".into())
    }

    /// Returns the total space used by the compressed data
    fn space_used_bytes(&self) -> usize {
        self.compressed_blocks.iter().map(|block| block.len()).sum()
    }
    
    fn name(&self) -> &str {
        "LZ4"
    }
}
