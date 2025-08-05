use super::Compressor;
use onpair_rs::OnPair16;

/// OnPair compressor with 16-byte token length constraint
/// 
/// Length-constrained variant that trades some compression effectiveness for
/// significant performance improvements in both compression and decompression.
pub struct OnPair16Compressor {
    onpair16: OnPair16
}

impl Compressor for OnPair16Compressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        let onpair16 = OnPair16::with_capacity(data_size, n_elements);
        OnPair16Compressor { onpair16 }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        self.onpair16.compress_bytes(data, end_positions);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        self.onpair16.decompress_all(buffer)
    }
    
    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        self.onpair16.decompress_string(index, buffer)
    }

    fn space_used_bytes(&self) -> usize {
        self.onpair16.space_used()
    }

    fn name(&self) -> &str {
        "OnPair16"
    }
}