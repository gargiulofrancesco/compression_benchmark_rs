use compression_benchmark_rs::benchmark_utils::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const COMPRESSORS: [&str; 6] = ["copy", "lz4", "snappy", "zstd", "fsst", "onpair16"];
const BENCHMARK_PATH: &str = "./benchmark_individual";
const OUTPUT_FILE: &str = "benchmark_results.json";
const N_ITERATIONS: usize = 15;

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

    // Remove existing results file if it exists
    if Path::new(OUTPUT_FILE).exists() {
        fs::remove_file(OUTPUT_FILE).expect("Failed to remove existing results file");
    }

    // Load datasets from the specified directory
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            let dataset_path = path.to_str().unwrap();
            println!("Processing dataset \"{}\"", dataset_path);
            
            for &compressor in COMPRESSORS.iter() {
                println!("- {}", compressor);
                for _ in 0..N_ITERATIONS {
                    // Execute the benchmark command
                    let status = Command::new(BENCHMARK_PATH)
                        .arg(dataset_path)
                        .arg(compressor)
                        .arg(OUTPUT_FILE)
                        .status()
                        .expect("Failed to execute benchmark");
                    
                    if !status.success() {
                        eprintln!("Benchmark failed for dataset '{}' with compressor '{}'.", dataset_path, compressor);
                    }
                }
            }
        }
    }

    // Print the benchmark results
    let results = read_benchmark_results(OUTPUT_FILE);
    print_benchmark_results(&results);
}