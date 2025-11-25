# KoruDelta Performance Benchmarks

Comprehensive benchmarks for KoruDelta operations using [Criterion.rs](https://github.com/bheisler/criterion.rs).

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench core_operations

# Run specific test within benchmark
cargo bench -- database_init
cargo bench -- put_sequential
```

## Benchmark Results Summary

Based on the latest benchmark run:

### Core Operations
- **Database Init**: ~2.0 µs (very fast startup)
- **Single Put**: ~45 µs per operation
- **Single Get**: ~370 ns per operation (2.7M ops/sec)

### Throughput
- **Sequential Puts (10)**: 31K ops/sec
- **Sequential Puts (100)**: 28K ops/sec
- **Sequential Puts (1000)**: 22K ops/sec

### History Operations
- **History (10 versions)**: 4.0M elements/sec
- **History (50 versions)**: 4.3M elements/sec
- **History (100 versions)**: 4.3M elements/sec

### Scalability
- **Get from 100-key dataset**: 2.1M ops/sec
- **Get from 1K-key dataset**: 2.1M ops/sec
- **Get from 10K-key dataset**: 2.1M ops/sec

*Note: Get operations maintain constant O(1) performance regardless of dataset size*

### List Operations
- **List namespaces (10 keys)**: ~2.1 µs
- **List namespaces (1000 keys)**: ~80 µs
- **List keys (1000 keys)**: ~17 µs

### Stats Computation
- **100 keys**: 11.2M elements/sec
- **1000 keys**: 17.8M elements/sec
- **5000 keys**: 17.9M elements/sec

## Benchmark Configuration

The benchmarks use optimized Criterion settings for faster runs while maintaining accuracy:
- **Warm-up time**: 1 second (vs default 3s)
- **Measurement time**: 3 seconds (vs default 5s)
- **Sample size**: 50 (vs default 100)

This provides ~4x speedup while still giving reliable statistical measurements.

## Interpreting Results

- **time**: Average time per operation
- **thrpt**: Throughput (operations per second or elements per second)
- **Outliers**: Measurements that deviate from the normal distribution (expected in concurrent systems)

## Adding New Benchmarks

1. Add your benchmark function to `benches/core_operations.rs`
2. Register it in the `criterion_group!` macro
3. Run `cargo bench` to establish baseline
4. Future runs will compare against this baseline

## Performance Tips

For optimal performance in your applications:

1. **Reuse database instances** - Init is ~2µs but avoiding it is better
2. **Batch operations when possible** - Though individual ops are fast
3. **Use get() for simple reads** - It's extremely fast at ~370ns
4. **History is efficient** - Even with 100s of versions
5. **List operations scale well** - O(n) in number of keys, but fast
