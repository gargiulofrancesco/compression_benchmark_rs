use super::Compressor;
use onpair_rs::OnPairMini;

/// OnPairMini compressor with unlimited token length
/// 
/// Core implementation of the OnPairMini algorithm supporting arbitrary-length tokens.
pub struct OnPairMiniCompressor {
    onpair_mini: OnPairMini,
}

impl Compressor for OnPairMiniCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        let onpair_mini = OnPairMini::with_capacity(data_size, n_elements);
        OnPairMiniCompressor { onpair_mini }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        self.onpair_mini.compress_bytes(data, end_positions);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        self.onpair_mini.decompress_all(buffer)
    }

    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        self.onpair_mini.decompress_string(index, buffer)
    }

    fn space_used_bytes(&self) -> usize {
        self.onpair_mini.space_used()
    }

    fn name(&self) -> &str {
        "OnPair Mini"
    }
}