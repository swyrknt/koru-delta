//! End-to-End Validation Suite for KoruDelta
//!
//! This is NOT a test - it's a real-world validation that exercises
//! every feature of the database as a user would actually use it.
//!
//! Run with: cargo run --example e2e_validation

use koru_delta::{KoruDelta, json};
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     KoruDelta End-to-End Feature Validation Suite           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut passed = 0;
    let mut failed = 0;

    // Start the database
    print!("1. Starting database... ");
    let start = Instant::now();
    let db = match KoruDelta::start().await {
        Ok(db) => {
            println!("✅ ({:?})", start.elapsed());
            db
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
            std::process::exit(1);
        }
    };
    passed += 1;

    // ============================================
    // SECTION 1: Basic Storage Operations
    // ============================================
    println!("\n📦 SECTION 1: Basic Storage Operations");
    println!("─────────────────────────────────────────");

    // 1.1 Simple put/get
    print!("   1.1 Put/Get single value... ");
    let start = Instant::now();
    match db
        .put("users", "alice", json!({"name": "Alice", "age": 30}))
        .await
    {
        Ok(_) => match db.get("users", "alice").await {
            Ok(v) => {
                if v.value()["name"] == "Alice" && v.value()["age"] == 30 {
                    println!("✅ ({:?})", start.elapsed());
                    passed += 1;
                } else {
                    println!("❌ Data mismatch");
                    failed += 1;
                }
            }
            Err(e) => {
                println!("❌ Get failed: {}", e);
                failed += 1;
            }
        },
        Err(e) => {
            println!("❌ Put failed: {}", e);
            failed += 1;
        }
    }

    // 1.2 Multiple namespaces
    print!("   1.2 Multiple namespaces... ");
    let start = Instant::now();
    let mut ns_ok = true;
    for i in 0..5 {
        if let Err(e) = db.put(&format!("ns{}", i), "key", json!({"ns": i})).await {
            println!("❌ Failed on ns{}: {}", i, e);
            ns_ok = false;
            break;
        }
    }
    if ns_ok {
        // Verify isolation
        let mut all_correct = true;
        for i in 0..5 {
            match db.get(&format!("ns{}", i), "key").await {
                Ok(v) => {
                    if v.value()["ns"] != i {
                        all_correct = false;
                        break;
                    }
                }
                Err(_) => {
                    all_correct = false;
                    break;
                }
            }
        }
        if all_correct {
            println!("✅ ({:?})", start.elapsed());
            passed += 1;
        } else {
            println!("❌ Namespace isolation failed");
            failed += 1;
        }
    } else {
        failed += 1;
    }

    // 1.3 Complex nested data
    print!("   1.3 Complex nested data... ");
    let start = Instant::now();
    let complex = json!({
        "user": {
            "profile": {
                "name": "Bob",
                "settings": {
                    "theme": "dark",
                    "notifications": true
                }
            },
            "history": [
                {"action": "login", "time": 1},
                {"action": "update", "time": 2}
            ]
        }
    });
    match db.put("complex", "data", complex.clone()).await {
        Ok(_) => match db.get("complex", "data").await {
            Ok(v) => {
                if v.value() == &complex {
                    println!("✅ ({:?})", start.elapsed());
                    passed += 1;
                } else {
                    println!("❌ Data corruption");
                    failed += 1;
                }
            }
            Err(e) => {
                println!("❌ Get failed: {}", e);
                failed += 1;
            }
        },
        Err(e) => {
            println!("❌ Put failed: {}", e);
            failed += 1;
        }
    }

    // 1.4 Large values (~100KB)
    print!("   1.4 Large values (~100KB)... ");
    let start = Instant::now();
    let large_data: serde_json::Value = (0..1000)
        .map(|i| (format!("field{}", i), json!("x".repeat(100))))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();
    match db.put("large", "data", large_data.clone()).await {
        Ok(_) => match db.get("large", "data").await {
            Ok(v) => {
                if v.value() == &large_data {
                    println!("✅ ({:?})", start.elapsed());
                    passed += 1;
                } else {
                    println!("❌ Data corruption");
                    failed += 1;
                }
            }
            Err(e) => {
                println!("❌ Get failed: {}", e);
                failed += 1;
            }
        },
        Err(e) => {
            println!("❌ Put failed: {}", e);
            failed += 1;
        }
    }

    // 1.5 Empty values
    print!("   1.5 Empty/null values... ");
    let start = Instant::now();
    let empty_tests = vec![
        ("empty_obj", json!({})),
        ("empty_arr", json!([])),
        ("null_val", json!(null)),
        ("empty_str", json!("")),
        ("zero", json!(0)),
        ("false_val", json!(false)),
    ];
    let mut empty_ok = true;
    for (key, val) in &empty_tests {
        let key = *key; // Deref to get &str
        if let Err(e) = db.put("empty", key, val.clone()).await {
            println!("❌ Put {} failed: {}", key, e);
            empty_ok = false;
            break;
        }
        match db.get("empty", key).await {
            Ok(v) => {
                if v.value() != val {
                    println!("❌ Data mismatch for {}", key);
                    empty_ok = false;
                    break;
                }
            }
            Err(e) => {
                println!("❌ Get {} failed: {}", key, e);
                empty_ok = false;
                break;
            }
        }
    }
    if empty_ok {
        println!("✅ ({:?})", start.elapsed());
        passed += 1;
    } else {
        failed += 1;
    }

    // ============================================
    // SECTION 2: Versioning & History
    // ============================================
    println!("\n📜 SECTION 2: Versioning & History");
    println!("────────────────────────────────────");

    // 2.1 Version tracking
    print!("   2.1 Version tracking... ");
    let start = Instant::now();
    let v1 = db.put("version", "key", json!({"v": 1})).await.unwrap();
    let v2 = db.put("version", "key", json!({"v": 2})).await.unwrap();
    let v3 = db.put("version", "key", json!({"v": 3})).await.unwrap();

    let mut version_ok = true;
    if v1.previous_version().is_some() {
        println!("❌ First version should have no predecessor");
        version_ok = false;
    }
    if v2.previous_version() != Some(v1.write_id()) {
        println!("❌ Version 2 should link to version 1");
        version_ok = false;
    }
    if v3.previous_version() != Some(v2.write_id()) {
        println!("❌ Version 3 should link to version 2");
        version_ok = false;
    }
    if version_ok {
        println!("✅ ({:?})", start.elapsed());
        passed += 1;
    } else {
        failed += 1;
    }

    // 2.2 History retrieval
    print!("   2.2 History retrieval... ");
    let start = Instant::now();
    match db.history("version", "key").await {
        Ok(history) => {
            if history.len() == 3 {
                // History should be in reverse chronological order
                let current = db.get("version", "key").await.unwrap();
                if current.value()["v"] == 3 {
                    println!("✅ ({:?})", start.elapsed());
                    passed += 1;
                } else {
                    println!("❌ Current value incorrect");
                    failed += 1;
                }
            } else {
                println!("❌ Expected 3 versions, got {}", history.len());
                failed += 1;
            }
        }
        Err(e) => {
            println!("❌ History failed: {}", e);
            failed += 1;
        }
    }

    // ============================================
    // SECTION 3: Listing & Querying
    // ============================================
    println!("\n🔍 SECTION 3: Listing & Querying");
    println!("──────────────────────────────────");

    // 3.1 List namespaces
    print!("   3.1 List namespaces... ");
    let start = Instant::now();
    let namespaces = db.list_namespaces().await;
    // Should have: users, ns0-4, complex, large, empty, version, plus more
    if namespaces.len() >= 8 {
        println!(
            "✅ (found {} namespaces, {:?})",
            namespaces.len(),
            start.elapsed()
        );
        passed += 1;
    } else {
        println!(
            "❌ Expected at least 8 namespaces, found {}",
            namespaces.len()
        );
        failed += 1;
    }

    // 3.2 List keys
    print!("   3.2 List keys... ");
    let start = Instant::now();
    // Put some keys
    for i in 0..10 {
        db.put("list_test", &format!("key{}", i), json!(i))
            .await
            .unwrap();
    }
    let keys = db.list_keys("list_test").await;
    if keys.len() == 10 {
        println!("✅ (found {} keys, {:?})", keys.len(), start.elapsed());
        passed += 1;
    } else {
        println!("❌ Expected 10 keys, found {}", keys.len());
        failed += 1;
    }

    // 3.3 Query with filter
    print!("   3.3 Query with filter... ");
    let start = Instant::now();
    // Populate data
    for i in 0..100 {
        db.put(
            "query_test",
            &format!("item{}", i),
            json!({
                "category": i % 10,
                "value": i * 10
            }),
        )
        .await
        .unwrap();
    }

    // Query with a filter using Query struct
    use koru_delta::query::{Filter, Query};
    let query = Query {
        filters: vec![Filter::eq("category", 5)],
        limit: Some(10),
        ..Default::default()
    };

    let results = db.query("query_test", query).await;

    match results {
        Ok(items) => {
            if items.records.len() == 10 {
                println!(
                    "✅ (found {} items, {:?})",
                    items.records.len(),
                    start.elapsed()
                );
                passed += 1;
            } else {
                println!("❌ Expected 10 items, found {}", items.records.len());
                failed += 1;
            }
        }
        Err(e) => {
            println!("❌ Query failed: {}", e);
            failed += 1;
        }
    }

    // ============================================
    // SECTION 4: Error Handling
    // ============================================
    println!("\n⚠️  SECTION 4: Error Handling");
    println!("───────────────────────────────");

    // 4.1 Get non-existent key
    print!("   4.1 Get non-existent key... ");
    let start = Instant::now();
    match db.get("nonexistent", "nonexistent").await {
        Ok(_) => {
            println!("❌ Should have returned error");
            failed += 1;
        }
        Err(_) => {
            println!("✅ ({:?})", start.elapsed());
            passed += 1;
        }
    }

    // 4.2 Delete and verify
    print!("   4.2 Delete operation... ");
    let start = Instant::now();
    db.put("delete_test", "todelete", json!({"data": "value"}))
        .await
        .unwrap();
    if let Err(e) = db.delete("delete_test", "todelete").await {
        println!("❌ Delete failed: {}", e);
        failed += 1;
    } else {
        // After delete, get should still work but return null (tombstone)
        match db.get("delete_test", "todelete").await {
            Ok(v) => {
                if v.value().is_null() {
                    println!("✅ ({:?})", start.elapsed());
                    passed += 1;
                } else {
                    println!("❌ Expected null tombstone, got {}", v.value());
                    failed += 1;
                }
            }
            Err(_) => {
                println!("✅ ({:?})", start.elapsed());
                passed += 1;
            }
        }
    }

    // ============================================
    // SECTION 5: Concurrent Operations
    // ============================================
    println!("\n🔄 SECTION 5: Concurrent Operations");
    println!("─────────────────────────────────────");

    // 5.1 Concurrent writes
    print!("   5.1 Concurrent writes (100 ops)... ");
    let start = Instant::now();
    let mut handles = vec![];
    for i in 0..100 {
        let db_clone = db.clone();
        handles.push(tokio::spawn(async move {
            db_clone
                .put("concurrent", &format!("key{}", i), json!({"id": i}))
                .await
        }));
    }
    let mut all_ok = true;
    for handle in handles {
        if let Err(e) = handle.await.unwrap() {
            println!("❌ Task failed: {}", e);
            all_ok = false;
            break;
        }
    }
    if all_ok {
        let keys = db.list_keys("concurrent").await;
        if keys.len() == 100 {
            println!("✅ ({:?})", start.elapsed());
            passed += 1;
        } else {
            println!("❌ Expected 100 keys, found {}", keys.len());
            failed += 1;
        }
    } else {
        failed += 1;
    }

    // 5.2 Concurrent reads and writes
    print!("   5.2 Concurrent reads/writes... ");
    let start = Instant::now();
    let mut handles = vec![];

    // Spawn writers
    for i in 0..10 {
        let db_clone = db.clone();
        handles.push(tokio::spawn(async move {
            for j in 0..10 {
                let _ = db_clone
                    .put("rw_test", "shared", json!({"writer": i, "iter": j}))
                    .await;
            }
        }));
    }

    // Spawn readers
    for _ in 0..10 {
        let db_clone = db.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                let _ = db_clone.get("rw_test", "shared").await;
            }
        }));
    }

    let mut rw_ok = true;
    for handle in handles {
        if let Err(e) = handle.await {
            println!("❌ Task panicked: {}", e);
            rw_ok = false;
            break;
        }
    }
    if rw_ok {
        println!("✅ ({:?})", start.elapsed());
        passed += 1;
    } else {
        failed += 1;
    }

    // ============================================
    // SECTION 6: Stats & Metadata
    // ============================================
    println!("\n📊 SECTION 6: Stats & Metadata");
    println!("────────────────────────────────");

    // 6.1 Database stats
    print!("   6.1 Database stats... ");
    let start = Instant::now();
    let stats = db.stats().await;
    if stats.key_count > 0 {
        println!("✅ ({} keys, {:?})", stats.key_count, start.elapsed());
        passed += 1;
    } else {
        println!("❌ Stats returned 0 keys");
        failed += 1;
    }

    // ============================================
    // Summary
    // ============================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                      VALIDATION SUMMARY                      ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  ✅ Passed: {:3}                                              ║",
        passed
    );
    println!(
        "║  ❌ Failed: {:3}                                              ║",
        failed
    );
    println!("║  ─────────────────────────────────────────────────────────   ║");
    println!(
        "║  Total:    {:3}                                               ║",
        passed + failed
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    if failed > 0 {
        println!("\n⚠️  {} feature(s) failed validation!", failed);
        std::process::exit(1);
    } else {
        println!("\n✨ All {} features validated successfully!", passed);
    }
}
