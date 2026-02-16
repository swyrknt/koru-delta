use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use koru_delta::KoruDelta;
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark: Database initialization
fn bench_database_init(c: &mut Criterion) {
    c.bench_function("database_init", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| async { black_box(KoruDelta::start().await.unwrap()) })
    });
}

/// Benchmark: Single put operation
fn bench_put_single(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(async { KoruDelta::start().await.unwrap() });

    c.bench_function("put_single", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            black_box(
                db.put(
                    "bench",
                    "key1",
                    json!({
                        "name": "Alice",
                        "age": 30,
                        "email": "alice@example.com"
                    }),
                )
                .await
                .unwrap(),
            )
        })
    });
}

/// Benchmark: Sequential puts to different keys
fn bench_put_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("put_sequential");

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
                            "value": format!("data{}", i)
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

/// Benchmark: Get operation after single put
fn bench_get_single(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(async {
        let db = KoruDelta::start().await.unwrap();
        db.put("bench", "key1", json!({"data": "test"}))
            .await
            .unwrap();
        db
    });

    c.bench_function("get_single", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| async { black_box(db.get("bench", "key1").await.unwrap()) })
    });
}

/// Benchmark: Get operations with varying dataset sizes
fn bench_get_from_dataset(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_from_dataset");

    for dataset_size in [100, 1000, 10000] {
        let rt = Runtime::new().unwrap();
        let db = rt.block_on(async {
            let db = KoruDelta::start().await.unwrap();
            // Populate dataset
            for i in 0..dataset_size {
                db.put(
                    "bench",
                    &format!("key{}", i),
                    json!({
                        "id": i,
                        "value": format!("data{}", i)
                    }),
                )
                .await
                .unwrap();
            }
            db
        });

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(dataset_size),
            &dataset_size,
            |b, &size| {
                let middle_key = format!("key{}", size / 2);
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    // Get a key from the middle of the dataset
                    black_box(db.get("bench", &middle_key).await.unwrap())
                })
            },
        );
    }
    group.finish();
}

/// Benchmark: History retrieval with varying version counts
fn bench_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("history");

    for version_count in [10, 50, 100] {
        let rt = Runtime::new().unwrap();
        let db = rt.block_on(async {
            let db = KoruDelta::start().await.unwrap();
            // Create multiple versions of the same key
            for i in 0..version_count {
                db.put(
                    "bench",
                    "versioned_key",
                    json!({
                        "version": i,
                        "data": format!("value{}", i)
                    }),
                )
                .await
                .unwrap();
            }
            db
        });

        group.throughput(Throughput::Elements(version_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(version_count),
            &version_count,
            |b, &_count| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    black_box(db.history("bench", "versioned_key").await.unwrap())
                })
            },
        );
    }
    group.finish();
}

/// Benchmark: Versioning - multiple updates to same key
fn bench_versioning(c: &mut Criterion) {
    let mut group = c.benchmark_group("versioning");

    for update_count in [10, 50, 100] {
        group.throughput(Throughput::Elements(update_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(update_count),
            &update_count,
            |b, &count| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let db = KoruDelta::start().await.unwrap();
                    for i in 0..count {
                        db.put(
                            "bench",
                            "versioned",
                            json!({
                                "counter": i,
                                "timestamp": format!("t{}", i)
                            }),
                        )
                        .await
                        .unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: List operations with varying namespace/key counts
fn bench_list_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_operations");

    for key_count in [10, 100, 1000] {
        let rt = Runtime::new().unwrap();
        let db = rt.block_on(async {
            let db = KoruDelta::start().await.unwrap();
            // Create multiple namespaces with keys
            for i in 0..key_count {
                db.put(
                    &format!("namespace{}", i % 10), // 10 namespaces
                    &format!("key{}", i),
                    json!({"id": i}),
                )
                .await
                .unwrap();
            }
            db
        });

        group.bench_function(BenchmarkId::new("list_namespaces", key_count), |b| {
            b.to_async(Runtime::new().unwrap())
                .iter(|| async { black_box(db.list_namespaces().await) })
        });

        group.bench_function(BenchmarkId::new("list_keys", key_count), |b| {
            b.to_async(Runtime::new().unwrap())
                .iter(|| async { black_box(db.list_keys("namespace0").await) })
        });
    }
    group.finish();
}

/// Benchmark: Stats computation with varying dataset sizes
fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats");

    for key_count in [100, 1000, 5000] {
        let rt = Runtime::new().unwrap();
        let db = rt.block_on(async {
            let db = KoruDelta::start().await.unwrap();
            for i in 0..key_count {
                db.put("bench", &format!("key{}", i), json!({"id": i}))
                    .await
                    .unwrap();
            }
            db
        });

        group.throughput(Throughput::Elements(key_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(key_count),
            &key_count,
            |b, &_count| {
                b.to_async(Runtime::new().unwrap())
                    .iter(|| async { black_box(db.stats().await) })
            },
        );
    }
    group.finish();
}

// Configure Criterion for faster benchmarks while maintaining accuracy
// - Reduced warm-up time: 1s (vs default 3s)
// - Reduced measurement time: 3s (vs default 5s)
// - Sample size: 50 (vs default 100)
// This gives ~4x speedup while still providing reliable measurements
fn configure_criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(50)
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = bench_database_init,
        bench_put_single,
        bench_put_sequential,
        bench_get_single,
        bench_get_from_dataset,
        bench_history,
        bench_versioning,
        bench_list_operations,
        bench_stats
}

criterion_main!(benches);
