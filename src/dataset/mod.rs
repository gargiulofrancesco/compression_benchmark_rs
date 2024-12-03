use serde::{Serialize, Deserialize};
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
