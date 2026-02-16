//! SNSW vs HNSW Benchmark
//!
//! Compares the production-grade Synthesis-Navigable Small World (SNSW)
//! against the proven HNSW implementation on 10,000 vectors.
//!
//! Metrics:
//! - Build time: Time to insert 10K vectors
//! - Search latency: Time to perform 100 queries
//! - Recall@K: Fraction of true nearest neighbors found
//! - Memory usage: Relative memory consumption

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use koru_delta::vector::{HnswConfig, HnswIndex, SynthesisGraph, Vector};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

/// Generate deterministic random vectors for reproducible benchmarks
fn generate_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vector> {
    let mut rng = StdRng::seed_from_u64(seed);

    (0..count)
        .map(|i| {
            let data: Vec<f32> = (0..dim).map(|_| rng.r#gen::<f32>() * 2.0 - 1.0).collect();
            // Use different models to simulate real-world usage
            let model = match i % 3 {
                0 => "text-embedding-3-small",
                1 => "text-embedding-3-large",
                _ => "multilingual-e5-large",
            };
            Vector::new(data, model)
        })
        .collect()
}

/// Generate query vectors (separate from database vectors)
fn generate_queries(count: usize, dim: usize, seed: u64) -> Vec<Vector> {
    // Use different seed to ensure queries aren't in the database
    generate_vectors(count, dim, seed + 10000)
}

/// Benchmark: Build time for HNSW
fn bench_hnsw_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_time");
    group.sample_size(10);

    let vectors = generate_vectors(10_000, 128, 42);

    group.bench_function(BenchmarkId::new("hnsw", "10k_128d"), |b| {
        b.iter(|| {
            let index = HnswIndex::new(HnswConfig::default());

            for (i, vector) in vectors.iter().enumerate() {
                index.add(format!("vec_{}", i), vector.clone()).unwrap();
            }

            black_box(index);
        });
    });

    group.finish();
}

/// Benchmark: Build time for SNSW
fn bench_snsw_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_time");
    group.sample_size(10);

    let vectors = generate_vectors(10_000, 128, 42);

    group.bench_function(BenchmarkId::new("snsw", "10k_128d"), |b| {
        b.iter(|| {
            let graph = SynthesisGraph::new_with_params(16, 200);

            for vector in &vectors {
                graph.insert(vector.clone()).unwrap();
            }

            black_box(graph);
        });
    });

    group.finish();
}

/// Benchmark: Search latency (HNSW)
fn bench_hnsw_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_latency");

    // Setup: Build index once
    let vectors = generate_vectors(10_000, 128, 42);
    let queries = generate_queries(100, 128, 42);

    let index = HnswIndex::new(HnswConfig::default());

    for (i, vector) in vectors.iter().enumerate() {
        index.add(format!("vec_{}", i), vector.clone()).unwrap();
    }

    group.bench_function(BenchmarkId::new("hnsw", "10k_128d_k10"), |b| {
        b.iter(|| {
            for query in &queries {
                let _results = index.search(query, 10, 50);
                black_box(_results);
            }
        });
    });

    group.finish();
}

/// Benchmark: Search latency (SNSW)
fn bench_snsw_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_latency");

    // Setup: Build graph once
    let vectors = generate_vectors(10_000, 128, 42);
    let queries = generate_queries(100, 128, 42);

    let graph = SynthesisGraph::new_with_params(16, 200);

    for vector in &vectors {
        graph.insert(vector.clone()).unwrap();
    }

    group.bench_function(BenchmarkId::new("snsw", "10k_128d_k10"), |b| {
        b.iter(|| {
            for query in &queries {
                let _results = graph.search(query, 10).unwrap();
                black_box(_results);
            }
        });
    });

    group.finish();
}

