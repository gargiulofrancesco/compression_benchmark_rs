use std::path::Path;
use std::time::Instant;
use random_access_string_compression::dataset::{load_datasets, Dataset};
use random_access_string_compression::tokenizer::Tokenizer;
use std::error::Error;
use prettytable::{Table, row};

/// Struct to hold the benchmark results
struct BenchmarkResult {
    dataset_name: String,
    dataset_size: f64,
    tokenization_time: f64,
    tokenization_speed: f64,
}

fn run_benchmark(datasets: &[Dataset]) -> Result<Vec<BenchmarkResult>, Box<dyn Error>> {
    let mut results: Vec<BenchmarkResult> = Vec::new();

    // Iterate over each dataset
    for (i, dataset) in datasets.iter().enumerate() {
        println!("({}/{}) Running benchmarks for dataset: {}", i+1, datasets.len(), dataset.dataset_name);
        
        let mut tokenizer = Tokenizer::new(dataset.data.len());
        let dataset_size: f64 = dataset.data.iter().map(|s| s.len()).sum::<usize>() as f64  / (1024.0 * 1024.0);

        // === Compression Benchmark ===
        let start_tokenization = Instant::now();
        for s in dataset.data.iter() {
            tokenizer.tokenize(s);
        }
        let tokenization_time = start_tokenization.elapsed().as_secs_f64();
        let tokenization_speed = dataset_size / tokenization_time;

        // Record the benchmark result for this dataset
        results.push(BenchmarkResult {
            dataset_name: dataset.dataset_name.to_string(),
            dataset_size,
            tokenization_time,
            tokenization_speed,
        });
    }

    Ok(results)
}

/// Print benchmark results in a human-readable format
fn print_benchmark_results(results: &[BenchmarkResult]) {
    let mut table = Table::new();
    
    // Add the header row
    table.add_row(row![
        "Dataset", 
        "Dataset Size (MB)", 
        "Tokenization Time (s)", 
        "Tokenization Speed (MB/s)", 
    ]);
    
    // Add each benchmark result row
    for result in results {
        table.add_row(row![
            result.dataset_name,
            format!("{:.2}", result.dataset_size),
            format!("{:.3}", result.tokenization_time),
            format!("{:.2}", result.tokenization_speed),
        ]);
    }
    
    // Print the table
    table.printstd();

    let average_speed: f64 = results.iter().map(|r| r.tokenization_speed).sum::<f64>() / results.len() as f64;
    println!("Average tokenization speed: {:.2} MB/s", average_speed);
}

fn main () {
    let dir = Path::new("../../data/datasets");
    let datasets = load_datasets(dir).unwrap();

    let benchmark_results = run_benchmark(&datasets).unwrap();
    print_benchmark_results(&benchmark_results);
}
