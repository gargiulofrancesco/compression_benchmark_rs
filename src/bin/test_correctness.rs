use compression_benchmark_rs::compressor::copy::CopyCompressor;
use compression_benchmark_rs::compressor::fsst::FSSTCompressor;
use compression_benchmark_rs::compressor::lz4::LZ4Compressor;
use compression_benchmark_rs::compressor::snappy::SnappyCompressor;
use compression_benchmark_rs::compressor::bpe4::BPECompressor;
use compression_benchmark_rs::{compressor::Compressor, dataset::process_dataset, dataset::Dataset};
use std::env;
use std::fs;
use std::path::Path;

pub fn test<T: Compressor>(compressor: &mut T, data: &[u8], end_positions: &[usize]) {
    let mut buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);  // Buffer for decompression

    // Compression and Decompression Test
    compressor.compress(&data, &end_positions);  // Compress the dataset
    compressor.decompress(&mut buffer);  // Decompress the dataset
    for (i, byte) in buffer.iter().enumerate() {
        assert_eq!(data[i], *byte, 
            "Decompressed data does not match original data at index {} for compressor: {}", i, compressor.name());
    }

    // Random Access Test
    for query in 0..end_positions.len() {
        buffer.clear();  // Clear the buffer for random access decompression
        compressor.get_item_at(query, &mut buffer);  // The item obtained through random access
        let start = *end_positions.get(query-1).unwrap_or(&0);
        for (i, byte) in buffer.iter().enumerate() {
            assert_eq!(data[start + i], *byte, 
                "Random access at index {} failed for compressor: {}", query, compressor.name());
        }
    }
}

enum CompressorEnum {
    Copy(CopyCompressor),
    FSST(FSSTCompressor),
    LZ4(LZ4Compressor),
    Snappy(SnappyCompressor),
    BPE(BPECompressor),   
}

fn initialize_compressors(data_size: usize, n_elements: usize) -> Vec<CompressorEnum> {
    vec![
        CompressorEnum::Copy(CopyCompressor::new(data_size, n_elements)),
        CompressorEnum::FSST(FSSTCompressor::new(data_size, n_elements)),
        CompressorEnum::LZ4(LZ4Compressor::new(data_size, n_elements)),
        CompressorEnum::Snappy(SnappyCompressor::new(data_size, n_elements)),
        CompressorEnum::BPE(BPECompressor::new(data_size, n_elements)),
    ]
}

fn main() {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if a directory argument is provided
    if args.len() < 2 {
        eprintln!("Error: Missing directory argument. Usage is: {} <directory>", args[0]);
        std::process::exit(1);
    }

    let directory = &args[1];

    // Check if the path is a valid directory
    let dir = Path::new(directory);
    if !dir.exists() || !dir.is_dir() {
        eprintln!("Error: {} is not a valid directory.", directory);
        std::process::exit(1);
    }

    // Load all datasets from the specified directory
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            // Load the dataset from the JSON file
            let dataset = Dataset::load(&path);            
            println!("Testing dataset: {}", dataset.dataset_name);

            let (_, data, end_positions) = process_dataset(&dataset);
            let data_size = data.len();
            
            let mut compressors = initialize_compressors(data_size, end_positions.len());

            for compressor_enum in &mut compressors {
                match compressor_enum {
                    CompressorEnum::Copy(compressor) => {
                        test(compressor, &data, &end_positions);
                    }
                    CompressorEnum::FSST(compressor) => {
                        test(compressor, &data, &end_positions);
                    }
                    CompressorEnum::LZ4(compressor) => {
                        test(compressor, &data, &end_positions);
                    }
                    CompressorEnum::Snappy(compressor) => {
                        test(compressor, &data, &end_positions);
                    }
                    CompressorEnum::BPE(compressor) => {
                        test(compressor, &data, &end_positions);
                    }                    
                }
            }
        }
    }
}