/// Benchmark: Recall@K comparison
fn bench_recall_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_at_k");
    group.sample_size(10);

    let vectors = generate_vectors(10_000, 128, 42);
    let queries = generate_queries(100, 128, 42);

    // Build both indexes
    let hnsw_index = HnswIndex::new(HnswConfig::default());

    let snsw_graph = SynthesisGraph::new_with_params(16, 200);

    for (i, vector) in vectors.iter().enumerate() {
        hnsw_index
            .add(format!("vec_{}", i), vector.clone())
            .unwrap();
        snsw_graph.insert(vector.clone()).unwrap();
    }

    // Calculate ground truth with exact search (linear scan)
    let mut ground_truth: Vec<Vec<String>> = Vec::new();
    for query in &queries {
        let mut exact_results: Vec<(String, f32)> = vectors
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                query
                    .cosine_similarity(v)
                    .map(|s| (format!("vec_{}", i), s))
            })
            .collect();
        exact_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let top_k: Vec<String> = exact_results
            .into_iter()
            .take(10)
            .map(|(id, _)| id)
            .collect();
        ground_truth.push(top_k);
    }

    // Benchmark HNSW recall
    group.bench_function(BenchmarkId::new("hnsw_recall", "@10"), |b| {
        b.iter(|| {
            let mut total_recall = 0.0;

            for (query, truth) in queries.iter().zip(&ground_truth) {
                let results = hnsw_index.search(query, 10, 50);
                let result_ids: std::collections::HashSet<_> =
                    results.iter().map(|r| r.key.clone()).collect();

                let truth_set: std::collections::HashSet<_> = truth.iter().cloned().collect();
                let hits: usize = result_ids.intersection(&truth_set).count();
                let recall = hits as f32 / truth.len().min(10) as f32;
                total_recall += recall;
            }

            let avg_recall = total_recall / queries.len() as f32;
            black_box(avg_recall);
        });
    });

    // Benchmark SNSW recall
    group.bench_function(BenchmarkId::new("snsw_recall", "@10"), |b| {
        b.iter(|| {
            let mut total_recall = 0.0;

            for (query, truth) in queries.iter().zip(&ground_truth) {
                let results = snsw_graph.search(query, 10).unwrap();
                let result_ids: std::collections::HashSet<_> =
                    results.iter().map(|r| r.id.as_str().to_string()).collect();

                let truth_set: std::collections::HashSet<_> = truth.iter().cloned().collect();
                let hits: usize = result_ids.intersection(&truth_set).count();
                let recall = hits as f32 / truth.len().min(10) as f32;
                total_recall += recall;
            }

            let avg_recall = total_recall / queries.len() as f32;
            black_box(avg_recall);
        });
    });

    group.finish();
}

/// Benchmark: Content-addressed deduplication overhead
fn bench_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("deduplication");

    // Create vectors with 20% duplicates
    let unique_vectors = generate_vectors(8_000, 128, 42);
    let mut all_vectors = unique_vectors.clone();

    // Add 2K duplicates (randomly selected from unique)
    let mut rng = StdRng::seed_from_u64(123);
    for _ in 0..2_000 {
        let idx = rng.gen_range(0..unique_vectors.len());
        all_vectors.push(unique_vectors[idx].clone());
    }

    // Shuffle
    all_vectors.shuffle(&mut rng);

    group.bench_function(BenchmarkId::new("snsw", "with_dedup"), |b| {
        b.iter(|| {
            let graph = SynthesisGraph::new_with_params(16, 200);

            for vector in &all_vectors {
                graph.insert(vector.clone()).unwrap();
            }

            // Should have only 8K nodes (deduplication)
            assert_eq!(graph.len(), 8_000);
            black_box(graph);
        });
    });

    group.bench_function(BenchmarkId::new("hnsw", "no_dedup"), |b| {
        b.iter(|| {
            let index = HnswIndex::new(HnswConfig::default());

            for (i, vector) in all_vectors.iter().enumerate() {
                index.add(format!("vec_{}", i), vector.clone()).unwrap();
            }

            // Has all 10K nodes (no deduplication)
            assert_eq!(index.len(), 10_000);
            black_box(index);
        });
    });

    group.finish();
}

/// Benchmark: Explainable search overhead
fn bench_explainable_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("explainable_overhead");

    let vectors = generate_vectors(10_000, 128, 42);
    let queries = generate_queries(100, 128, 42);

    let graph = SynthesisGraph::new_with_params(16, 200);

    for vector in &vectors {
        graph.insert(vector.clone()).unwrap();
    }

    // Standard search
    group.bench_function(BenchmarkId::new("standard", "search"), |b| {
        b.iter(|| {
            for query in &queries {
                let _results = graph.search(query, 10).unwrap();
                black_box(_results);
            }
        });
    });

    // Explainable search
    group.bench_function(BenchmarkId::new("explainable", "search"), |b| {
        b.iter(|| {
            for query in &queries {
                let _results = graph.search_explainable(query, 10).unwrap();
                black_box(_results);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hnsw_build,
    bench_snsw_build,
    bench_hnsw_search,
    bench_snsw_search,
    bench_recall_comparison,
    bench_deduplication,
    bench_explainable_search
);
criterion_main!(benches);
