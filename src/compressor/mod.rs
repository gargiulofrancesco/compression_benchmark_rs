pub mod lz4;
pub mod copy;
pub mod bpe4;
pub mod fsst;

pub trait Compressor {
    /// Creates a new compressor allocating memory for the specified data size and number of elements.
    fn new(data_size: usize, n_elements: usize) -> Self;

    /// Compresses the provided data and stores it internally.
    /// The end_positions slice contains the end positions of each item in the data.
    fn compress(&mut self, data: &[u8], end_positions: &[usize]);

    /// Decompresses the internally stored data and returns it.
    fn decompress(&self, buffer: &mut Vec<u8>);

    /// Retrieves an item at the specified index with minimal decompression.
    fn get_item_at(&self, index: usize, buffer: &mut Vec<u8>);

    /// Returns the amount of space used by the compressed data in bytes.
    fn space_used_bytes(&self) -> usize;

    /// Returns the name of the compressor.
    fn name(&self) -> &str;
}

pub trait BlockCompressor: Compressor {
    /// Compresses a single block of data.
    fn compress_block(&mut self, block: &[u8], num_items: usize);

    /// Decompresses a single block of data into the provided buffer.
    fn decompress_block(&self, block_index: usize, buffer: &mut Vec<u8>);

    /// Sets the block size used by this compressor.
    fn set_block_size(&mut self, block_size: usize);

    /// Returns the block size (in bytes) used by this compressor.
    fn get_block_size(&self) -> usize;

    /// Returns the number of blocks stored by the compressor.
    fn get_num_blocks(&self) -> usize;

    /// Returns the block index that contains a given item.
    fn get_block_index(&self, item_index: usize) -> usize;

    /// Returns the buffer used to store the compressed data of a block.
    fn get_block_buffer(&self) -> Vec<u8>;

    /// Returns the start and end indices of the item at the specified index within a block.
    fn get_item_delimiters(&self, block_index: usize, item_index: usize) -> (usize, usize);

    /// Compress the data in blocks according to the block size.
    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let block_size = self.get_block_size();
        let mut block_start = 0;  // Start of the current block
        let mut num_items_in_block = 0;  // Number of items in the current block
        let mut current_block_size = 0;  // Total size of the current block
        let mut item_start = 0;  // Start of the current item

        for &item_end in end_positions {
            let sitem_size = item_end - item_start;
        
            // If adding this item exceeds the block size, compress the current block
            if current_block_size + sitem_size > block_size {
                // Compress the block from block_start up to item_start
                unsafe {
                    self.compress_block(data.get_unchecked(block_start..item_start), num_items_in_block);
                }

                // Reset block parameters for the next block
                block_start = item_start;  // Start the next block from this item
                num_items_in_block = 0;
                current_block_size = 0;
            }

            // Add the current item to the current block
            current_block_size += sitem_size;
            num_items_in_block += 1;

            // Move start to the end of the current item
            item_start = item_end;
        }

        // Compress the last block, if there is any remaining data
        if num_items_in_block > 0 {
            unsafe {
                self.compress_block(data.get_unchecked(block_start..item_start), num_items_in_block);
            }
        }
    }

    /// Decompress all blocks.
    fn decompress(&self, buffer: &mut Vec<u8>) {
        for i in 0..self.get_num_blocks() {
            self.decompress_block(i, buffer);
        }
    }

    /// Retrieves an item by finding the correct block and decompressing only what is needed.
    fn get_item_at(&self, index: usize, buffer: &mut Vec<u8>) {
        // Find the block that contains the item
        let block_index = self.get_block_index(index);

        // Decompress the block containing the item
        let mut block_buffer = self.get_block_buffer();
        self.decompress_block(block_index, &mut block_buffer);

        // Find the item delimiters within the block
        let (start, end) = self.get_item_delimiters(block_index, index);

        // Retrieve the item starting at block_offset using an optimized approach
        unsafe {
            // Get a slice of the item without bounds checking
            let item_slice = block_buffer.get_unchecked(start..end);

            // Extend the buffer with the item slice
            buffer.extend_from_slice(item_slice);
        }
    }
}
