use std::time::Instant;
use crate::compressor::Compressor;
use crate::dataset::Dataset;
use std::error::Error;

use crate::compressor::lz4::LZ4Compressor;

/// Struct to hold the benchmark results
struct BenchmarkResult {
    compressor_name: String,
    compression_time: f64,  // Time in seconds
    compression_rate: f64,  // Ratio of original size to compressed size
    decompression_time: f64,
    random_access_time: f64,
}

/// Runs a benchmark on all compressors and test cases
pub fn run_benchmark(test_cases: &[Dataset]) -> Result<(), Box<dyn Error>> {
    let mut compressors: Vec<Box<dyn Compressor>> = vec![
        Box::new(LZ4Compressor::new(64 * 1024)),  // LZ4 with 64KB blocks
    ];

    let mut results: Vec<BenchmarkResult> = Vec::new();

    // Iterate over each test case
    for test_case in test_cases {
        // For each test case, run the benchmark on each compressor
        for compressor in compressors.iter_mut() {
            let compressor_name = compressor.name().to_string();

            // === Compression Benchmark ===
            let start_compression = Instant::now();
            compressor.compress(&test_case.data)?;  // Compress the dataset
            let compression_time = start_compression.elapsed().as_secs_f64();
            let compression_rate = total_data_size(&test_case.data) as f64 / compressor.space_used_bytes() as f64;

            // === Decompression Benchmark ===
            let start_decompression = Instant::now();
            compressor.decompress()?;  // Decompress the dataset
            let decompression_time = start_decompression.elapsed().as_secs_f64();

            // === Random Access Benchmark ===
            let mut random_access_times = Vec::new();
            for &query in &test_case.queries {
                let start_random_access = Instant::now();
                compressor.get_string_at(query)?;  // Access the string at index query
                let random_access_time = start_random_access.elapsed().as_secs_f64();
                random_access_times.push(random_access_time);
            }
            let average_random_access_time: f64 = random_access_times.iter().sum::<f64>() / random_access_times.len() as f64;

            // Record the benchmark result for this compressor
            results.push(BenchmarkResult {
                compressor_name: compressor_name.to_string(),
                compression_time,
                compression_rate,
                decompression_time,
                random_access_time: average_random_access_time,
            });
        }
    }

    // Output results (you can modify this to output in JSON, CSV, or another format)
    print_benchmark_results(&results);

    Ok(())
}

/// Utility function to compute the total size of the dataset in bytes
fn total_data_size(data: &[String]) -> usize {
    data.iter().map(|s| s.len()).sum()
}

/// Print benchmark results in a human-readable format
fn print_benchmark_results(results: &[BenchmarkResult]) {
    println!("{:<20} {:<15} {:<15} {:<15} {:<20}", 
        "Compressor", "Comp Time (s)", "Comp Rate", "Decomp Time (s)", "Random Access Time (s)");
    for result in results {
        println!("{:<20} {:<15.6} {:<15.6} {:<15.6} {:<20.6}", 
            result.compressor_name, 
            result.compression_time, 
            result.compression_rate, 
            result.decompression_time, 
            result.random_access_time);
    }
}
