//! LCA Architecture Performance Benchmarks
//!
//! Phase 4.3: Benchmark LCA operations to ensure no performance regression
//!
//! Success Criteria:
//! - LCA operations maintain >95% of original speed
//! - Memory usage is comparable or better
//! - Concurrent operations scale well

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use koru_delta::KoruDelta;
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark: Synthesis operation (the core LCA primitive)
fn bench_synthesis_operation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(async { KoruDelta::start().await.unwrap() });

    c.bench_function("lca_synthesis_put", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            black_box(
                db.put(
                    "bench",
                    "synthesis_key",
                    json!({"data": "test_value", "timestamp": 1234567890}),
                )
                .await
                .unwrap(),
            )
        })
    });
}

/// Benchmark: Sequential synthesis operations with varying batch sizes
fn bench_sequential_synthesis(c: &mut Criterion) {
    let mut group = c.benchmark_group("lca_sequential_synthesis");

    for size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(Runtime::new().unwrap()).iter(|| async {
                let db = KoruDelta::start().await.unwrap();
                for i in 0..size {
                    db.put(
                        "bench",
                        &format!("key{}", i),
                        json!({
                            "id": i,
                            "data": format!("synthesis_batch_{}", i)
                        }),
                    )
                    .await
                    .unwrap();
                }
            });
        });
    }
    group.finish();
}

/// Benchmark: Content addressing overhead (same content = same distinction)
fn bench_content_addressing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(async { KoruDelta::start().await.unwrap() });

    // Pre-populate with data
    let data = json!({
        "complex": {
            "nested": {
                "structure": [1, 2, 3, 4, 5],
                "metadata": {
                    "created": "2024-01-01",
                    "tags": ["test", "benchmark", "lca"]
                }
            }
        }
    });

    c.bench_function("lca_content_addressing", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            // Storing same content should be fast due to content addressing
            black_box(
                db.put("content_bench", "same_key", data.clone())
                    .await
                    .unwrap(),
            )
        })
    });
}

/// Benchmark: Memory tier synthesis (Hot → Warm → Cold)
fn bench_memory_tier_synthesis(c: &mut Criterion) {
    let mut group = c.benchmark_group("lca_memory_tier_synthesis");

    let rt = Runtime::new().unwrap();
    let db = rt.block_on(async { KoruDelta::start().await.unwrap() });

    // Populate hot memory
    rt.block_on(async {
        for i in 0..100 {
            db.put("tier_bench", &format!("key{}", i), json!({"index": i}))
                .await
                .unwrap();
        }
    });

    group.bench_function("hot_read_synthesis", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            // Reading from hot memory synthesizes an access action
            black_box(db.get("tier_bench", "key50").await.unwrap())
        })
    });

    group.finish();
}

/// Benchmark: Concurrent synthesis operations
fn bench_concurrent_synthesis(c: &mut Criterion) {
    let mut group = c.benchmark_group("lca_concurrent_synthesis");

    for num_tasks in [4, 8, 16] {
        group.throughput(Throughput::Elements(num_tasks as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let db = KoruDelta::start().await.unwrap();
                    let mut handles = vec![];

                    for i in 0..num_tasks {
                        let db_clone = db.clone();
                        handles.push(tokio::spawn(async move {
                            db_clone
                                .put(
                                    "concurrent_bench",
                                    &format!("task{}", i),
                                    json!({"task_id": i}),
                                )
                                .await
                                .unwrap();
                        }));
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Root advancement (local_root progression)
fn bench_root_advancement(c: &mut Criterion) {
    let mut group = c.benchmark_group("lca_root_advancement");

    for chain_length in [10, 100, 1000] {
        group.throughput(Throughput::Elements(chain_length as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(chain_length),
            &chain_length,
            |b, &chain_length| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let db = KoruDelta::start().await.unwrap();

                    // Create a synthesis chain: each operation advances local_root
                    for i in 0..chain_length {
                        db.put("chain_bench", "chain_key", json!({"iteration": i}))
                            .await
                            .unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: History synthesis (version tracking)
fn bench_history_synthesis(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(async {
        let db = KoruDelta::start().await.unwrap();
        // Create version history
        for i in 0..100 {
            db.put("history_bench", "versioned_key", json!({"version": i}))
                .await
                .unwrap();
        }
        db
    });

    c.bench_function("lca_history_synthesis", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            // Retrieving history synthesizes chronicle actions
            black_box(db.history("history_bench", "versioned_key").await.unwrap())
        })
    });
}

// Configure Criterion for faster benchmarks while maintaining accuracy
fn configure_criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(50)
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = bench_synthesis_operation,
        bench_sequential_synthesis,
        bench_content_addressing,
        bench_memory_tier_synthesis,
        bench_concurrent_synthesis,
        bench_root_advancement,
        bench_history_synthesis
}

criterion_main!(benches);
