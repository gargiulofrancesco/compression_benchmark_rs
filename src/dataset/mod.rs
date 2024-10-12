use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fs;
use std::path::Path;

/// Represents a single test case containing both data and queries.
#[derive(Serialize, Deserialize, Debug)]
pub struct Dataset {
    pub dataset_name: String,
    pub data: Vec<String>,
    pub queries: Vec<usize>,
}

impl Dataset {
    /// Loads a test case from a JSON file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let mut dataset: Dataset = serde_json::from_str(&content)?;

        dataset.dataset_name.shrink_to_fit();
        dataset.data.shrink_to_fit();
        dataset.queries.shrink_to_fit();
        
        Ok(dataset)
    }
}

/// Loads all datasets from the specified directory.
pub fn load_datasets<P: AsRef<Path>>(dir: P) -> Result<Vec<Dataset>, Box<dyn Error>> {
    let mut datasets = Vec::new();
    
    // Read the directory
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            // Load the dataset from the JSON file
            let dataset = Dataset::load(&path)?;
            datasets.push(dataset);
        }
    }

    Ok(datasets)
}