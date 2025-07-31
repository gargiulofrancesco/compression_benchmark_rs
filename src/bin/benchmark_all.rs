//! Comprehensive benchmark for string compression algorithms
//! 
//! This binary orchestrates systematic evaluation of compression algorithms for
//! string collections with random access requirements. The benchmark suite measures:
//! - Compression ratio and throughput
//! - Decompression throughput  
//! - Random access latency
//!
//! Each algorithm is evaluated across N_ITERATIONS runs for statistical significance.
//! Results are aggregated and persisted in JSON format for further analysis.

use compression_benchmark_rs::benchmark_utils::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Compression algorithms under evaluation
const COMPRESSORS: [&str; 3] = ["raw", "onpair", "onpair16"];
/// Path to individual benchmark executable
const BENCHMARK_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/benchmark_individual");
/// Output file for aggregated benchmark results
const OUTPUT_FILE: &str = "benchmark_results.json";
/// Number of iterations per algorithm-dataset combination for statistical robustness
const N_ITERATIONS: usize = 15;

/// Main benchmark orchestrator
/// 
/// Executes comprehensive evaluation of compression algorithms across all JSON datasets
/// in the specified directory. For each dataset-algorithm pair, performs N_ITERATIONS
/// independent measurements to ensure statistical significance.
fn main() {
    // Parse command-line arguments: dataset directory and optional CPU core ID
    let args: Vec<String> = env::args().collect();

    // Validate command-line interface
    if args.len() < 2 {
        eprintln!("Usage: {} <directory> [core_id]", args[0]);
        eprintln!("  <directory>  - Directory containing JSON dataset files");
        eprintln!("  [core_id]    - Optional CPU core ID for pinning");
        std::process::exit(1);
    }

    let directory = &args[1];
    // Optional CPU core affinity for consistent performance measurements
    let core_id = if args.len() > 2 {
        Some(args[2].parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: Invalid core_id '{}'. Must be a valid number.", args[2]);
            std::process::exit(1);
        }))
    } else {
        None
    };

    // Validate dataset directory
    let dir = Path::new(directory);
    if !dir.exists() || !dir.is_dir() {
        eprintln!("Error: {} is not a valid directory.", directory);
        std::process::exit(1);
    }

    // Initialize clean results file for this benchmark run
    if Path::new(OUTPUT_FILE).exists() {
        fs::remove_file(OUTPUT_FILE).expect("Failed to remove existing results file");
    }

    // Systematic evaluation across all datasets and compression algorithms
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Process only JSON dataset files
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            let dataset_path = path.to_str().unwrap();
            println!("Processing dataset \"{}\"", dataset_path);
            
            // Evaluate each compression algorithm
            for &compressor in COMPRESSORS.iter() {
                println!("- {}", compressor);
                // Multiple iterations for statistical robustness
                for _ in 0..N_ITERATIONS {
                    // Execute individual benchmark with specified parameters
                    let mut cmd = Command::new(BENCHMARK_PATH);
                    cmd.arg(dataset_path)
                       .arg(compressor)
                       .arg(OUTPUT_FILE);
                    
                    // Apply CPU core affinity if specified
                    if let Some(core) = core_id {
                        cmd.arg(core.to_string());
                    }
                    
                    let status = cmd.status().expect("Failed to execute benchmark");
                    
                    if !status.success() {
                        eprintln!("Benchmark failed for dataset '{}' with compressor '{}'.", dataset_path, compressor);
                    }
                }
            }
        }
    }

    // Generate comprehensive benchmark report
    let results = read_benchmark_results(OUTPUT_FILE);
    print_benchmark_results(&results);
}