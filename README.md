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

3. Execute the benchmark:
   ```bash
   ./benchmark_all <input_directory>
   ```
   Replace `<input_directory>` with the path to the folder containing your dataset files.

   This command runs the benchmark defined in `benchmark_all.rs`, compressing and querying the dataset files in `<input_directory>`, and outputs a performance comparison of the implemented algorithms.

### Dataset Format

Each dataset file should be a JSON object representing a flat list of strings:

```json
[
   "entry1",
   "entry2",
   "entry3"
]
```
