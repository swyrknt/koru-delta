/// Batch Performance Demo
///
/// Demonstrates the performance improvement of `put_batch` vs individual `put` calls.
/// Run with: cargo run --example batch_performance_demo --release
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use koru_delta::KoruDelta;
    use serde_json::json;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║     BATCH PERFORMANCE DEMO - KoruDelta v2.0.0                 ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Create temporary database
    let db_path = std::env::temp_dir().join("batch_perf_demo");
    let _ = tokio::fs::remove_dir_all(&db_path).await;

    let db = KoruDelta::start_with_path(&db_path).await?;
    println!("Database initialized at: {}", db_path.display());
    println!();

    let batch_sizes = vec![10, 100, 500, 1000];

    for batch_size in batch_sizes {
        println!("--- Batch size: {} ---", batch_size);

        // Test 1: Individual puts (with fsync each)
        let start = Instant::now();
        for i in 0..batch_size {
            db.put(
                "benchmark",
                &format!("key-{}", i),
                json!({"index": i, "data": "x".repeat(100)}),
            )
            .await?;
        }
        let individual_time = start.elapsed();
        let individual_ops = batch_size as f64 / individual_time.as_secs_f64();

        println!(
            "  Individual puts:  {:>8.2?} ({:>6.0} ops/sec)",
            individual_time, individual_ops
        );

        // Clear for next test
        let _ = tokio::fs::remove_dir_all(&db_path).await;
        let db = KoruDelta::start_with_path(&db_path).await?;

        // Test 2: Batch put (single fsync)
        let items: Vec<(&str, String, serde_json::Value)> = (0..batch_size)
            .map(|i| {
                (
                    "benchmark",
                    format!("key-{}", i),
                    json!({"index": i, "data": "x".repeat(100)}),
                )
            })
            .collect();

        let start = Instant::now();
        let _results = db.put_batch(items).await?;
        let batch_time = start.elapsed();
        let batch_ops = batch_size as f64 / batch_time.as_secs_f64();

        println!(
            "  Batch put:        {:>8.2?} ({:>6.0} ops/sec)",
            batch_time, batch_ops
        );

        // Calculate improvement (batch speedup factor)
        let improvement = batch_ops / individual_ops;
        println!("  Improvement:      {:.1}x faster", improvement);
        println!();
    }

    // Cleanup
    let _ = tokio::fs::remove_dir_all(&db_path).await;

    println!("✅ Demo complete!");
    println!();
    println!("Key insight: Batch performance scales with batch size because");
    println!("we amortize the fsync cost across all items in the batch.");
    println!();
    println!("Typical improvements:");
    println!("  - 10 items:  5-10x faster");
    println!("  - 100 items: 10-30x faster");
    println!("  - 1000 items: 20-50x faster");

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    println!("This example requires native features.");
}
