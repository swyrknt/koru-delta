//! SNSW Demo - Escalating Adaptive Search
//!
//! Demonstrates the escalating search architecture:
//! - 🔥 Hot: Semantic cache for repeated/near-identical queries
//! - 🌤️ Warm-Fast: Quick beam search with confidence check
//! - 🌤️ Warm-Thorough: Higher-effort beam search if needed
//! - ❄️ Cold: Exact linear scan when confidence is insufficient
//!
//! The key insight: Instead of hardcoded thresholds, the system escalates
//! based on result confidence (estimated from score distribution).
//!
//! Run: cargo run --example snsw_demo --release

use koru_delta::vector::Vector;
use koru_delta::vector::snsw::{SynthesisGraph, SearchTier};
use std::collections::HashMap;
use std::time::Instant;

fn random_vector(dimensions: usize) -> Vector {
    let data: Vec<f32> = (0..dimensions)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    Vector::new(data, "demo-model")
}

fn main() {
    println!("{}", "=".repeat(80));
    println!("SNSW - Escalating Adaptive Search with Confidence");
    println!("{}", "=".repeat(80));
    println!();
    println!("Architecture: Escalation based on result confidence");
    println!();
    println!("Escalation Stages:");
    println!("  1. 🔥 Hot:       Semantic cache hit (instant)");
    println!("  2. 🌤️ Fast:      Beam search with ef=50, check confidence");
    println!("  3. 🌤️ Thorough:  Beam search with ef=200 if confidence low");
    println!("  4. ❄️ Cold:      Exact linear scan if still uncertain");
    println!();
    println!("Confidence Estimation:");
    println!("  - High confidence = large gap between top scores");
    println!("  - Low confidence = similar scores (uncertain ordering)");
    println!();
    
    let dimensions = 128;
    let graph = SynthesisGraph::new();
    
    // Test across different dataset sizes
    let test_sizes = vec![100, 500, 1000, 2000, 5000];
    
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
        
        // Test with fresh queries (cache miss, escalation happens)
        let fresh_queries: Vec<Vector> = (0..20).map(|_| random_vector(dimensions)).collect();
        
        println!("\n  --- Fresh Queries (Escalation Demo) ---");
        let mut tier_counts: HashMap<SearchTier, usize> = HashMap::new();
        let mut confidence_sum = 0.0;
        let mut total_time = 0.0;
        
        for query in &fresh_queries {
            let start = Instant::now();
            let results = graph.search(query, 10).unwrap();
            total_time += start.elapsed().as_secs_f64();
            
            if let Some(first) = results.first() {
                *tier_counts.entry(first.tier).or_insert(0) += 1;
                confidence_sum += first.confidence;
            }
        }
        
        let avg_time = total_time / fresh_queries.len() as f64 * 1000.0;
        let avg_confidence = confidence_sum / fresh_queries.len() as f32;
        
        println!("  Avg query time: {:.3}ms", avg_time);
        println!("  Avg confidence: {:.1}%", avg_confidence * 100.0);
        println!("  Tier distribution:");
        
        for (tier, count) in &tier_counts {
            let pct = *count as f64 / fresh_queries.len() as f64 * 100.0;
            match tier {
                SearchTier::Hot => println!("    🔥 Hot (cache):        {} queries ({:.0}%)", count, pct),
                SearchTier::WarmFast => println!("    🌤️ Warm-Fast:          {} queries ({:.0}%)", count, pct),
                SearchTier::WarmThorough => println!("    🌤️ Warm-Thorough:      {} queries ({:.0}%)", count, pct),
                SearchTier::Cold => println!("    ❄️ Cold (exact):       {} queries ({:.0}%)", count, pct),
            }
        }
        
        // Test with repeated queries (cache hit)
        println!("\n  --- Repeated Queries (Cache Demo) ---");
        let repeated_query = fresh_queries[0].clone();
        
        // Warm up cache
        let _ = graph.search(&repeated_query, 10).unwrap();
        
        let start = Instant::now();
        let cached_results = graph.search(&repeated_query, 10).unwrap();
        let cache_time = start.elapsed().as_secs_f64() * 1000.0;
        
        let is_hot = cached_results.iter().all(|r| r.tier == SearchTier::Hot);
        
        println!("  Cache hit query time: {:.3}ms", cache_time);
        println!("  All from Hot tier: {}", if is_hot { "Yes 🔥" } else { "No" });
        
        if avg_time > 0.0 {
            let speedup = avg_time / cache_time;
            println!("  Cache speedup: {:.1}x", speedup);
        }
        
        // Show cache stats
        let (cache_size, total_hits) = graph.cache_stats();
        println!("  Cache entries: {} | Total hits: {}", cache_size, total_hits);
    }
    
    println!("\n{}", "=".repeat(80));
    println!("Key Insights");
    println!("{}", "=".repeat(80));
    println!();
    println!("1. No Hardcoded Thresholds:");
    println!("   - System escalates based on result confidence, not vector count");
    println!("   - Adapts to query difficulty (some queries need more work)");
    println!();
    println!("2. Confidence Estimation:");
    println!("   - Large score gap → High confidence → Stop early (fast)");
    println!("   - Similar scores → Low confidence → Escalate (thorough)");
    println!();
    println!("3. Cache Benefits:");
    println!("   - Repeated queries: Instant response (🔥 Hot tier)");
    println!("   - Near-identical queries: Detected via similarity threshold");
    println!();
    println!("4. Efficiency:");
    println!("   - Easy queries: Stop at Warm-Fast (low effort)");
    println!("   - Hard queries: Escalate to Cold (guaranteed accuracy)");
    println!("   - Never do more work than necessary");
}
