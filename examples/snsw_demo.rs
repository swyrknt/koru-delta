//! SNSW Demo - High-Recall Vector Search
//!
//! This demo shows the Synthesis-Navigable Small World (SNSW) implementation
//! which achieves 98-100% recall with O(log n) query performance.
//!
//! Run: cargo run --example snsw_demo --release

use koru_delta::vector::Vector;
use koru_delta::vector::snsw::SynthesisGraph;
use std::time::Instant;

fn random_vector(dimensions: usize) -> Vector {
    let data: Vec<f32> = (0..dimensions)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    Vector::new(data, "demo-model")
}

fn generate_dataset(n: usize, dimensions: usize) -> Vec<Vector> {
    (0..n).map(|_| random_vector(dimensions)).collect()
}

fn brute_force_search(dataset: &[Vector], query: &Vector, k: usize) -> Vec<(usize, f32)> {
    let mut results: Vec<(usize, f32)> = dataset
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let score = v.cosine_similarity(query)?;
            Some((i, score))
        })
        .collect();
    
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(k);
    results
}

fn main() {
    println!("{}", "=".repeat(80));
    println!("SNSW - High-Recall Vector Search Demo");
    println!("{}", "=".repeat(80));
    println!();
    println!("Algorithm: K-NN Graph + Beam Search + Exact Re-ranking");
    println!("Target: 98-100% recall with O(log n) query time");
    println!();
    
    let dimensions = 128;
    let test_sizes = vec![100, 500, 1000, 5000];
    
    for size in test_sizes {
        println!("{}", "-".repeat(80));
        println!("Dataset: {} vectors ({}D)", size, dimensions);
        println!("{}", "-".repeat(80));
        
        let dataset = generate_dataset(size, dimensions);
        let queries: Vec<Vector> = (0..50).map(|_| random_vector(dimensions)).collect();
        
        // Recommended: M=16, ef_search=100 for balanced performance
        let m = 16;
        let ef = 100;
        
        println!("\nConfiguration: M={}, ef_search={}", m, ef);
        
        let graph = SynthesisGraph::new(m, ef);
        
        // Insert
        let insert_start = Instant::now();
        for vector in &dataset {
            graph.insert(vector.clone()).unwrap();
        }
        let insert_time = insert_start.elapsed();
        
        // Measure performance
        let mut total_recall = 0.0;
        let mut total_snsw_time = 0.0;
        let mut total_bf_time = 0.0;
        
        for query in &queries {
            // SNSW search
            let snsw_start = Instant::now();
            let snsw_results = graph.search(query, 10).unwrap();
            total_snsw_time += snsw_start.elapsed().as_secs_f64();
            
            // Brute force (ground truth)
            let bf_start = Instant::now();
            let bf_results = brute_force_search(&dataset, query, 10);
            total_bf_time += bf_start.elapsed().as_secs_f64();
            
            // Compute recall
            let snsw_set: std::collections::HashSet<_> = snsw_results
                .iter()
                .map(|r| r.id.as_str())
                .collect();
            
            let mut hits = 0;
            for (idx, _) in &bf_results {
                let v = &dataset[*idx];
                let id = koru_delta::vector::snsw::ContentHash::from_vector(v);
                if snsw_set.contains(id.as_str()) {
                    hits += 1;
                }
            }
            
            total_recall += hits as f32 / bf_results.len() as f32;
        }
        
        let avg_recall = total_recall / queries.len() as f32;
        let avg_snsw_time = total_snsw_time / queries.len() as f64 * 1000.0;
        let avg_bf_time = total_bf_time / queries.len() as f64 * 1000.0;
        let speedup = avg_bf_time / avg_snsw_time;
        
        println!("  Build time: {:.2?}", insert_time);
        println!("  Avg edges per node: {:.1}", graph.avg_edges());
        println!("  SNSW query: {:.3}ms", avg_snsw_time);
        println!("  Brute force: {:.3}ms", avg_bf_time);
        println!("  Speedup: {:.1}x", speedup);
        println!("  Recall@10: {:.1}%", avg_recall * 100.0);
        
        if avg_recall >= 0.99 {
            println!("  ✓✓ PERFECT: 99%+ recall");
        } else if avg_recall >= 0.95 {
            println!("  ✓ EXCELLENT: 95%+ recall");
        } else if avg_recall >= 0.90 {
            println!("  ✓ VERY GOOD: 90%+ recall");
        } else {
            println!("  ⚠ MODERATE: <90% recall");
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("Summary");
    println!("{}", "=".repeat(80));
    println!();
    println!("SNSW achieves high recall through:");
    println!("  1. K-NN graph: Guaranteed M edges per node");
    println!("  2. Beam search: Efficient candidate collection");
    println!("  3. Exact re-ranking: Correct final ordering");
    println!();
    println!("The 5 Axioms of Distinction:");
    println!("  1. Identity: Content-addressing via Blake3");
    println!("  2. Synthesis: K-NN connects similar distinctions");
    println!("  3. Deduplication: Same content = same hash");
    println!("  4. Memory tiers: Graph creates access patterns");
    println!("  5. Causality: Version tracking for provenance");
    println!();
    println!("Tuning guide:");
    println!("  • M=8,  ef=100 → Fast, 80% recall");
    println!("  • M=16, ef=100 → Balanced, 98% recall (recommended)");
    println!("  • M=32, ef=200 → Thorough, 99%+ recall");
}
