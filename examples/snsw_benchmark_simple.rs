//! Simple benchmark: SNSW vs Flat Index
//!
//! Run with: cargo run --example snsw_benchmark_simple --release

use koru_delta::vector::{AnnIndex, FlatIndex, Vector, VectorSearchOptions};
use koru_delta::vector::snsw::{SynthesisConfig, SynthesisGraph};
use std::time::Instant;

fn random_vector(dimensions: usize) -> Vector {
    let data: Vec<f32> = (0..dimensions)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    Vector::new(data, "bench-model")
}

fn generate_dataset(n: usize, dimensions: usize) -> Vec<Vector> {
    (0..n).map(|_| random_vector(dimensions)).collect()
}

fn main() {
    println!("{}", "=".repeat(70));
    println!("SNSW vs Flat Index - Performance Benchmark");
    println!("{}", "=".repeat(70));
    
    let dimensions = 128;
    let sizes = vec![100, 500, 1000, 5000];
    
    for size in sizes {
        println!("\n--- Dataset: {} vectors ({}D) ---", size, dimensions);
        
        let dataset = generate_dataset(size, dimensions);
        let queries: Vec<Vector> = (0..100).map(|_| random_vector(dimensions)).collect();
        
        // Flat Index Benchmark
        print!("Flat Index: building... ");
        let flat_start = Instant::now();
        let flat_index = FlatIndex::new();
        for (i, vector) in dataset.iter().enumerate() {
            let key = koru_delta::FullKey::new("test", format!("vec_{}", i));
            flat_index.add(key, vector.clone());
        }
        let flat_build_time = flat_start.elapsed();
        println!("{:.2?}", flat_build_time);
        
        print!("Flat Index: querying 100 searches... ");
        let flat_query_start = Instant::now();
        let opts = VectorSearchOptions::new().top_k(10);
        for query in &queries {
            let _ = flat_index.search(query, &opts);
        }
        let flat_query_time = flat_query_start.elapsed();
        let flat_avg_per_query = flat_query_time / 100;
        println!("{:.2?} (avg {:.2?} per query)", flat_query_time, flat_avg_per_query);
        
        // SNSW Benchmark
        print!("SNSW: building... ");
        let snsw_start = Instant::now();
        let config = SynthesisConfig::default();
        let snsw_graph = SynthesisGraph::new(config);
        for vector in &dataset {
            snsw_graph.insert(vector.clone()).unwrap();
        }
        let snsw_build_time = snsw_start.elapsed();
        println!("{:.2?}", snsw_build_time);
        
        print!("SNSW: querying 100 searches... ");
        let snsw_query_start = Instant::now();
        for query in &queries {
            let _ = snsw_graph.search_explainable(query, 10).unwrap();
        }
        let snsw_query_time = snsw_query_start.elapsed();
        let snsw_avg_per_query = snsw_query_time / 100;
        println!("{:.2?} (avg {:.2?} per query)", snsw_query_time, snsw_avg_per_query);
        
        // Results
        println!("\nResults:");
        let build_ratio = snsw_build_time.as_secs_f64() / flat_build_time.as_secs_f64();
        let query_ratio = snsw_query_time.as_secs_f64() / flat_query_time.as_secs_f64();
        
        println!("  Build time ratio (SNSW/Flat): {:.2}x", build_ratio);
        println!("  Query time ratio (SNSW/Flat): {:.2}x", query_ratio);
        
        if query_ratio < 1.0 {
            println!("  ✓ SNSW is {:.2}x FASTER for queries", 1.0 / query_ratio);
        } else {
            println!("  ✗ SNSW is {:.2}x SLOWER for queries", query_ratio);
        }
        
        println!("  Storage: {} unique vectors (deduplication active)", snsw_graph.len());
    }
    
    println!("\n{}", "=".repeat(70));
    println!("Benchmark Complete!");
    println!("{}", "=".repeat(70));
    println!("\nNotes:");
    println!("- SNSW build is slower (graph construction overhead)");
    println!("- SNSW query should be faster at scale (O(log n) vs O(n))");
    println!("- SNSW provides explainable results (synthesis paths)");
    println!("- At small scale (<1K), overhead may not be worth it");
}
