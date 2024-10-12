use std::path::Path;
use std::time::Instant;
use random_access_string_compression::compressor::Compressor;
use random_access_string_compression::compressor::lz4::LZ4Compressor;
use random_access_string_compression::dataset::{load_datasets, Dataset};
use std::error::Error;
use prettytable::{Table, row};

/// Struct to hold the benchmark results
struct BenchmarkResult {
    dataset_name: String,
    compressor_name: String,
    compression_rate: f64,
    compression_speed: f64, 
    decompression_speed: f64, 
    random_access_speed: f64,
    average_random_access_time: f64,
}

/// Runs a benchmark on all compressors and datasets
fn run_benchmark(datasets: &[Dataset]) -> Result<Vec<BenchmarkResult>, Box<dyn Error>> {
    let mut results: Vec<BenchmarkResult> = Vec::new();

    // Iterate over each dataset
    for (i, dataset) in datasets.iter().enumerate() {
        println!("({}/{}) Running benchmarks for dataset: {}", i+1, datasets.len(), dataset.dataset_name);

        let dataset_bytes: usize = dataset.data.iter().map(|s| s.len()).sum();
        let random_access_bytes: usize = dataset.queries.iter().map(|&i| dataset.data[i].len()).sum();
        let mut compressors: Vec<Box<dyn Compressor>> = vec![
            Box::new(LZ4Compressor::new(64 * 1024)),  // LZ4 with 64KB blocks
        ];

        // For each dataset, run the benchmark on each compressor
        for compressor in compressors.iter_mut() {
            let compressor_name = compressor.name().to_string();

            // === Compression Benchmark ===
            let start_compression = Instant::now();
            compressor.compress(&dataset.data)?;  // Compress the dataset
            let compression_time = start_compression.elapsed().as_secs_f64();
            let compression_rate = dataset_bytes as f64 / compressor.space_used_bytes() as f64;
            let compression_speed = (dataset_bytes as f64 / (1024.0 * 1024.0)) / compression_time;

            // === Decompression Benchmark ===
            let start_decompression = Instant::now();
            compressor.decompress()?;  // Decompress the dataset
            let decompression_time = start_decompression.elapsed().as_secs_f64();
            let decompression_speed = (dataset_bytes as f64 / (1024.0 * 1024.0)) / decompression_time;

            // === Random Access Benchmark ===
            let mut random_access_times = Vec::new();
            for &query in &dataset.queries {
                let start_random_access = Instant::now();
                compressor.get_string_at(query)?;  // Access the string at index query
                let random_access_time = start_random_access.elapsed().as_secs_f64();
                random_access_times.push(random_access_time);
            }
            let random_access_speed = (random_access_bytes as f64 / (1024.0 * 1024.0)) / random_access_times.iter().sum::<f64>();
            let average_random_access_time: f64 = random_access_times.iter().sum::<f64>() / random_access_times.len() as f64;

            // Record the benchmark result for this compressor
            results.push(BenchmarkResult {
                dataset_name: dataset.dataset_name.to_string(),
                compressor_name: compressor_name.to_string(),
                compression_rate,
                compression_speed,
                decompression_speed,
                random_access_speed,
                average_random_access_time
            });
        }
    }

    Ok(results)
}

/// Print benchmark results in a human-readable format
fn print_benchmark_results(results: &[BenchmarkResult]) {
    let mut table = Table::new();
    
    // Add the header row
    table.add_row(row![
        "Dataset", 
        "Compressor", 
        "Comp Rate", 
        "Comp Speed (MB/s)", 
        "Decomp Speed (MB/s)", 
        "Random Access Speed (MB/s)", 
        "Avg Random Access Time (s)"
    ]);
    
    // Add each benchmark result row
    for result in results {
        table.add_row(row![
            result.dataset_name,
            result.compressor_name,
            format!("{:.3}", result.compression_rate),
            format!("{:.2}", result.compression_speed),
            format!("{:.2}", result.decompression_speed),
            format!("{:.2}", result.random_access_speed),
            format!("{:.9}", result.average_random_access_time),
        ]);
    }
    
    // Print the table
    println!();
    table.printstd();
    println!();
}

fn main () {
    let dir = Path::new("../../data/samples");
    let datasets = load_datasets(dir).unwrap();

    let benchmark_results = run_benchmark(&datasets).unwrap();
    print_benchmark_results(&benchmark_results);
}