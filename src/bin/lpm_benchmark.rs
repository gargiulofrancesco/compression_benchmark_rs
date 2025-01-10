use compression_benchmark_rs::longest_prefix_matcher::lpm16::{
    LongestPrefixMatcher, StaticLongestPrefixMatcher,
};
use std::env;
use std::fs::{self, File};
use std::time::Instant;

const N_ITERATIONS: usize = 5;

fn deserialize(dataset_name: &str) -> (Vec<u8>, Vec<usize>, Vec<u8>, LongestPrefixMatcher<u16>) {
    let base_path = "/home/rossano/data/lpm_bench";
    let dataset_folder = format!("{}/{}", base_path, dataset_name);

    let data_path = format!("{}/data.bin", dataset_folder);
    println!("{}", data_path);
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
    let start = Instant::now();
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <dataset_name>", args[0]);
        return;
    }

    let dataset_name = &args[1];
    // `parse_lengths[i]` is the correct length of the i-th `find_longest_match` funcion call
    let (data, end_positions, parse_lengths, lpm) = deserialize(dataset_name);

    let file_size = parse_lengths
        .iter()
        .map(|length| *length as usize)
        .sum::<usize>();
    println!(
        "File size: {} {}",
        file_size,
        *end_positions.last().unwrap()
    );

    let lpm = StaticLongestPrefixMatcher::from(lpm);

    println!("Please skip {:?}", start.elapsed());

    let start = Instant::now();
    // Benchmark parsing
    let mut useless: usize = 0;
    for _ in 0..N_ITERATIONS {
        let mut start: usize = 0;
        let mut i = 0;
        for &end in end_positions.iter() {
            if start == end {
                continue;
            }

            let mut pos: usize = start;
            while pos < end {
                let true_length = parse_lengths[i];
                let (id, _length) = lpm.find_longest_match(&data[pos..end]).unwrap();
                // assert_eq!(_length, true_length as usize);
                pos += true_length as usize;
                useless = useless.wrapping_add(id as usize);
                i += 1;
            }

            start = end;
        }
    }

    let time = start.elapsed().as_secs_f64() / N_ITERATIONS as f64;

    println!("Time to parse per iteration: {:.2}", time);

    println!(
        "Throughput: {:.2} MB/s",
        file_size as f64 / time / 1024.0 / 1024.0
    );

    if useless == 42 {
        println!("very unlikely to happen");
    }
}
