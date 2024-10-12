use title_compressor::compressor::lz4::LZ4Compressor;
use title_compressor::dataset::{load_datasets, Dataset};
use title_compressor::compressor::Compressor;
use std::error::Error;
use std::path::Path;

/// Test the compressor for correctness on each dataset
///
/// - Ensures that after compression and decompression, the data is unchanged.
/// - Tests that random access works correctly by comparing with the original data.
pub fn test_compressors(test_cases: &[Dataset]) -> Result<(), Box<dyn Error>> {
    // Initialize compressors
    let mut compressors: Vec<Box<dyn Compressor>> = vec![
        Box::new(LZ4Compressor::new(64 * 1024)),
        // Box::new(OtherCompressor::new()),  // Add other compressors here
    ];

    // Iterate over each test case (dataset)
    for test_case in test_cases {
        println!("Testing dataset: {}", test_case.dataset_name);

        // For each test case, test each compressor
        for compressor in compressors.iter_mut() {
            let compressor_name = compressor.name().to_string();
            println!("  Testing compressor: {}", compressor_name);

            // === Compression and Decompression Test ===
            compressor.compress(&test_case.data)?;  // Compress the dataset
            let decompressed_data = compressor.decompress()?;  // Decompress the dataset
            for i in 0..test_case.data.len() {
                assert_eq!(test_case.data[i], decompressed_data[i], 
                    "Decompressed data does not match original data at index {} for compressor: {}", i, compressor_name);
            }

            // === Random Access Test ===
            for &query in &test_case.queries {
                let original_string = &test_case.data[query];  // The original string at this index
                let accessed_string = compressor.get_string_at(query)?;  // The string obtained through random access

                assert_eq!(original_string, &accessed_string, 
                    "Random access at index {} failed for compressor: {}", query, compressor_name);
            }

            println!("  Compressor {} passed all tests for dataset: {}", compressor_name, test_case.dataset_name);
        }
    }

    Ok(())
}

fn main () {
    let dir = Path::new("../../data");
    let datasets = load_datasets(dir).unwrap();

    // Run the correctness tests on each dataset
    test_compressors(&datasets).unwrap();
}