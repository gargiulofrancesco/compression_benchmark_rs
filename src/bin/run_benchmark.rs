use compression_benchmark_rs::dataset::BenchmarkResult;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use prettytable::{Table, row};

const COMPRESSORS: [&str; 7] = ["copy", "fsst", "lz4", "snappy", "zstd", "bpe", "on-pair"];
const BENCHMARK_PATH: &str = "./run_single_benchmark";
const OUTPUT_FILE: &str = "benchmark_results.json";
const N_ITERATIONS: usize = 5;

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

    // Load all datasets from the specified directory
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            let dataset_path = path.to_str().unwrap();
            println!("Processing dataset \"{}\"", dataset_path);
            
            for &compressor in COMPRESSORS.iter() {
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
    let results = read_results(OUTPUT_FILE);
    print_benchmark_results_with_averages(&results);
}

fn read_results(file_path: &str) -> Vec<BenchmarkResult> {
    if Path::new(file_path).exists() {
        let file_content = fs::read_to_string(file_path).expect("Failed to read file");
        serde_json::from_str::<Vec<BenchmarkResult>>(&file_content).unwrap_or_else(|_| {
            eprintln!("Error parsing results file '{}'. Starting fresh.", file_path);
            Vec::new()
        })
    } else {
        Vec::new()
    }
}

fn print_benchmark_results_with_averages(results: &[BenchmarkResult]) {
    // Group results by compressor and dataset name
    let mut grouped_results: HashMap<(String, String), Vec<&BenchmarkResult>> = HashMap::new();
    for result in results {
        grouped_results
            .entry((result.compressor_name.clone(), result.dataset_name.clone()))
            .or_default()
            .push(result);
    }

    // A map to store results grouped by compressor name
    let mut compressor_groups: HashMap<String, Vec<BenchmarkResult>> = HashMap::new();

    // Calculate averaged results for each (compressor, dataset) pair
    for ((compressor, dataset), group) in grouped_results {
        let len = group.len() as f64;
        let avg_compression_rate = group.iter().map(|r| r.compression_rate).sum::<f64>() / len;
        let avg_compression_speed = group.iter().map(|r| r.compression_speed).sum::<f64>() / len;
        let avg_decompression_speed = group.iter().map(|r| r.decompression_speed).sum::<f64>() / len;
        let avg_random_access_speed = group.iter().map(|r| r.random_access_speed).sum::<f64>() / len;
        let avg_average_random_access_time = group.iter().map(|r| r.average_random_access_time).sum::<f64>() / len;

        // Store the averaged result
        let averaged_result = BenchmarkResult {
            dataset_name: dataset,
            compressor_name: compressor.clone(),
            compression_rate: avg_compression_rate,
            compression_speed: avg_compression_speed,
            decompression_speed: avg_decompression_speed,
            random_access_speed: avg_random_access_speed,
            average_random_access_time: avg_average_random_access_time,
        };

        compressor_groups
            .entry(compressor)
            .or_default()
            .push(averaged_result);
    }

    // Print results grouped by compressor
    for (compressor, results) in compressor_groups {
        let mut sorted_results = results;
        // Sort results by dataset name
        sorted_results.sort_by(|a, b| a.dataset_name.cmp(&b.dataset_name));

        // Create a new table for each compressor
        let mut table = Table::new();
        table.add_row(row![
            "Dataset",
            "Comp Rate",
            "Comp Speed (MB/s)",
            "Decomp Speed (MB/s)",
            "Random Access Speed (MB/s)",
            "Avg Random Access Time (ns)"
        ]);

        // Add rows for each averaged result
        for result in &sorted_results {
            table.add_row(row![
                &result.dataset_name,
                format!("{:.3}", result.compression_rate),
                format!("{:.2}", result.compression_speed),
                format!("{:.2}", result.decompression_speed),
                format!("{:.2}", result.random_access_speed),
                format!("{}", (result.average_random_access_time * 1_000_000_000.0).round() as u64),
            ]);
        }

        // Calculate overall averages for this compressor
        let len = sorted_results.len() as f64;
        let overall_avg_compression_rate =
            sorted_results.iter().map(|r| r.compression_rate).sum::<f64>() / len;
        let overall_avg_compression_speed =
            sorted_results.iter().map(|r| r.compression_speed).sum::<f64>() / len;
        let overall_avg_decompression_speed =
            sorted_results.iter().map(|r| r.decompression_speed).sum::<f64>() / len;
        let overall_avg_random_access_speed =
            sorted_results.iter().map(|r| r.random_access_speed).sum::<f64>() / len;
        let overall_avg_random_access_time =
            sorted_results.iter().map(|r| r.average_random_access_time).sum::<f64>() / len;

        // Add overall averages row
        table.add_row(row![
            "AVERAGE",
            format!("{:.3}", overall_avg_compression_rate),
            format!("{:.2}", overall_avg_compression_speed),
            format!("{:.2}", overall_avg_decompression_speed),
            format!("{:.2}", overall_avg_random_access_speed),
            format!("{}", (overall_avg_random_access_time * 1_000_000_000.0).round() as u64),
        ]);

        // Print the table for this compressor
        println!("\nResults for Compressor: {}", compressor);
        table.printstd();
    }
}