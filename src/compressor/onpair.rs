use super::Compressor;
use onpair_rs::OnPair;

/// OnPair compressor with unlimited token length
/// 
/// Core implementation of the OnPair algorithm supporting arbitrary-length tokens.
pub struct OnPairCompressor {
    onpair: OnPair,
}

impl Compressor for OnPairCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        let onpair = OnPair::with_capacity(data_size, n_elements);
        OnPairCompressor { onpair }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        self.onpair.compress_bytes(data, end_positions);
    }

    fn decompress(&self, buffer: &mut [u8]) -> usize {
        self.onpair.decompress_all(buffer)
    }

    fn get_item_at(&mut self, index: usize, buffer: &mut [u8]) -> usize {
        self.onpair.decompress_string(index, buffer)
    }

    fn space_used_bytes(&self) -> usize {
        self.onpair.space_used()
    }

    fn name(&self) -> &str {
        "OnPair"
    }
}