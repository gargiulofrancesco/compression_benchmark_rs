use compression_benchmark_rs::longest_prefix_matcher::lpm16::LongestPrefixMatcher;
use std::env;
use std::fs::{self, File};

const N_ITERATIONS: usize = 10;

fn deserialize(dataset_name: &str) -> (Vec<u8>, Vec<usize>, Vec<u8>, LongestPrefixMatcher<u16>) {
    let base_path = "/home/gargiulo/data/lpm_bench/";
    let dataset_folder = format!("{}/{}", base_path, dataset_name);

    let data_path = format!("{}/data.bin", dataset_folder);
    let end_positions_path = format!("{}/end_positions.bin", dataset_folder);
    let parse_lengths_path = format!("{}/parse_lengths.bin", dataset_folder);    
    let lpm_path = format!("{}/lpm.bin", dataset_folder);

    let data = fs::read(data_path).unwrap();
    let parse_lengths = fs::read(parse_lengths_path).unwrap();
    let lpm = bincode::deserialize_from(File::open(lpm_path).unwrap()).unwrap();
    
    let end_positions_temp = fs::read(end_positions_path).unwrap();
    let end_positions: Vec<usize> = end_positions_temp
        .chunks_exact(4)
        .map(|chunk| (u32::from_ne_bytes(chunk.try_into().unwrap())) as usize)
        .collect();

    (data, end_positions, parse_lengths, lpm)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <dataset_name>", args[0]);
        return;
    }

    let dataset_name = &args[1];
    // `parse_lengths[i]` is the correct length of the i-th `find_longest_match` funcion call
    let (data, end_positions, parse_lengths, lpm) = deserialize(dataset_name);

    // Benchmark parsing
    let mut useless: usize = 0;
    for _ in 0..N_ITERATIONS {
        let mut start: usize = 0;
        for &end in end_positions.iter() {
            if start == end {
                continue;
            }

            let mut pos: usize = start;
            while pos < end {
                let (id, length) = lpm.find_longest_match(&data[pos..end]).unwrap();
                pos += length;
                useless = useless.wrapping_add(id as usize);
            }

            start = end;
        }
    }

    if useless == 42 {
        println!("very unlikely to happen");
    }
}