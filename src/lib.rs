//! Compression benchmark suite for string collections with random access
//!
//! This crate provides an evaluation framework for compression algorithms
//! optimized for string collections requiring efficient random access. The benchmark
//! suite measures compression ratio, throughput, and random access latency across
//! datasets to enable systematic algorithm comparison.

pub mod benchmark_utils;
pub mod compressor;
pub mod bit_vector;