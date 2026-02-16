//! Phase 7 Integration Tests
//!
//! Tests for the unified core with full memory tiering:
//! - Tiered GET (Hot → Warm → Cold → Storage)
//! - Background process orchestration
//! - Memory promotion/demotion
//! - Genome extraction

use koru_delta::KoruDelta;
use koru_delta::engine::SharedEngine;
use serde_json::json;

/// Test tiered memory GET with promotion
#[tokio::test]
async fn test_tiered_get_promotion() {
    let db = KoruDelta::start().await.unwrap();

    // Put a value
    db.put("test", "key1", json!({"data": "value1"}))
        .await
        .unwrap();

    // First get - should go to storage and promote to hot
    let v1 = db.get("test", "key1").await.unwrap();
    assert_eq!(v1.value()["data"], "value1");

    // Second get - should hit hot memory (faster)
    let v2 = db.get("test", "key1").await.unwrap();
    assert_eq!(v2.value()["data"], "value1");

    // Both should be identical
    assert_eq!(v1.write_id(), v2.write_id());
}

/// Test hot memory capacity management
#[tokio::test]
async fn test_hot_memory_eviction_to_warm() {
    let db = KoruDelta::start().await.unwrap();

    // Fill hot memory beyond capacity (default 1000)
    for i in 0..1100 {
        db.put("evict_test", &format!("key{}", i), json!({"i": i}))
            .await
            .unwrap();
    }

    // All values should still be retrievable from storage
    for i in 0..100 {
        let v = db.get("evict_test", &format!("key{}", i)).await.unwrap();
        assert_eq!(v.value()["i"], i);
    }
}

/// Test background processes start successfully
#[tokio::test]
async fn test_background_processes_start() {
    let db = KoruDelta::start().await.unwrap();

    // Put some data
    for i in 0..100 {
        db.put("bg_test", &format!("key{}", i), json!({"v": i}))
            .await
            .unwrap();
    }

    // Wait a moment for background tasks to potentially run
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // All data should still be accessible
    for i in 0..100 {
        let v = db.get("bg_test", &format!("key{}", i)).await.unwrap();
        assert_eq!(v.value()["v"], i);
    }
}

/// Test genome storage in deep memory
#[tokio::test]
async fn test_genome_storage() {
    use koru_delta::memory::EssenceAgent;
    use koru_delta::processes::GenomeUpdateProcess;

    let process = GenomeUpdateProcess::new();

    // Extract a genome
    let genome = process.extract_genome();

    // Store in deep memory
    let shared_engine = SharedEngine::new();
    let deep = EssenceAgent::new(&shared_engine);
    deep.store_genome("test_genome", genome.clone());

    // Retrieve it
    let retrieved = deep.get_genome("test_genome").unwrap();
    assert_eq!(retrieved.version, genome.version);
}

/// Test memory tier statistics
#[tokio::test]
async fn test_memory_tier_stats() {
    let db = KoruDelta::start().await.unwrap();

    // Put some data
    for i in 0..100 {
        db.put("stats_test", &format!("key{}", i), json!({"i": i}))
            .await
            .unwrap();
    }

    // Get stats
    let stats = db.stats().await;
    assert_eq!(stats.key_count, 100);

    // Access some keys to generate hits
    for i in 0..50 {
        let _ = db.get("stats_test", &format!("key{}", i)).await.unwrap();
    }

    // Access same keys again (should hit hot memory)
    for i in 0..50 {
        let _ = db.get("stats_test", &format!("key{}", i)).await.unwrap();
    }
}

/// Test contains checks all tiers
#[tokio::test]
async fn test_contains_tiered() {
    let db = KoruDelta::start().await.unwrap();

    // Put a value
    db.put("contains", "key1", json!({"exists": true}))
        .await
        .unwrap();

    // Should find it
    assert!(db.contains("contains", "key1").await);

    // Should not find non-existent
    assert!(!db.contains("contains", "nonexistent").await);
}

/// Test that sync get works
#[tokio::test]
async fn test_sync_get() {
    let db = KoruDelta::start().await.unwrap();

    // Put a value
    db.put("sync", "key1", json!({"sync": true})).await.unwrap();

    // Sync get should work
    let v = db.get_sync("sync", "key1").unwrap();
    assert_eq!(v.value()["sync"], true);
}

/// Test memory tier cascade under load
#[tokio::test]
async fn test_memory_tier_cascade() {
    let db = KoruDelta::start().await.unwrap();

    // Phase 1: Insert many unique keys
    for i in 0..500 {
        db.put("cascade", &format!("key{}", i), json!({"phase": 1, "i": i}))
            .await
            .unwrap();
    }

    // Phase 2: Access subset frequently (promotes to hot)
    for _ in 0..5 {
        for i in 0..100 {
            let _ = db.get("cascade", &format!("key{}", i)).await.unwrap();
        }
    }

    // Phase 3: Insert more to trigger evictions
    for i in 500..1000 {
        db.put("cascade", &format!("key{}", i), json!({"phase": 3, "i": i}))
            .await
            .unwrap();
    }

    // Phase 4: Verify all data still accessible
    for i in 0..1000 {
        let v = db.get("cascade", &format!("key{}", i)).await.unwrap();
        assert!(v.value()["i"] == i);
    }
}

/// Test shutdown gracefully stops background tasks
#[tokio::test]
async fn test_graceful_shutdown() {
    let db = KoruDelta::start().await.unwrap();

    // Put some data
    db.put("shutdown", "key1", json!({"data": "test"}))
        .await
        .unwrap();

    // Shutdown
    db.shutdown().await.unwrap();

    // After shutdown, database is consumed
    // This test mainly verifies shutdown doesn't panic
}
