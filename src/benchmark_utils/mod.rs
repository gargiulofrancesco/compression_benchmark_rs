//! Benchmark utilities and data structures for compression algorithm evaluation
//!
//! This module provides core infrastructure for systematic performance measurement
//! of string compression algorithms, including:
//! - Dataset loading and preprocessing
//! - Random query generation for access pattern simulation  
//! - Result aggregation and statistical analysis
//! - CPU affinity management for reproducible measurements

use prettytable::{row, Table};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(target_os = "linux")]
use libc::{self, cpu_set_t, CPU_SET, CPU_ZERO};
use rand::{thread_rng, Rng};
use rand::distributions::Uniform;

/// Performance metrics for a single algorithm-dataset combination
#[derive(Serialize, Deserialize, Clone)]
pub struct BenchmarkResult {
    pub dataset_name: String,
    pub compressor_name: String,
    pub compression_rate: f64,              // Space reduction factor
    pub compression_speed: f64,             // Throughput in MiB/s
    pub decompression_speed: f64,           // Throughput in MiB/s
    pub average_random_access_time: u128,   // Latency in nanoseconds
}

/// Loads and preprocesses JSON string datasets for benchmark evaluation
/// 
/// Expects JSON format: array of strings representing individual strings.
/// Returns flattened byte representation and positional metadata for efficient
/// random access during benchmark execution.
/// 
/// # Arguments
/// - `path`: Path to the JSON dataset file
///
/// # Returns
/// - `Vec<u8>`: Concatenated string data as bytes
/// - `Vec<usize>`: Boundary positions starting with 0, then cumulative string lengths.
///   String i is located at `data[end_positions[i]..end_positions[i+1]]`
pub fn load_dataset(path: &Path) -> (Vec<u8>, Vec<usize>) {
    let content = fs::read_to_string(path).unwrap();
    let strings: Vec<String> = serde_json::from_str(&content).unwrap();

    let data: Vec<u8> = strings.iter().flat_map(|s| s.as_bytes()).copied().collect();
    let mut end_positions: Vec<usize> = Vec::new();

    // Start with 0, then append cumulative string lengths for boundary indexing
    end_positions.push(0);
    for str in strings.iter() {
        end_positions.push(end_positions.last().unwrap() + str.len());
    }

    (data, end_positions)
}

/// Generates uniformly distributed random queries for access pattern simulation
/// 
/// Creates a representative workload for random access performance measurement.
/// Uniform distribution ensures unbiased latency assessment across all dataset strings.
///
/// # Arguments
/// - `n`: Total number of strings in dataset
/// - `n_queries`: Number of random queries to generate
/// 
/// # Returns
/// - `Vec<usize>`: Vector of random indices within the range [0, n)
pub fn generate_random_queries(n: usize, n_queries: usize) -> Vec<usize> {
    let mut rng = thread_rng();
    let dist = Uniform::from(0..n);
    let mut queries = Vec::with_capacity(n_queries);

    for _ in 0..n_queries {
        queries.push(rng.sample(&dist));
    }

    queries
}

/// Reads benchmark results from a JSON file
/// 
/// Loads previously saved benchmark results for analysis or continuation of benchmarking.
/// Returns empty vector if file doesn't exist or cannot be parsed.
///
/// # Arguments
/// - `file_path`: Path to the JSON results file
/// 
/// # Returns
/// - `Vec<BenchmarkResult>`: Loaded benchmark results
pub fn read_benchmark_results(file_path: &str) -> Vec<BenchmarkResult> {
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

/// Appends a new benchmark result to the results file
/// 
/// Reads existing results, appends the new result, and writes back to file.
/// Creates the file if it doesn't exist. Preserves all existing results.
///
/// # Arguments
/// - `result`: The new benchmark result to append
/// - `output_path`: Path to the output JSON file 
pub fn append_benchmark_result(result: &BenchmarkResult, output_path: &Path) {
    let mut results: Vec<BenchmarkResult> = if output_path.exists() {
        // Read existing results from the file if it exists
        let data = fs::read_to_string(output_path).expect("Failed to read file");
        serde_json::from_str(&data).expect("Failed to deserialize existing results")
    } else {
        // If the file doesn't exist, start with an empty vector
        Vec::new()
    };

    // Append the new result to the vector
    results.push(result.clone());

    // Serialize the vector and write it back to the file
    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize results");
    fs::write(output_path, json).expect("Failed to write results to file");
}

/// Prints formatted benchmark results grouped by compressor
/// 
/// Groups results by compressor and dataset, calculates averages for each combination,
/// then displays results in a tabular format with overall averages per compressor.
/// 
/// # Arguments
/// - `results`: Vector of benchmark results to display
pub fn print_benchmark_results(results: &[BenchmarkResult]) {
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
        let avg_average_random_access_time = group.iter().map(|r| r.average_random_access_time).sum::<u128>() / group.len() as u128;

        // Store the averaged result
        let averaged_result = BenchmarkResult {
            dataset_name: dataset,
            compressor_name: compressor.clone(),
            compression_rate: avg_compression_rate,
            compression_speed: avg_compression_speed,
            decompression_speed: avg_decompression_speed,
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
            "Comp. Rate",
            "Comp. Speed (MiB/s)",
            "Decomp. Speed (MiB/s)",
            "Avg. Random Access Time (ns)"
        ]);

        // Add rows for each averaged result
        for result in &sorted_results {
            table.add_row(row![
                &result.dataset_name,
                format!("{:.3}", result.compression_rate),
                format!("{:.2}", result.compression_speed),
                format!("{:.2}", result.decompression_speed),
                format!("{}", result.average_random_access_time),
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
        let overall_avg_random_access_time =
            sorted_results.iter().map(|r| r.average_random_access_time).sum::<u128>() / sorted_results.len() as u128;

        // Add overall averages row
        table.add_row(row![
            "AVERAGE",
            format!("{:.3}", overall_avg_compression_rate),
            format!("{:.2}", overall_avg_compression_speed),
            format!("{:.2}", overall_avg_decompression_speed),
            format!("{}", overall_avg_random_access_time),
        ]);

        // Print the table for this compressor
        println!("\nResults for Compressor: {}", compressor);
        table.printstd();
    }
}

/// Attempts to set CPU affinity for reproducible measurements
/// 
/// Tries to bind the current process to a specific CPU core to reduce
/// measurement variance. Only supported on Linux systems.
/// 
/// # Arguments
/// - `core_id`: The CPU core ID to bind to
/// 
/// # Returns
/// - `bool`: True if CPU affinity was successfully set, false otherwise
#[cfg(target_os = "linux")]
pub fn try_set_affinity(core_id: usize) -> bool {
    unsafe {
        let mut cpuset: cpu_set_t = std::mem::zeroed();
        CPU_ZERO(&mut cpuset);
        CPU_SET(core_id, &mut cpuset);
        
        libc::sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset) == 0
    }
}

#[cfg(not(target_os = "linux"))]
pub fn try_set_affinity(_core_id: usize) -> bool {
    // CPU affinity is not supported on this platform
    false
}
