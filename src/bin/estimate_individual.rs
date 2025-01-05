use compression_benchmark_rs::benchmark_utils::*;
use lz4::block::CompressionMode;
use std::env;
use std::io::{Read, Write};
use std::time::Instant;
use std::path::Path;

const DEFAULT_CORE_ID: usize = 0;

fn main() {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if a directory argument is provided
    if args.len() < 5 {
        eprintln!("Usage: {} <dataset_path> <compressor_name> <compression_level> <output_file> [core_id]\n", args[0]);
        std::process::exit(1);
    }

    let dataset_path = &args[1];
    let compressor_name = &args[2];
    let compression_level = args[3].parse::<i32>().unwrap();
    let output_file = &args[4];
    let core_id = if args.len() > 5 {
        args[5].parse::<usize>().unwrap()
    } else {
        DEFAULT_CORE_ID
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

    // Set CPU affinity
    set_affinity(core_id);

    // Load dataset
    let dataset = Dataset::load(dataset_path);
    let (dataset_name, data, _, _) = process_dataset(&dataset);
    let result: BenchmarkResult;

    match compressor_name.as_str() {
        "deflate" => { result = compress_deflate(&dataset_name, &data, compression_level); },
        "brotli" => { result = compress_brotli(&dataset_name, &data, compression_level); },
        "zstd" => { result = compress_zstd(&dataset_name, &data, compression_level); },
        "lz4" => { result = compress_lz4(&dataset_name, &data, compression_level); },
        "snappy" => { result = compress_snappy(&dataset_name, &data); },
        "xz" => { result = compress_xz(&dataset_name, &data, compression_level); },
        _ => {
            eprintln!("Unknown compressor: {}", compressor_name);
            std::process::exit(1);
        }        
    }

    // Append the result to the file
    append_benchmark_result(&result, Path::new(output_file));
}

fn compress_deflate(dataset_name: &str, data: &[u8], compression_level: i32) -> BenchmarkResult {
    let data_size_mb = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();
    let mut compressor = flate2::Compress::new(flate2::Compression::new(compression_level as u32), false);
    compressor.compress_vec(&data, &mut compression_buffer, flate2::FlushCompress::Finish).unwrap();
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size_mb / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    let mut decompressor = flate2::Decompress::new(false);
    decompressor.decompress_vec(&compression_buffer, &mut decompression_buffer, flate2::FlushDecompress::Finish).unwrap();
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size_mb / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    BenchmarkResult {
        dataset_name: dataset_name.to_string(),
        compressor_name: format!("deflate -{}", compression_level),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed: 0.0,
        average_random_access_time: 0.0
    }
}

fn compress_brotli(dataset_name: &str, data: &[u8], compression_level: i32) -> BenchmarkResult {
    let data_size_mb = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();
    {
        let mut encoder = brotli::CompressorWriter::new(&mut compression_buffer, data.len(), compression_level as u32, 22);
        encoder.write_all(&data).unwrap();
    }
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size_mb / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    {
        let mut decoder = brotli::DecompressorWriter::new(&mut decompression_buffer, data.len() // buffer size hint
        );
        decoder.write_all(&compression_buffer).unwrap();
    }
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size_mb / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    BenchmarkResult {
        dataset_name: dataset_name.to_string(),
        compressor_name: format!("brotli -{}", compression_level),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed: 0.0,
        average_random_access_time: 0.0
    }
}

fn compress_zstd(dataset_name: &str, data: &[u8], compression_level: i32) -> BenchmarkResult {
    let data_size_mb = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();    
    let mut encoder = zstd::stream::Encoder::new(&mut compression_buffer, compression_level).unwrap();
    encoder.write_all(&data).unwrap();
    encoder.finish().unwrap();
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size_mb / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    let mut decoder = zstd::stream::Decoder::new(&compression_buffer[..]).unwrap();
    decoder.read_to_end(&mut decompression_buffer).unwrap();
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size_mb / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    BenchmarkResult {
        dataset_name: dataset_name.to_string(),
        compressor_name: format!("zstd -{}", compression_level),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed: 0.0,
        average_random_access_time: 0.0
    }
}

fn compress_lz4(dataset_name: &str, data: &[u8], compression_level: i32) -> BenchmarkResult {
    let data_size_mb = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();
    unsafe{
        compression_buffer.set_len(data.len());
    }
    let compressed_size = if compression_level == 0 {
        lz4::block::compress_to_buffer(&data, None, false, &mut compression_buffer).unwrap()
    } else {
        lz4::block::compress_to_buffer(&data, Some(CompressionMode::HIGHCOMPRESSION(compression_level)), false, &mut compression_buffer).unwrap()
    };
    unsafe{
        compression_buffer.set_len(compressed_size);
    }
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size_mb / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    unsafe{
        decompression_buffer.set_len(data.len());
    }
    let _ = lz4::block::decompress_to_buffer(&compression_buffer, Some(data.len() as i32), &mut decompression_buffer);
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size_mb / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    BenchmarkResult {
        dataset_name: dataset_name.to_string(),
        compressor_name: format!("lz4 -{}", compression_level),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed: 0.0,
        average_random_access_time: 0.0
    }
}

fn compress_snappy(dataset_name: &str, data: &[u8]) -> BenchmarkResult {
    let data_size_mb = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();
    unsafe { compression_buffer.set_len(snap::raw::max_compress_len(data.len())); }
    let mut encoder = snap::raw::Encoder::new();
    let compressed_bytes = encoder.compress(&data, &mut compression_buffer).expect("Snappy compression failed");
    unsafe { compression_buffer.set_len(compressed_bytes); }
    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size_mb / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    let mut decoder = snap::raw::Decoder::new();
    unsafe { decompression_buffer.set_len(data.len()); }
    decoder.decompress(&compression_buffer, &mut decompression_buffer).expect("Snappy decompression failed");
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size_mb / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    BenchmarkResult {
        dataset_name: dataset_name.to_string(),
        compressor_name: "snappy".to_string(),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed: 0.0,
        average_random_access_time: 0.0
    }
}

fn compress_xz(dataset_name: &str, data: &[u8], compression_level: i32) -> BenchmarkResult {
    let data_size_mb = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();

    {
        let mut encoder = xz2::write::XzEncoder::new(&mut compression_buffer, compression_level as u32);
        encoder.write_all(&data).unwrap();
        encoder.finish().unwrap();
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size_mb / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    
    {
        let mut decoder = xz2::read::XzDecoder::new(&compression_buffer[..]);
        decoder.read_to_end(&mut decompression_buffer).unwrap();
    }

    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size_mb / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    BenchmarkResult {
        dataset_name: dataset_name.to_string(),
        compressor_name: format!("xz -{}", compression_level),
        compression_rate,
        compression_speed,
        decompression_speed,
        random_access_speed: 0.0,
        average_random_access_time: 0.0
    }
}
