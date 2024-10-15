use std::path::Path;
use std::time::Instant;
use random_access_string_compression::dataset::{load_datasets, process_dataset};
use random_access_string_compression::tokenizer::Tokenizer;
use prettytable::{Table, row};

/// Struct to hold the benchmark results
struct BenchmarkResult {
    dataset_name: String,
    dataset_size: f64,
    tokenization_time: f64,
    tokenization_speed: f64,
}

fn benchmark(tokenizer: &mut Tokenizer, dataset_name: String, data: &[u8], end_positions: &[usize]) -> BenchmarkResult {
    let data_size = data.len() as f64;
    let data_size_mb = data_size / (1024.0 * 1024.0);

    // === Compression Benchmark ===
    let start_tokenization = Instant::now();
    tokenizer.tokenize(data, end_positions);
    let tokenization_time = start_tokenization.elapsed().as_secs_f64();
    let tokenization_speed = data_size_mb / tokenization_time;

    BenchmarkResult {
        dataset_name,
        dataset_size: data_size_mb,
        tokenization_time,
        tokenization_speed,
    }
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
    let datasets = load_datasets(dir);

    let mut results: Vec<BenchmarkResult> = Vec::new();
    for (i, dataset) in datasets.iter().enumerate() {
        println!("{}/{}) Processing dataset {}", i+1, datasets.len(), dataset.dataset_name);

        let (dataset_name, data, end_positions, _) = process_dataset(dataset);
        let mut tokenizer = Tokenizer::new(data.len(), end_positions.len());
        let result = benchmark(&mut tokenizer, dataset_name, &data, &end_positions);
        results.push(result);
    }

    print_benchmark_results(&results);    
}
