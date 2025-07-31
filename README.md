# OnPair: Short Strings Compression for Fast Random Access

This repository contains the Rust implementation and benchmark suite for **OnPair**, a compression algorithm designed for efficient random access on sequences of short strings.

## Overview

OnPair is a field-level compression algorithm designed for workloads requiring fast random access to individual strings in large collections. The compression process consists of two distinct phases:

- **Training Phase**: A longest prefix matching strategy is used to parse the input and identify frequent adjacent token pairs. When the frequency of a pair exceeds a predefined threshold, a new token is created to represent the merged pair. This continues until the dictionary is full or the input data is exhausted. The dictionary supports up to 65,536 tokens, with each token assigned a fixed 2-byte ID.
- **Parsing Phase**: Once the dictionary is constructed, each string is compressed independently into a sequence of token IDs by greedily applying longest prefix matching

OnPair16 is a variant that limits dictionary entries to a maximum length of 16 bytes. This constraint enables further optimizations in both longest prefix matching and decoding.

## Quick Start

### Installation

```bash
git clone https://github.com/gargiulofrancesco/compression_benchmark_rs
cd compression_benchmark_rs
```

### Building

```bash
# Build with native CPU optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Running Benchmarks

#### Single Algorithm Benchmark
```bash
./target/release/benchmark_individual <dataset.json> <algorithm> <output.json> [core_id]
```

**Example:**
```bash
./target/release/benchmark_individual data/example.json onpair16 results.json
```

#### Complete Benchmark Suite
```bash
./target/release/benchmark_all <dataset_directory> [core_id]
```

**Examples:**
```bash
# Run on all datasets with core pinning to CPU core 0
./target/release/benchmark_all data/ 0
```

This runs all algorithms on all JSON datasets in the directory and produces a comprehensive comparison.

### Supported Algorithms
- `raw` - Uncompressed baseline
- `bpe` - Byte Pair Encoding
- `onpair` - OnPair algorithm
- `onpair_bv` - OnPair with bit vectors
- `onpair16` - OnPair with 16-byte limits  

## Dataset Format

Datasets should be JSON arrays of strings:

```json
[
   "first string",
   "second string", 
   "third string"
]
```

## Benchmark Metrics

The benchmark suite measures:

- **Compression Ratio**: `original_size / compressed_size`
- **Compression Speed**: MiB/s during compression
- **Decompression Speed**: MiB/s during full decompression  
- **Random Access Time**: Average nanoseconds per string access

Results are output in JSON format for easy analysis and visualization.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contact

- **Francesco Gargiulo**: [francesco.gargiulo@phd.unipi.it]
- **Rossano Venturini**: [rossano.venturini@unipi.it]
