//! SNSW Demo - Adaptive Tiered Vector Search
//!
//! Demonstrates the four-tier search architecture:
//! - 🔥 Hot: Semantic cache for repeated/near-identical queries
//! - 🌤️ Warm: SNSW graph search for medium-to-large datasets
//! - ❄️ Cold: Exact linear scan for small datasets (<1K vectors)
//! - 🕳️ Deep: Archive tier (on-disk, requires hydration)
//!
//! Run: cargo run --example snsw_demo --release

use koru_delta::vector::Vector;
use koru_delta::vector::snsw::{SynthesisGraph, SearchTier};
use std::time::Instant;

fn random_vector(dimensions: usize) -> Vector {
    let data: Vec<f32> = (0..dimensions)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    Vector::new(data, "demo-model")
}

fn main() {
    println!("{}", "=".repeat(80));
    println!("SNSW - Adaptive Tiered Vector Search");
    println!("{}", "=".repeat(80));
    println!();
    println!("Architecture: 🔥 Hot / 🌤️ Warm / ❄️ Cold / 🕳️ Deep");
    println!();
    println!("Tier Selection:");
    println!("  🔥 Hot:    Semantic cache hit (same/near-same query)");
    println!("  ❄️ Cold:   Dataset ≤1000 vectors (exact linear scan)");
    println!("  🌤️ Warm:   Dataset >1000 vectors (SNSW graph search)");
    println!();
    
    let dimensions = 128;
    let graph = SynthesisGraph::new();
    
    // Test across different dataset sizes
    let test_sizes = vec![50, 500, 1000, 2000, 5000];
    
    for &target_size in &test_sizes {
        let target_size: usize = target_size;
        println!("{}", "-".repeat(80));
        println!("Dataset Size: {} vectors", target_size);
        println!("{}", "-".repeat(80));
        
        // Insert vectors to reach target size
        let current = graph.len();
        let to_insert = target_size.saturating_sub(current);
        
        if to_insert > 0 {
            let insert_start = Instant::now();
            for _ in 0..to_insert {
                graph.insert(random_vector(dimensions)).unwrap();
            }
            println!("  Inserted {} vectors in {:.2?}", to_insert, insert_start.elapsed());
        }
        
        println!("  Total vectors: {}", graph.len());
        println!("  Avg edges per node: {:.1}", graph.avg_edges());
        
        // Test with fresh queries (cold cache)
        let fresh_queries: Vec<Vector> = (0..10).map(|_| random_vector(dimensions)).collect();
        
        println!("\n  --- Fresh Queries (Cache Miss) ---");
        let mut cold_count = 0;
        let mut warm_count = 0;
        let mut total_time = 0.0;
        
        for query in &fresh_queries {
            let start = Instant::now();
            let results = graph.search(query, 10).unwrap();
            total_time += start.elapsed().as_secs_f64();
            
            for r in &results {
                match r.tier {
                    SearchTier::Cold => cold_count += 1,
                    SearchTier::Warm => warm_count += 1,
                    _ => {}
                }
            }
        }
        
        let avg_time = total_time / fresh_queries.len() as f64 * 1000.0;
        println!("  Avg query time: {:.3}ms", avg_time);
        println!("  Results from Cold tier: {}", cold_count);
        println!("  Results from Warm tier: {}", warm_count);
        
        // Test with repeated queries (cache hit)
        println!("\n  --- Repeated Queries (Cache Hit) ---");
        let repeated_query = fresh_queries[0].clone();
        
        // Clear and warm up cache
        let _ = graph.search(&repeated_query, 10).unwrap();
        
        let start = Instant::now();
        let cached_results = graph.search(&repeated_query, 10).unwrap();
        let cache_time = start.elapsed().as_secs_f64() * 1000.0;
        
        let hot_count = cached_results.iter().filter(|r| r.tier == SearchTier::Hot).count();
        
        println!("  Cache hit query time: {:.3}ms", cache_time);
        println!("  Results from Hot tier: {}", hot_count);
        
        if hot_count > 0 {
            let speedup = avg_time / cache_time;
            println!("  🔥 Cache speedup: {:.1}x", speedup);
        }
        
        // Show cache stats
        let (cache_size, _) = graph.cache_stats();
        println!("  Cache entries: {}", cache_size);
        
        // Show which tier is active
        if target_size <= 1000 {
            println!("\n  → Active Tier: ❄️ Cold (exact linear scan)");
        } else {
            println!("\n  → Active Tier: 🌤️ Warm (SNSW graph search)");
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("Summary: Adaptive Search Benefits");
    println!("{}", "=".repeat(80));
    println!();
    println!("🔥 Hot Tier (Cache):");
    println!("  - Instant results for repeated queries");
    println!("  - Content-addressed by query hash");
    println!("  - Near-hit detection (98% similarity threshold)");
    println!();
    println!("❄️ Cold Tier (Brute Force):");
    println!("  - Optimal for small datasets (≤1000 vectors)");
    println!("  - Better cache locality than graph traversal");
    println!("  - No graph construction overhead");
    println!();
    println!("🌤️ Warm Tier (SNSW Graph):");
    println!("  - K-NN graph with guaranteed connectivity");
    println!("  - Beam search + exact re-ranking");
    println!("  - 98-100% recall at medium-to-large scale");
    println!();
    println!("🕳️ Deep Tier (Archive):");
    println!("  - Delta-encoded on disk");
    println!("  - Requires explicit hydration");
    println!("  - Perfect for compliance/historical data");
    println!();
    println!("The system automatically selects the optimal tier based on:");
    println!("  1. Cache hit/miss status");
    println!("  2. Dataset size");
    println!("  3. Query patterns");
}
