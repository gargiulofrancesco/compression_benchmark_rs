use prettytable::{row, Table};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct BenchmarkResult {
    pub dataset_name: String,
    pub compressor_name: String,
    pub compression_rate: f64,
    pub compression_speed: f64,
    pub decompression_speed: f64,
    pub random_access_speed: f64,
    pub average_random_access_time: f64,
}

/// Represents a single dataset.
#[derive(Serialize, Deserialize, Debug)]
pub struct Dataset {
    pub dataset_name: String,
    pub data: Vec<String>,
    pub queries: Vec<usize>,
}

impl Dataset {
    /// Loads a dataset from a JSON file.
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let content = fs::read_to_string(path).unwrap();
        let mut dataset: Dataset = serde_json::from_str(&content).unwrap();

        dataset.dataset_name.shrink_to_fit();
        dataset.data.shrink_to_fit();
        
        dataset
    }
}

/// Loads all datasets from the specified directory.
pub fn load_datasets<P: AsRef<Path>>(dir: P) -> Vec<Dataset> {
    let mut datasets = Vec::new();
    
    // Read the directory
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            // Load the dataset from the JSON file
            let dataset = Dataset::load(&path);
            datasets.push(dataset);
        }
    }

    datasets
}

/// Processes a dataset into a format that can be used by the compressors.
pub fn process_dataset(dataset: &Dataset) -> (String, Vec<u8>, Vec<usize>, Vec<usize>) {
    let dataset_name = dataset.dataset_name.clone();
    let data: Vec<u8> = dataset.data.iter().flat_map(|s| s.as_bytes()).copied().collect();
    let queries: Vec<usize> = dataset.queries.clone();
    let end_positions: Vec<usize> = dataset.data.iter()
        .scan(0, |state, s| {
            *state += s.len();
            Some(*state)
        })
        .collect();

    (dataset_name, data, end_positions, queries)
}

pub fn read_benchmark_results(file_path: &str) -> Vec<BenchmarkResult> {
    if Path::new(file_path).exists() {
        let file_content = fs::read_to_string(file_path).expect("Failed to read file");
        serde_json::from_str::<Vec<BenchmarkResult>>(&file_content).unwrap_or_else(|_| {
            eprintln!("Error parsing results file '{}'. Starting fresh.", file_path);
            Vec::new()
        })
    } else {
        Vec::new()
    }
}

pub fn append_benchmark_result(result: &BenchmarkResult, file_path: &Path) {
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

pub fn print_benchmark_results(results: &[BenchmarkResult]) {
    // Group results by compressor and dataset name
    let mut grouped_results: HashMap<(String, String), Vec<&BenchmarkResult>> = HashMap::new();
    for result in results {
        grouped_results
            .entry((result.compressor_name.clone(), result.dataset_name.clone()))
            .or_default()
            .push(result);
    }

    // A map to store results grouped by compressor name
    let mut compressor_groups: HashMap<String, Vec<BenchmarkResult>> = HashMap::new();

    // Calculate averaged results for each (compressor, dataset) pair
    for ((compressor, dataset), group) in grouped_results {
        let len = group.len() as f64;
        let avg_compression_rate = group.iter().map(|r| r.compression_rate).sum::<f64>() / len;
        let avg_compression_speed = group.iter().map(|r| r.compression_speed).sum::<f64>() / len;
        let avg_decompression_speed = group.iter().map(|r| r.decompression_speed).sum::<f64>() / len;
        let avg_random_access_speed = group.iter().map(|r| r.random_access_speed).sum::<f64>() / len;
        let avg_average_random_access_time = group.iter().map(|r| r.average_random_access_time).sum::<f64>() / len;

        // Store the averaged result
        let averaged_result = BenchmarkResult {
            dataset_name: dataset,
            compressor_name: compressor.clone(),
            compression_rate: avg_compression_rate,
            compression_speed: avg_compression_speed,
            decompression_speed: avg_decompression_speed,
            random_access_speed: avg_random_access_speed,
            average_random_access_time: avg_average_random_access_time,
        };

        compressor_groups
            .entry(compressor)
            .or_default()
            .push(averaged_result);
    }

    // Print results grouped by compressor
    for (compressor, results) in compressor_groups {
        let mut sorted_results = results;
        // Sort results by dataset name
        sorted_results.sort_by(|a, b| a.dataset_name.cmp(&b.dataset_name));

        // Create a new table for each compressor
        let mut table = Table::new();
        table.add_row(row![
            "Dataset",
            "Comp Rate",
            "Comp Speed (MB/s)",
            "Decomp Speed (MB/s)",
            "Random Access Speed (MB/s)",
            "Avg Random Access Time (ns)"
        ]);

        // Add rows for each averaged result
        for result in &sorted_results {
            table.add_row(row![
                &result.dataset_name,
                format!("{:.3}", result.compression_rate),
                format!("{:.2}", result.compression_speed),
                format!("{:.2}", result.decompression_speed),
                format!("{:.2}", result.random_access_speed),
                format!("{}", (result.average_random_access_time * 1_000_000_000.0).round() as u64),
            ]);
        }

        // Calculate overall averages for this compressor
        let len = sorted_results.len() as f64;
        let overall_avg_compression_rate =
            sorted_results.iter().map(|r| r.compression_rate).sum::<f64>() / len;
        let overall_avg_compression_speed =
            sorted_results.iter().map(|r| r.compression_speed).sum::<f64>() / len;
        let overall_avg_decompression_speed =
            sorted_results.iter().map(|r| r.decompression_speed).sum::<f64>() / len;
        let overall_avg_random_access_speed =
            sorted_results.iter().map(|r| r.random_access_speed).sum::<f64>() / len;
        let overall_avg_random_access_time =
            sorted_results.iter().map(|r| r.average_random_access_time).sum::<f64>() / len;

        // Add overall averages row
        table.add_row(row![
            "AVERAGE",
            format!("{:.3}", overall_avg_compression_rate),
            format!("{:.2}", overall_avg_compression_speed),
            format!("{:.2}", overall_avg_decompression_speed),
            format!("{:.2}", overall_avg_random_access_speed),
            format!("{}", (overall_avg_random_access_time * 1_000_000_000.0).round() as u64),
        ]);

        // Print the table for this compressor
        println!("\nResults for Compressor: {}", compressor);
        table.printstd();
    }
}