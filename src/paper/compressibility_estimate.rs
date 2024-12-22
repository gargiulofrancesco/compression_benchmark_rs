use compression_benchmark_rs::dataset::{process_dataset, Dataset};
use lz4::block::CompressionMode;
use std::path::PathBuf;
use std::env;
use std::io::{Read, Write};
use std::time::Instant;
use std::{fs, path::Path};

const N_ITER: usize = 10;

struct CompressionResult {
    size: f64,
    rate: f64,
    c_speed: f64,
    d_speed: f64,
}

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

    // Load all datasets from the specified directory
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() {
            let filepath_str = path.as_os_str().to_str().unwrap();
            if filepath_str == "/home/gargiulo/data/corpus/book_titles.json" {
                continue;
            }
            
            println!("\nProcessing: {}", filepath_str);
            
            // LZ4
            {
                let mut results = Vec::new();
                for _ in 0..N_ITER {
                    let result = compress_lz4(&path, CompressionMode::DEFAULT);
                    results.push(result);
                }
                print_results(&results, &"LZ4 Default".to_string());
            }

            // LZ4HC
            for compression_level in vec![1, 3, 6, 9, 12] {
                let mut results = Vec::new();
                for _ in 0..N_ITER {
                    let result = compress_lz4(&path, CompressionMode::HIGHCOMPRESSION(compression_level));
                    results.push(result);
                }
                print_results(&results, &format!("LZ4HC -{}", compression_level));
            }

            // Zstd
            for compression_level in vec![1, 3, 6, 9, 12] {
                let mut results = Vec::new();
                for _ in 0..N_ITER {
                    let result = compress_zstd(&path, compression_level);
                    results.push(result);
                }
                print_results(&results, &format!("Zstd -{}", compression_level));
            }

            // deflate
            for compression_level in vec![1, 6, 9] {
                let mut results = Vec::new();
                for _ in 0..N_ITER {
                    let result = compress_deflate(&path, compression_level);
                    results.push(result);
                }
                print_results(&results, &format!("deflate -{}", compression_level));
            }

            // xz
            for compression_level in vec![1, 3] {
                let mut results = Vec::new();
                for _ in 0..N_ITER {
                    let result = compress_xz(&path, compression_level);
                    results.push(result);
                }
                print_results(&results, &format!("xz -{}", compression_level));
            }

            // brotli
            for compression_level in vec![1, 3, 6] {
                let mut results = Vec::new();
                for _ in 0..N_ITER {
                    let result = compress_brotli(&path, compression_level);
                    results.push(result);
                }
                print_results(&results, &format!("brotli -{}", compression_level));
            }

            // snappi
            let mut results = Vec::new();
            for _ in 0..N_ITER {
                let result = compress_snappi(&path);
                results.push(result);
            }
            print_results(&results, &format!("snappi"));
        }
    }
}

fn get_average(results: &[CompressionResult]) -> CompressionResult {
    let count = results.len() as f64;

    // Sum the values for each field
    let total_size: f64 = results.iter().map(|r| r.size).sum();
    let total_rate: f64 = results.iter().map(|r| r.rate).sum();
    let total_c_speed: f64 = results.iter().map(|r| r.c_speed).sum();
    let total_d_speed: f64 = results.iter().map(|r| r.d_speed).sum();

    // Calculate averages
    let avg_size = total_size / count;
    let avg_rate = total_rate / count;
    let avg_c_speed = total_c_speed / count;
    let avg_d_speed = total_d_speed / count;

    CompressionResult {
        size: avg_size,
        rate: avg_rate,
        c_speed: avg_c_speed,
        d_speed: avg_d_speed
    }
}

fn print_results(results: &[CompressionResult], compressor_name: &str) {
    println!("\n{}", compressor_name);
    for result in results.iter() {
        println!("size: {:.2}, rate: {:.3}, comp speed: {:.2}, decomp speed: {:.2}", result.size, result.rate, result.c_speed, result.d_speed);
    }
    let avg = get_average(&results);
    println!("AVERAGE. size: {:.2}, rate: {:.3}, comp speed: {:.2}, decomp speed: {:.2}", avg.size, avg.rate, avg.c_speed, avg.d_speed);
}

