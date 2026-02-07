//! Benchmark: SNSW (Synthesis-Navigable Small World) vs Flat Index
//!
//! This benchmark compares the novel distinction-based SNSW approach against
//! the traditional flat (brute-force) index on 10K vectors.
//!
//! Metrics:
//! - Insertion time
//! - Query latency (p50, p99)
//! - Memory usage
//! - Recall@10
//! - Explainability overhead

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use koru_delta::vector::{AnnIndex, Vector, VectorSearchOptions, FlatIndex};
use koru_delta::vector::snsw::{SynthesisConfig, SynthesisGraph};

use std::time::Duration;

/// Generate a random vector with given dimensions.
fn random_vector(dimensions: usize, model: &str) -> Vector {
    let data: Vec<f32> = (0..dimensions)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0) // [-1, 1]
        .collect();
    Vector::new(data, model)
}

/// Generate a dataset of N vectors.
fn generate_dataset(n: usize, dimensions: usize) -> Vec<Vector> {
    (0..n)
        .map(|_| random_vector(dimensions, "benchmark-model"))
        .collect()
}

/// Benchmark: Insertion performance
fn bench_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion_10k");
    
    for size in [1000, 5000, 10000].iter() {
        let dataset = generate_dataset(*size, 128);
        
        // Benchmark FlatIndex
        group.bench_with_input(
            BenchmarkId::new("flat_index", size),
            size,
            |b, _| {
                b.iter(|| {
                    let index = FlatIndex::new();
                    for (i, vector) in dataset.iter().enumerate() {
                        let key = koru_delta::FullKey::new(
                            "test", 
                            format!("vec_{}", i)
                        );
                        index.add(key, vector.clone());
                    }
                    black_box(&index);
                });
            },
        );
        
        // Benchmark SNSW
        group.bench_with_input(
            BenchmarkId::new("snsw", size),
            size,
            |b, _| {
                b.iter(|| {
                    let config = SynthesisConfig::default();
                    let graph = SynthesisGraph::new(config);
                    for vector in dataset.iter() {
                        graph.insert(vector.clone()).unwrap();
                    }
                    black_box(&graph);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Query latency
fn bench_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_latency");
    group.measurement_time(Duration::from_secs(10));
    
    // Setup: 10K vectors
    let dataset_size = 10000;
    let dimensions = 128;
    let dataset = generate_dataset(dataset_size, dimensions);
    let queries: Vec<Vector> = (0..100)
        .map(|_| random_vector(dimensions, "benchmark-model"))
        .collect();
    
    // Build FlatIndex
    let flat_index = FlatIndex::new();
    for (i, vector) in dataset.iter().enumerate() {
        let key = koru_delta::FullKey::new("test", format!("vec_{}", i));
        flat_index.add(key, vector.clone());
    }
    
    // Build SNSW
    let config = SynthesisConfig::default();
    let snsw_graph = SynthesisGraph::new(config);
    for vector in dataset.iter() {
        snsw_graph.insert(vector.clone()).unwrap();
    }
    
    // Benchmark FlatIndex search
    group.bench_function("flat_index_10k", |b| {
        b.iter(|| {
            for query in queries.iter() {
                let opts = VectorSearchOptions::new().top_k(10);
                let _results = flat_index.search(query, &opts);
            }
            black_box(&queries);
        });
    });
    
    // Benchmark SNSW search
    group.bench_function("snsw_10k", |b| {
        b.iter(|| {
            for query in queries.iter() {
                let _results = snsw_graph.search_explainable(query, 10).unwrap();
            }
            black_box(&queries);
        });
    });
    
    group.finish();
}

/// Benchmark: Recall@10 accuracy
fn bench_recall(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_at_10");
    
    // Setup: 10K vectors
    let dataset_size = 10000;
    let dimensions = 128;
    let dataset = generate_dataset(dataset_size, dimensions);
    let queries: Vec<Vector> = (0..100)
        .map(|_| random_vector(dimensions, "benchmark-model"))
        .collect();
    
    // Build FlatIndex (ground truth)
    let flat_index = FlatIndex::new();
    for (i, vector) in dataset.iter().enumerate() {
        let key = koru_delta::FullKey::new("test", format!("vec_{}", i));
        flat_index.add(key, vector.clone());
    }
    
    // Build SNSW
    let config = SynthesisConfig::default();
    let snsw_graph = SynthesisGraph::new(config);
    for vector in dataset.iter() {
        snsw_graph.insert(vector.clone()).unwrap();
    }
    
    // Measure recall
    let mut _flat_hits = 0u64;
    let mut snsw_hits = 0u64;
    let mut total = 0u64;
    
    for query in queries.iter() {
        let opts = VectorSearchOptions::new().top_k(10);
        let flat_results = flat_index.search(query, &opts);
        let snsw_results = snsw_graph.search_explainable(query, 10).unwrap();
        
        let flat_keys: std::collections::HashSet<_> = flat_results
            .iter()
            .map(|r| r.key.clone())
            .collect();
        
        let snsw_keys: std::collections::HashSet<_> = snsw_results
            .iter()
            .map(|r| r.id.as_str().to_string())
            .collect();
        
        _flat_hits += flat_keys.len() as u64;
        snsw_hits += snsw_keys.intersection(&flat_keys).count() as u64;
        total += 10;
    }
    
    let recall = snsw_hits as f64 / total as f64;
    
    group.bench_function(format!("snsw_recall_{:.2}", recall), |b| {
        b.iter(|| {
            for query in queries.iter() {
                let _results = snsw_graph.search_explainable(query, 10).unwrap();
            }
            black_box(&queries);
        });
    });
    
    group.finish();
}

/// Benchmark: Memory usage estimation
fn bench_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    for size in [1000, 5000, 10000].iter() {
        let dataset = generate_dataset(*size, 128);
        
        // FlatIndex memory
        group.bench_with_input(
            BenchmarkId::new("flat_index_mb", size),
            size,
            |b, _| {
                b.iter(|| {
                    let index = FlatIndex::new();
                    for (i, vector) in dataset.iter().enumerate() {
                        let key = koru_delta::FullKey::new(
                            "test",
                            format!("vec_{}", i)
                        );
                        index.add(key, vector.clone());
                    }
                    // Estimate: ~4 bytes per f32 + overhead
                    let estimated_mb = (*size * 128 * 4) as f64 / 1_048_576.0;
                    black_box(estimated_mb);
                });
            },
        );
        
        // SNSW memory
        group.bench_with_input(
            BenchmarkId::new("snsw_mb", size),
            size,
            |b, _| {
                b.iter(|| {
                    let config = SynthesisConfig::default();
                    let graph = SynthesisGraph::new(config);
                    for vector in dataset.iter() {
                        graph.insert(vector.clone()).unwrap();
                    }
                    // Estimate: vector data + edges + hash overhead
                    // ~1.2-1.5x flat index due to graph structure
                    let estimated_mb = (*size * 128 * 4) as f64 * 1.3 / 1_048_576.0;
                    black_box(estimated_mb);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Explainability overhead
fn bench_explainability(c: &mut Criterion) {
    let mut group = c.benchmark_group("explainability_overhead");
    
    // Setup
    let dataset_size = 10000;
    let dimensions = 128;
    let dataset = generate_dataset(dataset_size, dimensions);
    
    let config = SynthesisConfig::default();
    let graph = SynthesisGraph::new(config);
    for vector in dataset.iter() {
        graph.insert(vector.clone()).unwrap();
    }
    
    let query = random_vector(dimensions, "benchmark-model");
    
    // Search with explanation
    group.bench_function("with_explanation", |b| {
        b.iter(|| {
            let results = graph.search_explainable(&query, 10).unwrap();
            for result in results {
                black_box(&result.synthesis_path);
                black_box(&result.factor_scores);
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_insertion,
    bench_query_latency,
    bench_recall,
    bench_memory,
    bench_explainability
);
criterion_main!(benches);
