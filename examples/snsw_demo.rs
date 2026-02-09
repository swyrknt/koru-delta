//! SNSW Demo - Production-Ready Adaptive Search
//!
//! Demonstrates the production-grade implementation:
//! - 🔥 Hot: O(1) exact cache match (generation-based, survives insertions)
//! - 🌤️ Warm-Fast/Thorough: Beam search with adaptive thresholds
//! - ❄️ Cold: Exact linear scan when needed
//! - 📊 Adaptive Learning: Thresholds improve from query feedback
//!
//! Run: cargo run --example snsw_demo --release

use koru_delta::vector::snsw::{SearchTier, SynthesisGraph};
use koru_delta::vector::Vector;
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
    println!("SNSW - Production-Ready Adaptive Search");
    println!("{}", "=".repeat(80));
    println!();
    println!("Key Features:");
    println!("  • Generation-based cache (survives {} inserts)", 100);
    println!("  • O(1) exact cache match (no scanning overhead)");
    println!("  • Adaptive thresholds (learned from query feedback)");
    println!("  • Confidence verification (vs actual recall)");
    println!();

    let dimensions = 128;
    let graph = SynthesisGraph::new();

    // Phase 1: Small dataset - cache behavior
    println!("{}", "-".repeat(80));
    println!("Phase 1: Cache Behavior (100 vectors)");
    println!("{}", "-".repeat(80));

    let insert_start = Instant::now();
    for _ in 0..100 {
        graph.insert(random_vector(dimensions)).unwrap();
    }
    println!("Inserted 100 vectors in {:.2?}", insert_start.elapsed());

    let query = random_vector(dimensions);

    // First search - cache miss
    let start = Instant::now();
    let _results1 = graph.search(&query, 10).unwrap();
    let time1 = start.elapsed().as_secs_f64() * 1000.0;
    println!("First search:  {:.3}ms (cache miss)", time1);

    // Second search - cache hit
    let start = Instant::now();
    let results2 = graph.search(&query, 10).unwrap();
    let time2 = start.elapsed().as_secs_f64() * 1000.0;
    let is_hot = results2.iter().all(|r| r.tier == SearchTier::Hot);
    println!(
        "Second search: {:.3}ms (cache {}) {}🔥",
        time2,
        if is_hot { "HIT" } else { "miss" },
        if is_hot { "" } else { "NOT " }
    );

    if time2 > 0.0 {
        println!("Cache speedup: {:.1}x", time1 / time2);
    }

    let (cache_size, hits, epoch) = graph.cache_stats();
    println!(
        "Cache stats: {} entries, {} hits, epoch {}",
        cache_size, hits, epoch
    );

    // Phase 2: Adaptive threshold learning
    println!("\n{}", "-".repeat(80));
    println!("Phase 2: Adaptive Threshold Learning");
    println!("{}", "-".repeat(80));

    // Insert more vectors
    for _ in 0..500 {
        graph.insert(random_vector(dimensions)).unwrap();
    }
    println!("Total vectors: {}", graph.len());

    let (initial_fast, _) = graph.get_thresholds();
    println!("Initial fast threshold: {:.2}", initial_fast);

    // Generate feedback through searches
    println!("\nGenerating query feedback...");
    for i in 0..30 {
        let query = random_vector(dimensions);
        let _ = graph.search(&query, 10).unwrap();

        if i % 10 == 9 {
            let (fast, thorough) = graph.get_thresholds();
            println!(
                "  After {} queries: fast={:.2}, thorough={:.2}",
                i + 1,
                fast,
                thorough
            );
        }
    }

    // Phase 3: Tier distribution analysis
    println!("\n{}", "-".repeat(80));
    println!("Phase 3: Tier Distribution Analysis");
    println!("{}", "-".repeat(80));

    // More vectors to trigger different tiers
    for _ in 0..500 {
        graph.insert(random_vector(dimensions)).unwrap();
    }
    println!("Total vectors: {}\n", graph.len());

    let test_queries: Vec<Vector> = (0..50).map(|_| random_vector(dimensions)).collect();
    let mut tier_counts: HashMap<SearchTier, usize> = HashMap::new();
    let mut total_time = 0.0;
    let mut total_confidence = 0.0;

    for query in &test_queries {
        let start = Instant::now();
        let results = graph.search(query, 10).unwrap();
        total_time += start.elapsed().as_secs_f64();

        if let Some(first) = results.first() {
            *tier_counts.entry(first.tier).or_insert(0) += 1;
            total_confidence += first.confidence;
        }
    }

    let avg_time = total_time / test_queries.len() as f64 * 1000.0;
    let avg_confidence = total_confidence / test_queries.len() as f32;

    println!("Average query time: {:.3}ms", avg_time);
    println!("Average confidence: {:.1}%\n", avg_confidence * 100.0);

    println!("Tier distribution ({} queries):", test_queries.len());
    let mut tiers: Vec<_> = tier_counts.iter().collect();
    tiers.sort_by(|a, b| b.1.cmp(a.1));

    for (tier, count) in tiers {
        let pct = *count as f64 / test_queries.len() as f64 * 100.0;
        match tier {
            SearchTier::Hot => println!(
                "  🔥 Hot (cache):          {:3} queries ({:.0}%)",
                count, pct
            ),
            SearchTier::WarmFast => println!(
                "  🌤️ Warm-Fast:           {:3} queries ({:.0}%)",
                count, pct
            ),
            SearchTier::WarmThorough => println!(
                "  🌤️ Warm-Thorough:       {:3} queries ({:.0}%)",
                count, pct
            ),
            SearchTier::Cold => println!(
                "  ❄️ Cold (exact):        {:3} queries ({:.0}%)",
                count, pct
            ),
        }
    }

    // Final stats
    println!("\n{}", "-".repeat(80));
    println!("Final Statistics");
    println!("{}", "-".repeat(80));

    let (cache_size, hits, epoch) = graph.cache_stats();
    let (fast_thresh, thorough_thresh) = graph.get_thresholds();

    println!("Graph size:      {} vectors", graph.len());
    println!("Avg edges:       {:.1}", graph.avg_edges());
    println!("Cache entries:   {}", cache_size);
    println!("Cache hits:      {}", hits);
    println!("Current epoch:   {}", epoch);
    println!("Fast threshold:  {:.2} (adaptive)", fast_thresh);
    println!("Thorough thresh: {:.2}", thorough_thresh);

    println!("\n{}", "=".repeat(80));
    println!("Summary: Production-Ready Features");
    println!("{}", "=".repeat(80));
    println!();
    println!("✅ Generation-based cache:");
    println!("   - Survives {} insertions before invalidation", 100);
    println!("   - Lazy invalidation (check on access)");
    println!("   - O(1) exact match only (no scanning)");
    println!();
    println!("✅ Adaptive thresholds:");
    println!("   - Start with reasonable defaults");
    println!("   - Learn from actual query performance");
    println!("   - Self-tuning based on observed recall");
    println!();
    println!("✅ Confidence verification:");
    println!("   - Compare fast vs thorough results");
    println!("   - Actual recall measurement");
    println!("   - Feedback drives threshold adjustments");
    println!();
    println!("The system automatically finds the optimal balance");
    println!("between speed and accuracy for your specific data.");
}
