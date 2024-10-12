use std::error::Error;
use crate::compressor::Compressor;

pub struct CopyCompressor {
    data: Vec<String>,
}

impl CopyCompressor {
    pub fn new() -> Self {
        CopyCompressor {
            data: Vec::new(),
        }
    }
}

impl Compressor for CopyCompressor {
    fn compress(&mut self, data: &[String]) -> Result<(), Box<dyn Error>> {
        for s in data {
            self.data.push(s.clone());
        }

        Ok(())
    }

    fn decompress(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut decompressed_strings = Vec::new();
        for s in &self.data {
            decompressed_strings.push(s.clone());
        }

        Ok(decompressed_strings)
    }

    fn get_string_at(&self, index: usize) -> Result<String, Box<dyn Error>> {
        Ok(self.data[index].clone())
    }

    fn space_used_bytes(&self) -> usize {
        self.data.iter().map(|s| s.len()).sum()
    }
    
    fn name(&self) -> &str {
        "Copy"
    }
}
