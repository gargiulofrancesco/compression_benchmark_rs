use std::{env, fs, path::Path};

use compression_benchmark_rs::dataset::{process_dataset, Dataset};

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

    // Print the dataset statistics
    print_dataset_statistics(directory);
}

// Function to print the statistics (name, #rows, avg len, size) of all datasets in the specified directory
fn print_dataset_statistics(folder: &str) {
    // Load all datasets from the specified directory
    for entry in fs::read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // Check if the path is a file and has a .json extension
        if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
            let dataset_path = path.to_str().unwrap();
            
            // Load dataset and process it
            let dataset = Dataset::load(dataset_path);
            let (dataset_name, data, end_positions, _) = process_dataset(&dataset);
            
            println!("Dataset Name: {}, Uncompressed Size (MB): {:.2}, Rows: {}, Avg Entry Length (B): {:.2}", 
                dataset_name,
                data.len() as f64 / (1024.0 * 1024.0),
                end_positions.len(),
                data.len() as f64 / end_positions.len() as f64,
                
            );
        }
    } 
}