use random_access_string_compression::compressor::lz4::LZ4Compressor;
use random_access_string_compression::compressor::copy::CopyCompressor;
use random_access_string_compression::dataset::{load_datasets, Dataset};
use random_access_string_compression::compressor::Compressor;
use std::error::Error;
use std::path::Path;

/// Test the compressor for correctness on each dataset
///
/// - Ensures that after compression and decompression, the data is unchanged.
/// - Tests that random access works correctly by comparing with the original data.
pub fn test_compressors(datasets: &[Dataset]) -> Result<(), Box<dyn Error>> {
    // Iterate over each dataset
    for dataset in datasets {
        println!("Testing dataset: {}", dataset.dataset_name);

        // Initialize compressors
        let mut compressors: Vec<Box<dyn Compressor>> = vec![
            Box::new(CopyCompressor::new()),  // Copy compressor
            Box::new(LZ4Compressor::new(64 * 1024)),  // LZ4 with 64 KB block size
        ];

        // For each dataset, test each compressor
        for compressor in compressors.iter_mut() {
            let compressor_name = compressor.name().to_string();
            println!("  Testing compressor: {}", compressor_name);

            // === Compression and Decompression Test ===
            compressor.compress(&dataset.data)?;  // Compress the dataset
            let decompressed_data = compressor.decompress()?;  // Decompress the dataset
            for i in 0..dataset.data.len() {
                assert_eq!(dataset.data[i], decompressed_data[i], 
                    "Decompressed data does not match original data at index {} for compressor: {}", i, compressor_name);
            }

            // === Random Access Test ===
            for query in 0..dataset.data.len(){ // &dataset.queries {
                let original_string = &dataset.data[query];  // The original string at this index
                let accessed_string = compressor.get_string_at(query)?;  // The string obtained through random access

                assert_eq!(original_string, &accessed_string, 
                    "Random access at index {} failed for compressor: {}", query, compressor_name);
            }
        }
    }

    Ok(())
}

fn main () {
    let dir = Path::new("../../data/samples");
    let datasets = load_datasets(dir).unwrap();

    // Run the correctness tests on each dataset
    test_compressors(&datasets).unwrap();
}