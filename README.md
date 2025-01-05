# Compression Benchmark for Random Access

This benchmark evaluates the random access performance of various compression algorithms for sequences of strings.

## Contents

### How to Run

1. Clone the repository:
   ```bash
   git clone https://github.com/gargiulofrancesco/compression_benchmark_rs
   cd compression_benchmark_rs
   ```

2. Build the project:
   ```bash
   RUSTFLAGS="-C target-cpu=native" cargo build --release
   cd target/release
   ```

3. Execute the desired benchmark or test executable as described in the **Executables** section.

### Executables

1. **`benchmark_individual.rs`**
   - **Purpose**: Measures the performance of a single compression algorithm on a specified file.
   - **Usage**:
     ```bash
     ./benchmark_individual <algorithm> <input_file>
     ```
     - `<algorithm>`: The name of the compression algorithm to benchmark (e.g., `zstd`).
     - `<input_file>`: Path to the file you want to compress and analyze.

     Example:
     ```bash
     ./benchmark_individual zstd dataset.json
     ```
     This command benchmarks the `zstd` algorithm on the dataset file `dataset.json`.

2. **`benchmark_all.rs`**
   - **Purpose**: Compares the performance of multiple compression algorithms across datasets in a folder.
   - **Usage**:
     ```bash
     ./benchmark_all <input_directory>
     ```
     - `<input_directory>`: Path to the folder containing datasets for benchmarking.

     Example:
     ```bash
     ./benchmark_all ./datasets/
     ```
     This command benchmarks several compression algorithms on the dataset files contained in `./datasets/` and provides a performance comparison.

### Dataset Format

A dataset file is a JSON object with the following fields:

- `dataset_name` (`String`): The name or identifier for the dataset.
- `data` (`Vec<String>`): A list of data entries, where each entry is a string.
- `queries` (`Vec<usize>`): A list of indices (zero-based) referring to elements in the data field.

Example JSON:

```json
{
  "dataset_name": "Example Dataset",
  "data": ["entry1", "entry2", "entry3"],
  "queries": [0, 2]
}
```

In this example:

- The dataset is named `"Example Dataset"`.
- The `data` field contains three entries: `"entry1"`, `"entry2"`, and `"entry3"`.
- The `queries` field references the first (`entry1`) and third (`entry3`) data entries.