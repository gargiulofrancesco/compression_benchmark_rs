use compression_benchmark_rs::compressor::onpair::OnPairCompressor;
use compression_benchmark_rs::compressor::Compressor;
use xz2::stream::TELL_NO_CHECK;
use std::{env, vec};
use std::io::{Read, Write};
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

fn _compress_lz4(data: &[u8]) {
    // Compress
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut encoder = lz4::EncoderBuilder::new()
        .level(4)  // Compression level (1-9)
        .build(&mut compression_buffer)
        .unwrap();
    encoder.write_all(&data).unwrap();
    let _ = encoder.finish();

    // Decompress
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut decoder = lz4::Decoder::new(&compression_buffer[..]).unwrap();
    decoder.read_to_end(&mut decompression_buffer).unwrap();

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());

}

fn _compress_lz4hc(data: &[u8]) {
    // Compression
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    unsafe{
        compression_buffer.set_len(data.len());
    }
    let compressed_size = lz4::block::compress_to_buffer(&data, Some(lz4::block::CompressionMode::HIGHCOMPRESSION(9)), false, &mut compression_buffer).unwrap();
    unsafe{
        compression_buffer.set_len(compressed_size);
    }

    // Decompression
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);    
    unsafe{
        decompression_buffer.set_len(data.len());
    }
    let _ = lz4::block::decompress_to_buffer(&compression_buffer, Some(data.len() as i32), &mut decompression_buffer);

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());
}

fn _compress_snappy(data: &[u8]) {
    // Compression
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 2048);
    unsafe{
        compression_buffer.set_len(snap::raw::max_compress_len(data.len()));
    }

    let mut encoder = snap::raw::Encoder::new();
    let compressed_bytes = encoder.compress(&data, &mut compression_buffer).expect("Snappy compression failed");
    unsafe{
        compression_buffer.set_len(compressed_bytes);
    }

    // Decompression
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() as usize + 2048);
    let mut decoder = snap::raw::Decoder::new();
    unsafe{
        decompression_buffer.set_len(data.len());
    }
    decoder.decompress(&compression_buffer, &mut decompression_buffer).expect("Snappy decompression failed");

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());
}

fn _compress_zstd(data: &[u8]) {
    // Compression
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut encoder = zstd::stream::Encoder::new(&mut compression_buffer, 9).unwrap();
    encoder.write_all(&data).unwrap();
    encoder.finish().unwrap();
    compression_buffer.shrink_to_fit();

    // Decompression
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut decoder = zstd::stream::Decoder::new(&compression_buffer[..]).unwrap();
    decoder.read_to_end(&mut decompression_buffer).unwrap();
    decompression_buffer.shrink_to_fit();

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());
}

fn _compress_gzip(data: &[u8]) {
    // Compression
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut compressor = flate2::Compress::new(flate2::Compression::default(), false);
    compressor.compress_vec(data, &mut compression_buffer, flate2::FlushCompress::Finish).unwrap();
    let compressed_size = compressor.total_out();

    // Decompression
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut decompressor = flate2::Decompress::new(false);
    decompressor.decompress_vec(&compression_buffer, &mut decompression_buffer, flate2::FlushDecompress::Finish).unwrap();
    let decompressed_size = decompressor.total_out();

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());
}

fn _compress_xz(data: &[u8]) {
    // Compression
    let mut compression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut encoder = xz2::stream::Stream::new_easy_encoder(1, xz2::stream::Check::None).unwrap();
    encoder.process_vec(data, &mut compression_buffer, xz2::stream::Action::Finish).unwrap();

    // Decompression
    let mut decompression_buffer: Vec<u8> = Vec::with_capacity(data.len() + 1024);
    let mut decoder = xz2::stream::Stream::new_stream_decoder(data.len() as u64, TELL_NO_CHECK).unwrap();
    decoder.process_vec(&compression_buffer, &mut decompression_buffer, xz2::stream::Action::Finish).unwrap();

    // Verify the result
    assert_eq!(data, decompression_buffer.as_slice());
}