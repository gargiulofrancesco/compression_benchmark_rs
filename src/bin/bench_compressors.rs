use std::path::Path;
use std::time::Instant;
use random_access_string_compression::{compressor::Compressor, dataset::process_dataset};
use random_access_string_compression::compressor::lz4::LZ4Compressor;
use random_access_string_compression::compressor::copy::CopyCompressor;
use random_access_string_compression::dataset::load_datasets;
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

fn benchmark<T: Compressor>(compressor: &mut T, dataset_name: String, data: &[u8], end_positions: &[usize], queries: &[usize]) -> BenchmarkResult {
    let mut buffer: Vec<u8> = Vec::with_capacity(data.len());  // Buffer for decompression

    let data_bytes = data.len() as f64;
    let random_access_bytes: usize = queries.iter().map(|&i| {
        let prev_position = if i == 0 { 0 } else { end_positions[i - 1] };
        end_positions[i] - prev_position
    }).sum();    

    // === Compression Benchmark ===
    let start_compression = Instant::now();
    compressor.compress(&data, end_positions);  // Compress the dataset
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data_bytes / compressor.space_used_bytes() as f64;
    let compression_speed = (data_bytes / (1024.0 * 1024.0)) / compression_time;    

    // === Decompression Benchmark ===
    buffer.clear();
    let start_decompression = Instant::now();
    compressor.decompress(&mut buffer);  // Decompress the dataset
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = (data_bytes / (1024.0 * 1024.0)) / decompression_time;

    // === Random Access Benchmark ===
    let mut random_access_times = Vec::new();
    for &query in queries {
        buffer.clear();
        let start_random_access = Instant::now();
        compressor.get_item_at(query, &mut buffer);  // Access the item at index query
        let random_access_time = start_random_access.elapsed().as_secs_f64();
        random_access_times.push(random_access_time);
    }
    let random_access_speed = (random_access_bytes as f64 / (1024.0 * 1024.0)) / random_access_times.iter().sum::<f64>();
    let average_random_access_time: f64 = random_access_times.iter().sum::<f64>() / random_access_times.len() as f64;

    BenchmarkResult {
        dataset_name: dataset_name,
        compressor_name: compressor.name().to_string(),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed,
        average_random_access_time
    }
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
    table.printstd();

    let average_compression_speed: f64 = results.iter().map(|r| r.compression_speed).sum::<f64>() / results.len() as f64;
    let average_decompression_speed: f64 = results.iter().map(|r| r.decompression_speed).sum::<f64>() / results.len() as f64;
    println!("Average Compression Speed: {:.2} MB/s, Average Decompression Speed: {:.2} MB/s", average_compression_speed, average_decompression_speed);
}

enum CompressorEnum {
    Copy(CopyCompressor),
    LZ4(LZ4Compressor),
}

fn initialize_compressors(data_size: usize, n_elements: usize) -> Vec<CompressorEnum> {
    vec![
        CompressorEnum::Copy(CopyCompressor::new(data_size, n_elements)),
        CompressorEnum::LZ4(LZ4Compressor::new(data_size, n_elements)),
    ]
}

fn main() {
    let dir = Path::new("../../data/samples");
    let datasets = load_datasets(dir);

    let mut results: Vec<BenchmarkResult> = Vec::new();
    for (i, dataset) in datasets.iter().enumerate() {
        println!("{}/{}) Benchmarking dataset: {}", i+1, datasets.len(), dataset.dataset_name);

        let (dataset_name, data, end_positions, queries) = process_dataset(dataset);
        let mut compressors = initialize_compressors(data.len(), end_positions.len());
        for compressor_enum in &mut compressors {
            match compressor_enum {
                CompressorEnum::Copy(compressor) => {
                    let result = benchmark(compressor, dataset_name.clone(), &data, &end_positions, &queries);
                    results.push(result);
                }
                CompressorEnum::LZ4(compressor) => {
                    let result = benchmark(compressor, dataset_name.clone(), &data, &end_positions, &queries);
                    results.push(result);
                }
            }
        }
    }

    print_benchmark_results(&results);
}
