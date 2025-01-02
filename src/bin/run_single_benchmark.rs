use compression_benchmark_rs::compressor::Compressor;
use compression_benchmark_rs::dataset::*;
use compression_benchmark_rs::compressor::copy::CopyCompressor;
use compression_benchmark_rs::compressor::fsst::FSSTCompressor;
use compression_benchmark_rs::compressor::lz4::LZ4Compressor;
use compression_benchmark_rs::compressor::snappy::SnappyCompressor;
use compression_benchmark_rs::compressor::zstd::ZstdCompressor;
use compression_benchmark_rs::compressor::bpe::BPECompressor;
use compression_benchmark_rs::compressor::onpair::OnPairCompressor;
use compression_benchmark_rs::compressor::onpair16::OnPair16Compressor;
use std::fs;
use std::path::Path;
use std::time::Instant;

enum CompressorEnum {
    Copy(CopyCompressor),
    FSST(FSSTCompressor),
    LZ4(LZ4Compressor),
    Snappy(SnappyCompressor),
    Zstd(ZstdCompressor),
    BPE(BPECompressor),
    OnPair(OnPairCompressor), 
    OnPair16(OnPair16Compressor),
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <dataset_path> <compressor_name> <output_file>", args[0]);
        std::process::exit(1);
    }

    let dataset_path = &args[1];
    let compressor_name = &args[2];
    let output_file = &args[3];

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

    // Load dataset
    let dataset = Dataset::load(dataset_path);
    let (dataset_name, data, end_positions, queries) = process_dataset(&dataset);

    // Initialize the compressor
    let mut compressor = match compressor_name.as_str() {
        "copy" => CompressorEnum::Copy(CopyCompressor::new(data.len(), end_positions.len())),
        "fsst" => CompressorEnum::FSST(FSSTCompressor::new(data.len(), end_positions.len())),
        "lz4" => CompressorEnum::LZ4(LZ4Compressor::new(data.len(), end_positions.len())),
        "snappy" => CompressorEnum::Snappy(SnappyCompressor::new(data.len(), end_positions.len())),
        "zstd" => CompressorEnum::Zstd(ZstdCompressor::new(data.len(), end_positions.len())),
        "bpe" => CompressorEnum::BPE(BPECompressor::new(data.len(), end_positions.len())),
        "onpair" => CompressorEnum::OnPair(OnPairCompressor::new(data.len(), end_positions.len())),
        "onpair16" => CompressorEnum::OnPair16(OnPair16Compressor::new(data.len(), end_positions.len())),
        _ => {
            eprintln!("Unknown compressor: {}", compressor_name);
            std::process::exit(1);
        }
    };

    let result = match compressor {
        CompressorEnum::Copy(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::FSST(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::LZ4(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::Snappy(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::Zstd(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::BPE(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::OnPair(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
        CompressorEnum::OnPair16(ref mut c) => benchmark(c, dataset_name, &data, &end_positions, &queries),
    };

    // Append the result to the file
    append_result_to_file(&result, Path::new(output_file));
}

fn benchmark<T: Compressor>(
    compressor: &mut T, 
    dataset_name: String, 
    data: &[u8], 
    end_positions: &[usize], 
    queries: &[usize]
) -> BenchmarkResult {
    let mut buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);  // Buffer for decompression

    let data_bytes = data.len() as f64;
    let random_access_bytes: usize = queries.iter().map(|&i| {
        let prev_position = if i == 0 { 0 } else { end_positions[i - 1] };
        end_positions[i] - prev_position
    }).sum();    

    // Compression
    let start_compression = Instant::now();
    compressor.compress(&data, end_positions);  // Compress the dataset
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data_bytes / compressor.space_used_bytes() as f64;
    let compression_speed = (data_bytes / (1024.0 * 1024.0)) / compression_time;    

    // Decompression
    buffer.clear();
    let start_decompression = Instant::now();
    compressor.decompress(&mut buffer);  // Decompress the dataset
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = (data_bytes / (1024.0 * 1024.0)) / decompression_time;

    // Random Access
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

fn append_result_to_file(result: &BenchmarkResult, file_path: &Path) {
    let mut results: Vec<BenchmarkResult> = if file_path.exists() {
        // Read existing results from the file if it exists
        let data = fs::read_to_string(file_path).expect("Failed to read file");
        serde_json::from_str(&data).expect("Failed to deserialize existing results")
    } else {
        // If the file doesn't exist, start with an empty vector
        Vec::new()
    };

    // Append the new result to the vector
    results.push(result.clone());

    // Serialize the vector and write it back to the file
    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize results");
    fs::write(file_path, json).expect("Failed to write results to file");
}