//! Demo: SNSW vs Flat Index Performance Comparison
//!
//! This demo shows the performance characteristics of SNSW compared
//! to the traditional flat (brute-force) index.

use koru_delta::vector::{AnnIndex, FlatIndex, Vector, VectorSearchOptions};
use koru_delta::vector::snsw::{SynthesisConfig, SynthesisGraph};
use std::time::Instant;

fn random_vector(dimensions: usize) -> Vector {
    let data: Vec<f32> = (0..dimensions)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    Vector::new(data, "demo-model")
}

fn main() {
    println!("{}", "=".repeat(70));
    println!("SNSW vs Flat Index Performance Demo");
    println!("{}", "=".repeat(70));
    
    let sizes = [100, 500, 1000];
    let dimensions = 128;
    
    for size in sizes {
        println!("\n--- Dataset Size: {} vectors ({}D) ---", size, dimensions);
        
        // Generate dataset
        let dataset: Vec<Vector> = (0..size)
            .map(|_| random_vector(dimensions))
            .collect();
        
        let query = random_vector(dimensions);
        
        // Flat Index
        println!("\nFlat Index (Brute Force):");
        let flat_start = Instant::now();
        let flat_index = FlatIndex::new();
        for (i, vector) in dataset.iter().enumerate() {
            let key = koru_delta::FullKey::new("test", format!("vec_{}", i));
            flat_index.add(key, vector.clone());
        }
        let flat_insert_time = flat_start.elapsed();
        
        let flat_search_start = Instant::now();
        let opts = VectorSearchOptions::new().top_k(10);
        let flat_results = flat_index.search(&query, &opts);
        let flat_search_time = flat_search_start.elapsed();
        
        println!("  Insertion: {:?}", flat_insert_time);
        println!("  Search (10-NN): {:?}", flat_search_time);
        println!("  Results: {} vectors", flat_results.len());
        
        // SNSW
        println!("\nSNSW (Synthesis-Navigable):");
        let snsw_start = Instant::now();
        let config = SynthesisConfig::default();
        let snsw_graph = SynthesisGraph::new(config);
        for vector in dataset.iter() {
            snsw_graph.insert(vector.clone()).unwrap();
        }
        let snsw_insert_time = snsw_start.elapsed();
        
        let snsw_search_start = Instant::now();
        let snsw_results = snsw_graph.search_explainable(&query, 10).unwrap();
        let snsw_search_time = snsw_search_start.elapsed();
        
        println!("  Insertion: {:?}", snsw_insert_time);
        println!("  Search (10-NN with explanation): {:?}", snsw_search_time);
        println!("  Results: {} vectors", snsw_results.len());
        
        // Show explainability for first result
        if let Some(first) = snsw_results.first() {
            println!("\n  Example Explanation:");
            println!("    Score: {:.3}", first.score);
            println!("    Factors: geo={:.3}, shared={:.3}, path={:.3}, causal={:.3}",
                first.factor_scores.geometric,
                first.factor_scores.shared_distinctions,
                first.factor_scores.path_length,
                first.factor_scores.causal
            );
        }
        
        // Comparison
        let speedup = flat_search_time.as_secs_f64() / snsw_search_time.as_secs_f64();
        println!("\n  Comparison:");
        println!("    Search speedup: {:.2}x", speedup);
        println!("    Deduplication: {} unique / {} total", snsw_graph.len(), dataset.len());
    }
    
    println!("\n{}", "=".repeat(70));
    println!("Demo Complete!");
    println!("{}", "=".repeat(70));
    println!("\nKey Findings:");
    println!("- SNSW provides explainable results (synthesis paths)");
    println!("- Content-addressing enables automatic deduplication");
    println!("- Factor scores show WHY vectors are similar");
}
