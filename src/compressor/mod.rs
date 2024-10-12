pub mod lz4;

use std::error::Error;

/// Represents a trait for compressors with internal state.
pub trait Compressor {
    /// Compresses the provided data and stores it internally.
    fn compress(&mut self, data: &[String]) -> Result<(), Box<dyn Error>>;

    /// Decompresses the internally stored data and returns it.
    fn decompress(&self) -> Result<Vec<String>, Box<dyn Error>>;

    /// Retrieves a string at the specified index with minimal decompression.
    fn get_string_at(&self, index: usize) -> Result<String, Box<dyn Error>>;

    /// Returns the amount of space used by the compressed data in bytes.
    fn space_used_bytes(&self) -> usize;

    /// Returns the name of the compressor.
    fn name(&self) -> &str;
}