# OnPair Compression Benchmark Suite

[![Paper](https://img.shields.io/badge/Paper-arXiv:2508.02280-blue)](https://arxiv.org/abs/2508.02280)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

This repository contains the Rust **experimental evaluation framework** for the paper:

> **[OnPair: Short Strings Compression for Fast Random Access](https://arxiv.org/abs/2508.02280)**  

## Overview

**OnPair** is a compression algorithm specifically designed for workloads requiring fast random access to individual strings in large collections. This benchmark suite provides comprehensive performance evaluation tools to compare OnPair against established compression methods.

For the **standalone OnPair algorithm implementation**, see: **[onpair_rs](https://github.com/gargiulofrancesco/onpair_rs)** 


## Quick Start

### Installation & Building

```bash
# Clone the repository
git clone https://github.com/gargiulofrancesco/compression_benchmark_rs
cd compression_benchmark_rs

# Build with native optimizations for maximum performance
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Running Benchmarks

#### Single Algorithm Evaluation
Evaluate a specific algorithm on a dataset:

```bash
./target/release/benchmark_individual <dataset.json> <algorithm> <output.json> [core_id]
```

**Example:**
```bash
# Run onpair16 algorithm on example dataset with CPU core pinning
./target/release/benchmark_individual data/example.json onpair16 results.json 0
```

#### Comprehensive Benchmark Suite
Run all algorithms on all datasets in a directory:

```bash
./target/release/benchmark_all <dataset_directory> [core_id]
```

**Example:**
```bash
# Run complete benchmark suite with CPU core pinning
./target/release/benchmark_all data/ 0
```

This generates a comprehensive performance comparison across all algorithms and datasets.

## Supported Algorithms

| Algorithm | Description |
|-----------|-------------|
| `raw` | Uncompressed baseline |
| `bpe` | Byte Pair Encoding |
| `onpair` | OnPair (unlimited tokens) |
| `onpair_bv` | OnPair with bit vector |
| `onpair16` | OnPair (16-byte limit) |

## Dataset Format

Datasets must be JSON arrays of strings:

```json
[
   "user_12345",
   "admin_67890", 
   "guest_11111",
   "user_54321"
]
```

## Performance Metrics

The benchmark suite evaluates algorithms across four key dimensions:

| Metric | Description | Units |
|--------|-------------|-------|
| **Compression Ratio** | `original_size / compressed_size` | Ratio |
| **Compression Speed** | Throughput during compression | MiB/s |
| **Decompression Speed** | Throughput during full decompression | MiB/s |
| **Random Access Time** | Average time per individual string access | nanoseconds |

**Output Format:** Results are exported as structured JSON for easy analysis and visualization.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Authors

- **Francesco Gargiulo** - [francesco.gargiulo@phd.unipi.it](mailto:francesco.gargiulo@phd.unipi.it)
- **Rossano Venturini** - [rossano.venturini@unipi.it](mailto:rossano.venturini@unipi.it)

*University of Pisa, Italy*