fn compress_lz4(path: &PathBuf, mode: CompressionMode) -> CompressionResult {
    // Load dataset
    let dataset = Dataset::load(path);
    let (_, data, _, _) = process_dataset(&dataset);
    let data_size = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();

    {
        unsafe{
            compression_buffer.set_len(data.len());
        }
        let compressed_size = lz4::block::compress_to_buffer(&data, Some(mode), false, &mut compression_buffer).unwrap();
        unsafe{
            compression_buffer.set_len(compressed_size);
        }
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();

    {
        unsafe{
            decompression_buffer.set_len(data.len());
        }
        let _ = lz4::block::decompress_to_buffer(&compression_buffer, Some(data.len() as i32), &mut decompression_buffer);
    }

    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    CompressionResult {
        size: compression_buffer.len() as f64 / (1024.0 * 1024.0),
        rate: compression_rate,
        c_speed: compression_speed,
        d_speed: decompression_speed
    }
}

fn compress_zstd(path: &PathBuf, level: i32) -> CompressionResult {
    // Load dataset
    let dataset = Dataset::load(path);
    let (_, data, _, _) = process_dataset(&dataset);
    let data_size = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();
    
    {
        let mut encoder = zstd::stream::Encoder::new(&mut compression_buffer, level).unwrap();
        encoder.write_all(&data).unwrap();
        encoder.finish().unwrap();
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    
    {
        let mut decoder = zstd::stream::Decoder::new(&compression_buffer[..]).unwrap();
        decoder.read_to_end(&mut decompression_buffer).unwrap();
    }
    
    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    CompressionResult {
        size: compression_buffer.len() as f64 / (1024.0 * 1024.0),
        rate: compression_rate,
        c_speed: compression_speed,
        d_speed: decompression_speed
    }
}

fn compress_deflate(path: &PathBuf, level: u32) -> CompressionResult {
    // Load dataset
    let dataset = Dataset::load(path);
    let (_, data, _, _) = process_dataset(&dataset);
    let data_size = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();
    
    {
        let mut compressor = flate2::Compress::new(flate2::Compression::new(level), false);
        compressor.compress_vec(&data, &mut compression_buffer, flate2::FlushCompress::Finish).unwrap();
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    
    {
        let mut decompressor = flate2::Decompress::new(false);
        decompressor.decompress_vec(&compression_buffer, &mut decompression_buffer, flate2::FlushDecompress::Finish).unwrap();
    }

    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    CompressionResult {
        size: compression_buffer.len() as f64 / (1024.0 * 1024.0),
        rate: compression_rate,
        c_speed: compression_speed,
        d_speed: decompression_speed
    }
}


fn compress_xz(path: &PathBuf, level: u32) -> CompressionResult {
    // Load dataset
    let dataset = Dataset::load(path);
    let (_, data, _, _) = process_dataset(&dataset);
    let data_size = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();

    {
        let mut encoder = xz2::write::XzEncoder::new(&mut compression_buffer, level);
        encoder.write_all(&data).unwrap();
        encoder.finish().unwrap();
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    
    {
        let mut decoder = xz2::read::XzDecoder::new(&compression_buffer[..]);
        decoder.read_to_end(&mut decompression_buffer).unwrap();
    }

    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    CompressionResult {
        size: compression_buffer.len() as f64 / (1024.0 * 1024.0),
        rate: compression_rate,
        c_speed: compression_speed,
        d_speed: decompression_speed
    }
}

fn compress_brotli(path: &PathBuf, level: u32) -> CompressionResult {
    // Load dataset
    let dataset = Dataset::load(path);
    let (_, data, _, _) = process_dataset(&dataset);
    let data_size = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();

    {
        let mut encoder = brotli::CompressorWriter::new(&mut compression_buffer, data.len(), level, 22);
        encoder.write_all(&data).unwrap();
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    
    {
        let mut decoder = brotli::DecompressorWriter::new(&mut decompression_buffer, data.len() // buffer size hint
        );
        decoder.write_all(&compression_buffer).unwrap();
    }

    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    CompressionResult {
        size: compression_buffer.len() as f64 / (1024.0 * 1024.0),
        rate: compression_rate,
        c_speed: compression_speed,
        d_speed: decompression_speed
    }
}

fn compress_snappi(path: &PathBuf) -> CompressionResult {
    // Load dataset
    let dataset = Dataset::load(path);
    let (_, data, _, _) = process_dataset(&dataset);
    let data_size = data.len() as f64 / (1024.0 * 1024.0);

    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_compression = Instant::now();

    {
        unsafe { compression_buffer.set_len(snap::raw::max_compress_len(data.len())); }
        let mut encoder = snap::raw::Encoder::new();
        let compressed_bytes = encoder.compress(&data, &mut compression_buffer).expect("Snappy compression failed");
        unsafe { compression_buffer.set_len(compressed_bytes); }
    }

    let compression_time = start_compression.elapsed().as_secs_f64();
    let compression_rate = data.len() as f64 / compression_buffer.len() as f64;
    let compression_speed = data_size / compression_time;    

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let start_decompression = Instant::now();
    
    {
        let mut decoder = snap::raw::Decoder::new();
        unsafe { decompression_buffer.set_len(data.len()); }
        decoder.decompress(&compression_buffer, &mut decompression_buffer).expect("Snappy decompression failed");
    
    }

    let decompression_time = start_decompression.elapsed().as_secs_f64();
    let decompression_speed = data_size / decompression_time;  

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

    CompressionResult {
        size: compression_buffer.len() as f64 / (1024.0 * 1024.0),
        rate: compression_rate,
        c_speed: compression_speed,
        d_speed: decompression_speed
    }
}