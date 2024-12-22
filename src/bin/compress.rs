use compression_benchmark_rs::compressor::onpair::OnPairCompressor;
use compression_benchmark_rs::compressor::Compressor;
use std::{env, vec};
use std::time::Instant;
use std::{fs, path::Path};

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

    let mut total_size = 0;
    let mut total_compressed_size = 0;

    // Load all datasets from the specified directory
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() {
            let data = fs::read(&path).unwrap();
            let mut compressor = OnPairCompressor::new(data.len(), 1);
            let mut buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
            let data_bytes = data.len() as f64;
            println!("{}", path.file_name().unwrap().to_str().unwrap());
        
            // Compression
            let start_compression = Instant::now();
            compressor.compress(&data, &vec![data.len()]);  // Compress the dataset
            let compression_time = start_compression.elapsed().as_secs_f64();
            let compression_rate = data_bytes / compressor.space_used_bytes() as f64;
            let compression_speed = (data_bytes / (1024.0 * 1024.0)) / compression_time;    
        
            // Decompression
            buffer.clear();
            let start_decompression = Instant::now();
            compressor.decompress(&mut buffer);  // Decompress the dataset
            let decompression_time = start_decompression.elapsed().as_secs_f64();
            let decompression_speed = (data_bytes / (1024.0 * 1024.0)) / decompression_time;
            
            println!("size: {:.2} MB, c_size: {:.2} MB, c_rate: {:.3}, c_speed: {:.2} MB/s, dc_speed: {:.2} MB/s\n", 
                (data.len() as f64) / (1024.0 * 1024.0),
                (compressor.space_used_bytes() as f64) / (1024.0 * 1024.0),
                compression_rate,
                compression_speed,
                decompression_speed
            );

            total_size += data.len();
            total_compressed_size += compressor.space_used_bytes();
        }
    }

    println!("\nTotal size: {:.2} MB, c_size: {:.2} MB, c_rate: {:.3}", 
        (total_size as f64) / (1024.0 * 1024.0),
        (total_compressed_size as f64) / (1024.0 * 1024.0),
        (total_size as f64) / (total_compressed_size as f64),
    );
}
