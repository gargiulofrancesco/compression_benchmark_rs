use super::Compressor;

pub struct FSSTCompressor {
    data: Vec<u8>,                  // Store the "compressed" data as bytes
    end_positions: Vec<usize>,      // Store the end positions of each element
    compressor: Option<fsst::Compressor>,
}

impl Compressor for FSSTCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        Self {
            data: Vec::with_capacity(data_size),
            end_positions: Vec::with_capacity(n_elements + 1),
            compressor: None,
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let lines = to_lines(data, end_positions);
        let compressor = fsst::Compressor::train(&lines);
        self.end_positions.push(0);

        for text in lines {
            unsafe { compressor.compress_append(text, &mut self.data); }
            self.end_positions.push(self.data.len());
        }

        self.compressor = Some(compressor);
    }

    fn decompress(&self, buffer: &mut Vec<u8>) {
        let decompressor = self.compressor.as_ref().unwrap().decompressor();
        decompressor.decompress_into(&self.data, buffer);
    }

    /// Retrieves an item starting at the specified index.
    #[inline(always)]
    fn get_item_at(&self, index: usize, buffer: &mut Vec<u8>) {
        unsafe {
            let start = *self.end_positions.get_unchecked(index);
            let end = *self.end_positions.get_unchecked(index + 1);
            let data = &self.data[start..end];
            let decompressor = self.compressor.as_ref().unwrap().decompressor();
            decompressor.decompress_into(data, buffer);
        }
    }

    fn space_used_bytes(&self) -> usize {
        let decompressor = self.compressor.as_ref().unwrap().decompressor();
        self.data.len() + decompressor.space_used_bytes()
    }

    fn name(&self) -> &str {
        "FSST"
    }
}

fn to_lines<'a>(data: &'a [u8], end_positions: &[usize]) -> Vec<&'a [u8]> {
    let mut start = 0; // Initialize the starting index

    end_positions
    .iter()
    .map(|&end| {
        let segment = &data[start..end]; // Slice from start to the current end
        start = end; // Update start for the next segment
        segment
    })
    .collect()
}