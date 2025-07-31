//! Individual benchmark executor for compression algorithm evaluation
//!
//! This binary performs isolated performance measurement of a single compression algorithm
//! on a single dataset. Metrics collected include:
//! - Compression ratio and throughput (MiB/s)
//! - Decompression throughput (MiB/s) 
//! - Random access latency (ns)
//!
//! Results are appended to a JSON file for aggregation by the main benchmark harness.
//! CPU core affinity can be specified for consistent measurements in controlled environments.

use compression_benchmark_rs::benchmark_utils::*;
use compression_benchmark_rs::compressor::bpe::BPECompressor;
use compression_benchmark_rs::compressor::onpair_bv::OnPairBVCompressor;
use compression_benchmark_rs::compressor::Compressor;
use compression_benchmark_rs::compressor::raw::RawCompressor;
use compression_benchmark_rs::compressor::onpair16::OnPair16Compressor;
use compression_benchmark_rs::compressor::onpair::OnPairCompressor;
use std::path::Path;
use std::time::Instant;

/// Number of random access queries for latency measurement
const N_QUERIES: usize = 1000000;

/// Wrapper enum for compression algorithm implementations
enum CompressorEnum {
    Raw(RawCompressor),
    BPE(BPECompressor),
    OnPair(OnPairCompressor), 
    OnPair16(OnPair16Compressor),
    OnPairBV(OnPairBVCompressor),
}

/// Individual benchmark execution entry point
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <dataset_path> <compressor_name> <output_file> [core_id]", args[0]);
        std::process::exit(1);
    }

    let dataset_path = &args[1];
    let compressor_name = &args[2];
    let output_file = &args[3];
    let core_id = if args.len() > 4 {
        Some(args[4].parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: Invalid core_id '{}'. Must be a valid number.", args[4]);
            std::process::exit(1);
        }))
    } else {
        None
    };

    // Check if dataset path exists and is a file
    let dataset_path = Path::new(dataset_path);
    if !dataset_path.exists() {
        eprintln!("Error: Dataset path '{}' does not exist.", dataset_path.display());
        std::process::exit(1);
    }
    if !dataset_path.is_file() {
        eprintln!("Error: Dataset path '{}' is not a file.", dataset_path.display());
        std::process::exit(1);
    }

    // Set CPU affinity if specified
    if let Some(core) = core_id {
        if !try_set_affinity(core) {
            eprintln!("Warning: Failed to set CPU affinity to core {}. Continuing without core pinning.", core);
        }
    }
    
    // Load dataset
    let dataset_name = dataset_path.file_name().unwrap().to_str().unwrap().to_string();
    let (data, end_positions) = process_dataset(dataset_path);
    let n_elements = end_positions.len() - 1;
    let queries = generate_random_queries(n_elements, N_QUERIES);

    // Initialize the compressor
    let mut compressor = match compressor_name.as_str() {
        "raw" => CompressorEnum::Raw(RawCompressor::new(data.len(), end_positions.len()-1)),
        "bpe" => CompressorEnum::BPE(BPECompressor::new(data.len(), end_positions.len()-1)),
        "onpair" => CompressorEnum::OnPair(OnPairCompressor::new(data.len(), end_positions.len()-1)),
        "onpair16" => CompressorEnum::OnPair16(OnPair16Compressor::new(data.len(), end_positions.len()-1)),
        "onpair_bv" => CompressorEnum::OnPairBV(OnPairBVCompressor::new(data.len(), end_positions.len()-1)),
        _ => {
            eprintln!("Unknown compressor: {}", compressor_name);
            std::process::exit(1);
        }
    };

    let result = match compressor {
        CompressorEnum::Raw(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::BPE(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::OnPair(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::OnPair16(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::OnPairBV(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
    };

    // Append the result to the file
    append_benchmark_result(&result, Path::new(output_file));
}

/// Core benchmark function implementing the measurement protocol
/// 
/// Executes the complete evaluation pipeline:
/// 1. Compression phase with timing measurement
/// 2. Full decompression with validation and timing  
/// 3. Random access evaluation over N_QUERIES uniformly distributed queries
/// 4. Data integrity verification at each step
///
/// Returns aggregated performance metrics for statistical analysis.
fn benchmark<T: Compressor>(
    compressor: &mut T, 
    dataset_name: String, 
    data: &[u8], 
    end_positions: &[usize], 
    queries: &[usize]
) -> BenchmarkResult {
    let mut buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    buffer.resize(data.len() + 1024, 0);
    let data_bytes = data.len() as f64;

    // Phase 1: Compression measurement
    let start_compression = Instant::now();
    compressor.compress(&data, end_positions);
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data_bytes / compressor.space_used_bytes() as f64;
    let compression_speed = (data_bytes / (1024.0 * 1024.0)) / compression_time;    

    // Phase 2: Decompression measurement with validation
    let start_decompression = Instant::now();
    compressor.decompress(&mut buffer);
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = (data_bytes / (1024.0 * 1024.0)) / decompression_time;

    // Verify decompression correctness
    if !data.eq(&buffer[..data.len()]) {
        panic!("Data mismatch during decompression for compressor: {}", compressor.name());
    }

    // Phase 3: Random access latency measurement
    let mut random_access_times: Vec<u128> = Vec::new();
    for &query in queries {
        let start_position = end_positions[query];
        let end_position = end_positions[query+1];
        let item_size = end_position - start_position;

        let start_random_access = Instant::now();
        compressor.get_item_at(query, &mut buffer);
        let random_access_time = start_random_access.elapsed().as_nanos();
        random_access_times.push(random_access_time);

        // Verify random access correctness
        if !data[start_position..end_position].eq(&buffer[..item_size]) {
            panic!("Data mismatch during random access for compressor: {}", compressor.name());
        }
    }
    
    let average_random_access_time = random_access_times.iter().sum::<u128>() / random_access_times.len() as u128;

    BenchmarkResult {
        dataset_name: dataset_name,
        compressor_name: compressor.name().to_string(),
        compression_rate,
        compression_speed,
        decompression_speed,
        average_random_access_time
    }
}
