use super::Compressor;
use onpair_rs::OnPair16Mini;

/// OnPair16Mini compressor with 16-byte token length constraint
/// 
/// Length-constrained variant that trades some compression effectiveness for
/// significant performance improvements in both compression and decompression.
pub struct OnPair16MiniCompressor {
    onpair16_mini: OnPair16Mini
}

impl Compressor for OnPair16MiniCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        let onpair16_mini = OnPair16Mini::with_capacity(data_size, n_elements);
        OnPair16MiniCompressor { onpair16_mini }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        self.onpair16_mini.compress_bytes(data, end_positions);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        self.onpair16_mini.decompress_all(buffer)
    }
    
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        self.onpair16_mini.decompress_string(index, buffer)
    }

    fn space_used_bytes(&self) -> usize {
        self.onpair16_mini.space_used()
    }

    fn name(&self) -> &str {
        "OnPair16 Mini"
    }
}