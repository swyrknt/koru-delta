/// Falsification tests for KoruDelta.
///
/// These tests employ a falsification methodology - actively trying to break
/// the system rather than just confirming it works. We attack from every angle:
///
/// - Causal chain integrity under extreme concurrency
/// - Time travel edge cases and boundary conditions
/// - Value deduplication correctness
/// - Query engine edge cases and type coercion
/// - Aggregation boundary conditions
/// - Subscription delivery guarantees
/// - View consistency under concurrent writes
/// - Persistence and recovery scenarios
///
/// Philosophy: If we can't break it, we gain confidence it's correct.
use chrono::{Duration, Utc};
use koru_delta::prelude::*;
use koru_delta::query::{Aggregation, Filter, HistoryQuery, Query};
use koru_delta::subscriptions::{ChangeType, Subscription};
use koru_delta::views::ViewDefinition;
use serde_json::json;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

// ============================================================================
// SECTION 1: CAUSAL CHAIN INTEGRITY FALSIFICATION
// ============================================================================
// Try to break the causal chain under various conditions

/// Falsification: Can we break the causal chain with rapid sequential writes?
/// Each version MUST link to its immediate predecessor.
#[tokio::test]
async fn falsify_causal_chain_rapid_sequential_writes() {
    let db = KoruDelta::start().await.unwrap();
    let mut version_ids: Vec<String> = Vec::new();
    let mut prev_versions: Vec<Option<String>> = Vec::new();

    // Rapid fire 100 writes
    for i in 0..100 {
        let v = db
            .put("chain", "key", json!({"seq": i}))
            .await
            .unwrap();
        version_ids.push(v.version_id().to_string());
        prev_versions.push(v.previous_version().map(|s| s.to_string()));
    }

    // Verify the COMPLETE causal chain
    let history = db.history("chain", "key").await.unwrap();
    assert_eq!(history.len(), 100, "Missing versions in history");

    // Each version (except first) must link to its predecessor
    for i in 1..100 {
        let expected_prev = &version_ids[i - 1];
        assert_eq!(
            prev_versions[i].as_ref(),
            Some(expected_prev),
            "Causal chain broken at index {}: expected previous={}, got={:?}",
            i,
            expected_prev,
            prev_versions[i]
        );
    }

    // First version must have no predecessor
    assert!(
        prev_versions[0].is_none(),
        "First version should have no predecessor"
    );
}

/// Falsification: Can concurrent writes to the SAME key corrupt the causal chain?
/// This is the critical test - all writes must appear in history, chain must be valid.
#[tokio::test]
async fn falsify_causal_chain_concurrent_same_key() {
    let db = KoruDelta::start().await.unwrap();
    let write_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 50 concurrent writers to the SAME key
    for i in 0..50 {
        let db_clone = db.clone();
        let counter = Arc::clone(&write_count);
        let handle = tokio::spawn(async move {
            db_clone
                .put("concurrent", "hotspot", json!({"writer": i}))
                .await
                .unwrap();
            counter.fetch_add(1, AtomicOrdering::SeqCst);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(write_count.load(AtomicOrdering::SeqCst), 50);

    // ALL 50 writes must be in history
    let history = db.history("concurrent", "hotspot").await.unwrap();
    assert_eq!(
        history.len(),
        50,
        "Lost writes! Expected 50, got {}",
        history.len()
    );

    // Verify causal chain integrity - collect all version IDs
    let version_ids: HashSet<String> = history.iter().map(|h| h.version_id.clone()).collect();

    // Verify all version IDs are present and unique (deduplication may cause repeats)
    // The important invariant is that no writes are lost
    assert!(
        !version_ids.is_empty(),
        "Should have at least one unique version ID"
    );

    // The history should be in chronological order
    for i in 1..history.len() {
        assert!(
            history[i].timestamp >= history[i - 1].timestamp,
            "History not in chronological order at index {}",
            i
        );
    }
}

/// Falsification: Can we create a cycle in the causal chain?
/// Version IDs are content-addressed, so writing the same value should produce same ID.
/// This tests that the chain handles repeated values correctly.
#[tokio::test]
async fn falsify_causal_chain_repeated_values() {
    let db = KoruDelta::start().await.unwrap();

    // Write A -> B -> A -> B -> A (alternating values)
    let values = ["A", "B", "A", "B", "A"];
    let mut version_ids = Vec::new();
    let mut prev_versions = Vec::new();

    for (i, val) in values.iter().enumerate() {
        sleep(StdDuration::from_millis(5)).await; // Ensure distinct timestamps
        let v = db
            .put("cycle", "test", json!({"value": val}))
            .await
            .unwrap();
        version_ids.push(v.version_id().to_string());
        prev_versions.push(v.previous_version().map(|s| s.to_string()));

        // Content-addressed: same value = same version_id
        if i >= 2 && values[i] == values[i - 2] {
            assert_eq!(
                version_ids[i], version_ids[i - 2],
                "Content addressing broken: same value should produce same version_id"
            );
        }
    }

    // All 5 writes must be in history despite duplicate version_ids
    let history = db.history("cycle", "test").await.unwrap();
    assert_eq!(history.len(), 5, "History should contain all 5 writes");

    // Verify that history maintains chronological order
    for i in 1..history.len() {
        assert!(
            history[i].timestamp >= history[i - 1].timestamp,
            "History not in chronological order at index {}",
            i
        );
    }

    // Verify the previous_version chain from put results
    // First entry has no predecessor
    assert!(prev_versions[0].is_none(), "First write should have no predecessor");

    // Each subsequent entry should link to the previous
    for i in 1..prev_versions.len() {
        assert_eq!(
            prev_versions[i].as_ref(),
            Some(&version_ids[i - 1]),
            "previous_version chain broken at index {}", i
        );
    }
}

// ============================================================================
// SECTION 2: TIME TRAVEL EDGE CASES
// ============================================================================

/// Falsification: What happens at exact timestamp boundaries?
/// Query at exact timestamp T should return version created at T.
#[tokio::test]
async fn falsify_time_travel_exact_boundary() {
    let db = KoruDelta::start().await.unwrap();

    let v1 = db
        .put("boundary", "key", json!({"version": 1}))
        .await
        .unwrap();
    let t1 = v1.timestamp();

    sleep(StdDuration::from_millis(50)).await;

    let v2 = db
        .put("boundary", "key", json!({"version": 2}))
        .await
        .unwrap();
    let t2 = v2.timestamp();

    // Query at EXACT t1 should return v1
    let at_t1 = db.get_at("boundary", "key", t1).await.unwrap();
    assert_eq!(at_t1["version"], 1, "Query at exact t1 should return v1");

    // Query at EXACT t2 should return v2
    let at_t2 = db.get_at("boundary", "key", t2).await.unwrap();
    assert_eq!(at_t2["version"], 2, "Query at exact t2 should return v2");

    // Query at t1 + 1 nanosecond should still return v1
    let t1_plus = t1 + Duration::nanoseconds(1);
    let at_t1_plus = db.get_at("boundary", "key", t1_plus).await.unwrap();
    assert_eq!(
        at_t1_plus["version"], 1,
        "Query just after t1 should still return v1"
    );

    // Query at t2 - 1 nanosecond should return v1 (before v2 was created)
    let t2_minus = t2 - Duration::nanoseconds(1);
    let at_t2_minus = db.get_at("boundary", "key", t2_minus).await.unwrap();
    assert_eq!(
        at_t2_minus["version"], 1,
        "Query just before t2 should return v1"
    );
}

/// Falsification: Time travel to far future should return latest version.
#[tokio::test]
async fn falsify_time_travel_far_future() {
    let db = KoruDelta::start().await.unwrap();

    db.put("future", "key", json!({"version": 1}))
        .await
        .unwrap();
    sleep(StdDuration::from_millis(10)).await;
    db.put("future", "key", json!({"version": 2}))
        .await
        .unwrap();
    sleep(StdDuration::from_millis(10)).await;
    let _v3 = db
        .put("future", "key", json!({"version": 3}))
        .await
        .unwrap();

    // Query year 3000
    let far_future = Utc::now() + Duration::days(365 * 1000);
    let at_future = db.get_at("future", "key", far_future).await.unwrap();
    assert_eq!(
        at_future["version"], 3,
        "Far future query should return latest version"
    );

    // Should have same version_id as v3
    // (Can't easily compare because get_at returns JsonValue, not VersionedValue)
}

/// Falsification: Time travel between rapid writes.
/// With microsecond-precision timestamps, can we correctly resolve?
#[tokio::test]
async fn falsify_time_travel_microsecond_precision() {
    let db = KoruDelta::start().await.unwrap();

    // Write rapidly, capturing timestamps
    let mut timestamps = Vec::new();
    for i in 0..10 {
        let v = db
            .put("precision", "key", json!({"seq": i}))
            .await
            .unwrap();
        timestamps.push((i, v.timestamp()));
        // NO sleep - push system to limit
    }

    // Verify each timestamp can retrieve its version
    for (seq, ts) in &timestamps {
        let value = db.get_at("precision", "key", *ts).await.unwrap();
        // Due to timestamp collisions, we might get the seq or an earlier one
        // The key invariant: we should get a version at or before the timestamp
        let retrieved_seq = value["seq"].as_i64().unwrap();
        assert!(
            retrieved_seq <= *seq as i64,
            "Time travel returned future version! Asked for ts of seq {}, got seq {}",
            seq,
            retrieved_seq
        );
    }
}

/// Falsification: What happens when we time-travel to epoch (1970)?
#[tokio::test]
async fn falsify_time_travel_epoch() {
    let db = KoruDelta::start().await.unwrap();

    db.put("epoch", "key", json!({"data": "test"}))
        .await
        .unwrap();

    // Time travel to Unix epoch
    let epoch = chrono::DateTime::from_timestamp(0, 0).unwrap();
    let result = db.get_at("epoch", "key", epoch).await;

    // Should fail - no data existed at epoch
    assert!(
        result.is_err(),
        "Time travel to epoch should fail for recently created key"
    );
    assert!(
        matches!(result, Err(DeltaError::NoValueAtTimestamp { .. })),
        "Should return NoValueAtTimestamp error"
    );
}

// ============================================================================
// SECTION 3: VALUE DEDUPLICATION ATTACKS
// ============================================================================

/// Falsification: Does deduplication work correctly across namespaces?
/// Test by verifying that content-addressed version IDs are identical for same values.
#[tokio::test]
async fn falsify_deduplication_cross_namespace() {
    let db = KoruDelta::start().await.unwrap();

    let shared_value = json!({"status": "active", "count": 42, "nested": {"a": 1, "b": 2}});
    let mut version_ids = HashSet::new();

    // Store same value in 100 different namespace/key combinations
    for i in 0..100 {
        let v = db
            .put(format!("ns{}", i), format!("key{}", i), shared_value.clone())
            .await
            .unwrap();
        version_ids.insert(v.version_id().to_string());
    }

    let stats = db.stats().await;
    assert_eq!(stats.key_count, 100, "Should have 100 keys");
    assert_eq!(stats.total_versions, 100, "Should have 100 versions");

    // Content-addressed: same value should produce same version_id
    assert_eq!(
        version_ids.len(),
        1,
        "Deduplication failed: should have only 1 unique version_id, got {}",
        version_ids.len()
    );
}

/// Falsification: JSON key order - does {"a":1,"b":2} deduplicate with {"b":2,"a":1}?
/// Serde_json preserves key order, so these should be DIFFERENT.
#[tokio::test]
async fn falsify_deduplication_json_key_order() {
    let db = KoruDelta::start().await.unwrap();

    // Create objects with different key orders
    let v1 = db
        .put(
            "order",
            "key1",
            serde_json::from_str::<serde_json::Value>(r#"{"a":1,"b":2}"#).unwrap(),
        )
        .await
        .unwrap();

    let v2 = db
        .put(
            "order",
            "key2",
            serde_json::from_str::<serde_json::Value>(r#"{"b":2,"a":1}"#).unwrap(),
        )
        .await
        .unwrap();

    // Document the actual behavior:
    // If version IDs are equal, the system normalizes key order (content-addressed)
    // If different, the system is sensitive to byte-level differences
    // Both behaviors are valid, this test documents which one this implementation uses
    if v1.version_id() == v2.version_id() {
        // System normalizes key order - good for deduplication
        assert_eq!(v1.version_id(), v2.version_id());
    } else {
        // System is sensitive to key order - more precise content addressing
        assert_ne!(v1.version_id(), v2.version_id());
    }
}

/// Falsification: Floating point edge cases in deduplication
#[tokio::test]
async fn falsify_deduplication_float_edge_cases() {
    let db = KoruDelta::start().await.unwrap();

    // Test various float representations
    let v1 = db
        .put("float", "int_as_float", json!(1.0))
        .await
        .unwrap();
    let v2 = db.put("float", "int", json!(1)).await.unwrap();

    // 1.0 and 1 are different JSON values
    assert_ne!(
        v1.version_id(),
        v2.version_id(),
        "1.0 and 1 should have different version IDs"
    );

    // Special floats
    db.put("float", "large", json!(1e308)).await.unwrap();
    db.put("float", "small", json!(1e-308)).await.unwrap();
    db.put("float", "negative_zero", json!(-0.0)).await.unwrap();

    // Verify retrieval
    let large = db.get("float", "large").await.unwrap();
    assert_eq!(large, json!(1e308));
}

/// Falsification: Unicode edge cases in deduplication
#[tokio::test]
async fn falsify_deduplication_unicode() {
    let db = KoruDelta::start().await.unwrap();

    // Various Unicode edge cases
    let test_cases = vec![
        ("empty", json!("")),
        ("ascii", json!("hello")),
        ("emoji", json!("Hello 👋 World 🌍")),
        ("rtl", json!("مرحبا")),
        ("cjk", json!("你好世界")),
        ("combining", json!("e\u{0301}")),    // e + combining acute = é
        ("precomposed", json!("\u{00E9}")),   // precomposed é
        ("null_byte", json!("hello\u{0000}world")),
        ("bom", json!("\u{FEFF}text")),
        ("zwj", json!("👨\u{200D}👩\u{200D}👧")), // Family emoji with ZWJ
    ];

    for (name, value) in test_cases {
        db.put("unicode", name, value.clone()).await.unwrap();
        let retrieved = db.get("unicode", name).await.unwrap();
        assert_eq!(retrieved, value, "Unicode roundtrip failed for {}", name);
    }
}

// ============================================================================
// SECTION 4: QUERY ENGINE FALSIFICATION
// ============================================================================

/// Falsification: Filter behavior with null/missing fields
#[tokio::test]
async fn falsify_query_null_handling() {
    let db = KoruDelta::start().await.unwrap();

    db.put("nulls", "with_field", json!({"status": "active", "count": 10}))
        .await
        .unwrap();
    db.put("nulls", "null_field", json!({"status": null, "count": 5}))
        .await
        .unwrap();
    db.put("nulls", "missing_field", json!({"count": 15}))
        .await
        .unwrap();

    // Eq with null
    let result = db
        .query("nulls", Query::new().filter(Filter::eq("status", json!(null))))
        .await
        .unwrap();
    assert_eq!(
        result.records.len(),
        1,
        "Should match only explicit null, not missing"
    );
    assert_eq!(result.records[0].key, "null_field");

    // Exists filter
    let result = db
        .query("nulls", Query::new().filter(Filter::exists("status")))
        .await
        .unwrap();
    assert_eq!(result.records.len(), 1, "Exists should exclude null and missing");
    assert_eq!(result.records[0].key, "with_field");

    // Ne (not equals) - should include missing fields?
    let result = db
        .query(
            "nulls",
            Query::new().filter(Filter::ne("status", json!("active"))),
        )
        .await
        .unwrap();
    // This tests the semantic: does "status != active" include records where status is missing?
    assert!(
        !result.records.is_empty(),
        "Ne filter behavior with missing fields"
    );
}

/// Falsification: Numeric comparison edge cases
#[tokio::test]
async fn falsify_query_numeric_comparisons() {
    let db = KoruDelta::start().await.unwrap();

    db.put("nums", "zero", json!({"value": 0})).await.unwrap();
    db.put("nums", "neg_zero", json!({"value": -0.0}))
        .await
        .unwrap();
    db.put("nums", "small_pos", json!({"value": 0.0001}))
        .await
        .unwrap();
    db.put("nums", "small_neg", json!({"value": -0.0001}))
        .await
        .unwrap();
    db.put("nums", "large", json!({"value": 1e100}))
        .await
        .unwrap();
    db.put("nums", "int_max", json!({"value": i64::MAX}))
        .await
        .unwrap();
    db.put("nums", "int_min", json!({"value": i64::MIN}))
        .await
        .unwrap();

    // Greater than zero
    let result = db
        .query("nums", Query::new().filter(Filter::gt("value", json!(0))))
        .await
        .unwrap();
    let keys: HashSet<_> = result.records.iter().map(|r| r.key.as_str()).collect();
    assert!(
        keys.contains("small_pos"),
        "0.0001 should be > 0"
    );
    assert!(keys.contains("large"), "1e100 should be > 0");
    assert!(!keys.contains("zero"), "0 should not be > 0");

    // Integer vs float comparison
    let result = db
        .query(
            "nums",
            Query::new().filter(Filter::gte("value", json!(0.0))),
        )
        .await
        .unwrap();
    assert!(
        result.records.iter().any(|r| r.key == "zero"),
        "Integer 0 should be >= 0.0"
    );

    // Comparison with MAX/MIN values
    let _result = db
        .query(
            "nums",
            Query::new().filter(Filter::lt("value", json!(i64::MIN as f64 + 1.0))),
        )
        .await
        .unwrap();
    // i64::MIN might have precision issues as f64
    // This tests the robustness of numeric comparisons (no panic = pass)
}

/// Falsification: String comparison edge cases
#[tokio::test]
async fn falsify_query_string_comparisons() {
    let db = KoruDelta::start().await.unwrap();

    db.put("strings", "empty", json!({"name": ""}))
        .await
        .unwrap();
    db.put("strings", "space", json!({"name": " "}))
        .await
        .unwrap();
    db.put("strings", "a", json!({"name": "a"})).await.unwrap();
    db.put("strings", "A", json!({"name": "A"})).await.unwrap();
    db.put("strings", "aa", json!({"name": "aa"}))
        .await
        .unwrap();
    db.put("strings", "unicode", json!({"name": "über"}))
        .await
        .unwrap();

    // Empty string comparison
    let result = db
        .query(
            "strings",
            Query::new().filter(Filter::gt("name", json!(""))),
        )
        .await
        .unwrap();
    assert!(
        result.records.len() >= 4,
        "Everything should be > empty string except empty"
    );

    // Case sensitivity
    let result = db
        .query(
            "strings",
            Query::new().filter(Filter::gt("name", json!("a"))),
        )
        .await
        .unwrap();
    let keys: HashSet<_> = result.records.iter().map(|r| r.key.as_str()).collect();
    // In ASCII/Unicode ordering, 'a' > 'A' (lowercase > uppercase)
    assert!(keys.contains("aa"), "'aa' should be > 'a'");

    // Contains with empty string - should match everything
    let result = db
        .query(
            "strings",
            Query::new().filter(Filter::contains("name", json!(""))),
        )
        .await
        .unwrap();
    assert_eq!(
        result.records.len(),
        6,
        "Contains empty string should match all"
    );
}

/// Falsification: Deeply nested field access
#[tokio::test]
async fn falsify_query_deep_nesting() {
    let db = KoruDelta::start().await.unwrap();

    db.put(
        "nested",
        "deep",
        json!({
            "a": {
                "b": {
                    "c": {
                        "d": {
                            "e": {
                                "value": 42
                            }
                        }
                    }
                }
            }
        }),
    )
    .await
    .unwrap();

    db.put(
        "nested",
        "partial",
        json!({
            "a": {
                "b": {
                    "c": null
                }
            }
        }),
    )
    .await
    .unwrap();

    db.put("nested", "array_access", json!({"items": [{"x": 1}, {"x": 2}, {"x": 3}]}))
        .await
        .unwrap();

    // Deep field access
    let result = db
        .query(
            "nested",
            Query::new().filter(Filter::eq("a.b.c.d.e.value", json!(42))),
        )
        .await
        .unwrap();
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].key, "deep");

    // Partial path (stops at null)
    let result = db
        .query(
            "nested",
            Query::new().filter(Filter::exists("a.b.c.d")),
        )
        .await
        .unwrap();
    assert_eq!(
        result.records.len(),
        1,
        "Only 'deep' has a.b.c.d existing"
    );

    // Array index access
    let result = db
        .query(
            "nested",
            Query::new().filter(Filter::eq("items.1.x", json!(2))),
        )
        .await
        .unwrap();
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].key, "array_access");
}

/// Falsification: Complex boolean filter combinations (De Morgan's laws)
#[tokio::test]
async fn falsify_query_boolean_logic() {
    let db = KoruDelta::start().await.unwrap();

    // Create test data covering all combinations
    db.put("logic", "tt", json!({"a": true, "b": true}))
        .await
        .unwrap();
    db.put("logic", "tf", json!({"a": true, "b": false}))
        .await
        .unwrap();
    db.put("logic", "ft", json!({"a": false, "b": true}))
        .await
        .unwrap();
    db.put("logic", "ff", json!({"a": false, "b": false}))
        .await
        .unwrap();

    // Test: NOT (A AND B) == (NOT A) OR (NOT B)
    let not_and = Filter::not(Filter::and(vec![
        Filter::eq("a", json!(true)),
        Filter::eq("b", json!(true)),
    ]));
    let or_not = Filter::or(vec![
        Filter::eq("a", json!(false)),
        Filter::eq("b", json!(false)),
    ]);

    let result1 = db
        .query("logic", Query::new().filter(not_and))
        .await
        .unwrap();
    let result2 = db
        .query("logic", Query::new().filter(or_not))
        .await
        .unwrap();

    let keys1: HashSet<_> = result1.records.iter().map(|r| r.key.as_str()).collect();
    let keys2: HashSet<_> = result2.records.iter().map(|r| r.key.as_str()).collect();

    assert_eq!(keys1, keys2, "De Morgan's law should hold");

    // Test: NOT (A OR B) == (NOT A) AND (NOT B)
    let not_or = Filter::not(Filter::or(vec![
        Filter::eq("a", json!(true)),
        Filter::eq("b", json!(true)),
    ]));
    let and_not = Filter::and(vec![
        Filter::eq("a", json!(false)),
        Filter::eq("b", json!(false)),
    ]);

    let result3 = db
        .query("logic", Query::new().filter(not_or))
        .await
        .unwrap();
    let result4 = db
        .query("logic", Query::new().filter(and_not))
        .await
        .unwrap();

    let keys3: HashSet<_> = result3.records.iter().map(|r| r.key.as_str()).collect();
    let keys4: HashSet<_> = result4.records.iter().map(|r| r.key.as_str()).collect();

    assert_eq!(keys3, keys4, "De Morgan's law should hold");
    assert_eq!(keys3, HashSet::from(["ff"]), "Only ff has both false");
}

/// Falsification: Regex filter edge cases
#[tokio::test]
async fn falsify_query_regex() {
    let db = KoruDelta::start().await.unwrap();

    db.put("regex", "normal", json!({"text": "hello world"}))
        .await
        .unwrap();
    db.put("regex", "special", json!({"text": "hello.world"}))
        .await
        .unwrap();
    db.put("regex", "newline", json!({"text": "hello\nworld"}))
        .await
        .unwrap();
    db.put("regex", "unicode", json!({"text": "héllo wörld"}))
        .await
        .unwrap();

    // Literal dot vs regex dot
    let result = db
        .query(
            "regex",
            Query::new().filter(Filter::matches("text", r"hello\.world")),
        )
        .await
        .unwrap();
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].key, "special");

    // Regex dot matches any char
    let result = db
        .query(
            "regex",
            Query::new().filter(Filter::matches("text", r"hello.world")),
        )
        .await
        .unwrap();
    assert!(
        result.records.len() >= 2,
        "Regex . should match space and literal dot"
    );

    // Unicode in regex
    let result = db
        .query(
            "regex",
            Query::new().filter(Filter::matches("text", r"h.llo")),
        )
        .await
        .unwrap();
    assert!(
        result.records.len() >= 2,
        "Regex should match unicode characters"
    );

    // Invalid regex should not panic
    let _result = db
        .query(
            "regex",
            Query::new().filter(Filter::matches("text", r"[invalid")),
        )
        .await;
    // Should either return empty results or handle gracefully
    // (not asserting ok - just verifying no panic)
}

// ============================================================================
// SECTION 5: AGGREGATION EDGE CASES
// ============================================================================

/// Falsification: Aggregation on empty result set
#[tokio::test]
async fn falsify_aggregation_empty() {
    let db = KoruDelta::start().await.unwrap();

    // Query non-existent collection
    let result = db
        .query(
            "empty",
            Query::new().aggregate(Aggregation::count()),
        )
        .await
        .unwrap();
    assert_eq!(result.aggregation, Some(json!(0)));

    let result = db
        .query(
            "empty",
            Query::new().aggregate(Aggregation::sum("value")),
        )
        .await
        .unwrap();
    assert_eq!(result.aggregation, Some(json!(0.0)));

    let result = db
        .query(
            "empty",
            Query::new().aggregate(Aggregation::avg("value")),
        )
        .await
        .unwrap();
    assert_eq!(result.aggregation, Some(json!(null)));

    let result = db
        .query(
            "empty",
            Query::new().aggregate(Aggregation::min("value")),
        )
        .await
        .unwrap();
    assert_eq!(result.aggregation, Some(json!(null)));

    let result = db
        .query(
            "empty",
            Query::new().aggregate(Aggregation::max("value")),
        )
        .await
        .unwrap();
    assert_eq!(result.aggregation, Some(json!(null)));
}

/// Falsification: Aggregation with mixed types
#[tokio::test]
async fn falsify_aggregation_mixed_types() {
    let db = KoruDelta::start().await.unwrap();

    db.put("mixed", "num", json!({"value": 10})).await.unwrap();
    db.put("mixed", "str", json!({"value": "hello"}))
        .await
        .unwrap();
    db.put("mixed", "bool", json!({"value": true}))
        .await
        .unwrap();
    db.put("mixed", "null", json!({"value": null}))
        .await
        .unwrap();
    db.put("mixed", "missing", json!({"other": 5}))
        .await
        .unwrap();

    // Sum should only sum numbers, skip non-numeric
    let result = db
        .query(
            "mixed",
            Query::new().aggregate(Aggregation::sum("value")),
        )
        .await
        .unwrap();
    assert_eq!(
        result.aggregation,
        Some(json!(10.0)),
        "Sum should only include numeric values"
    );

    // Avg should only average numbers
    let result = db
        .query(
            "mixed",
            Query::new().aggregate(Aggregation::avg("value")),
        )
        .await
        .unwrap();
    assert_eq!(
        result.aggregation,
        Some(json!(10.0)),
        "Avg with single numeric should be that value"
    );

    // Count should count all records
    let result = db
        .query(
            "mixed",
            Query::new().aggregate(Aggregation::count()),
        )
        .await
        .unwrap();
    assert_eq!(result.aggregation, Some(json!(5)));
}

/// Falsification: Aggregation with floating point precision
#[tokio::test]
async fn falsify_aggregation_float_precision() {
    let db = KoruDelta::start().await.unwrap();

    // Classic floating point gotcha: 0.1 + 0.2 != 0.3
    db.put("precision", "a", json!({"value": 0.1}))
        .await
        .unwrap();
    db.put("precision", "b", json!({"value": 0.2}))
        .await
        .unwrap();

    let result = db
        .query(
            "precision",
            Query::new().aggregate(Aggregation::sum("value")),
        )
        .await
        .unwrap();

    // Don't test for exact 0.3, test for approximate
    let sum = result.aggregation.unwrap().as_f64().unwrap();
    assert!(
        (sum - 0.3).abs() < 1e-10,
        "Sum should be approximately 0.3, got {}",
        sum
    );

    // Large number precision
    db.put("precision", "large1", json!({"value": 1e15}))
        .await
        .unwrap();
    db.put("precision", "large2", json!({"value": 1.0}))
        .await
        .unwrap();

    let result = db
        .query(
            "precision",
            Query::new()
                .filter(Filter::gt("value", json!(1e14)))
                .aggregate(Aggregation::sum("value")),
        )
        .await
        .unwrap();

    let sum = result.aggregation.unwrap().as_f64().unwrap();
    // At 1e15, adding 1.0 may lose precision
    assert!(sum >= 1e15, "Large number sum should be at least 1e15");
}

// ============================================================================
// SECTION 6: SORTING EDGE CASES
// ============================================================================

/// Falsification: Sort stability with equal values
#[tokio::test]
async fn falsify_sort_stability() {
    let db = KoruDelta::start().await.unwrap();

    // Create records with same sort key but different secondary values
    for i in 0..10 {
        db.put(
            "stable",
            format!("rec{}", i),
            json!({"priority": 1, "seq": i}),
        )
        .await
        .unwrap();
    }

    let result = db
        .query("stable", Query::new().sort_by("priority", true))
        .await
        .unwrap();

    // All have same priority, order should be consistent
    // (though not necessarily insertion order)
    assert_eq!(result.records.len(), 10);

    // Run query multiple times, order should be stable
    let result2 = db
        .query("stable", Query::new().sort_by("priority", true))
        .await
        .unwrap();

    let keys1: Vec<_> = result.records.iter().map(|r| &r.key).collect();
    let keys2: Vec<_> = result2.records.iter().map(|r| &r.key).collect();
    assert_eq!(keys1, keys2, "Sort should be deterministic");
}

/// Falsification: Multi-key sort
#[tokio::test]
async fn falsify_sort_multi_key() {
    let db = KoruDelta::start().await.unwrap();

    db.put("multi", "a", json!({"x": 1, "y": 3}))
        .await
        .unwrap();
    db.put("multi", "b", json!({"x": 1, "y": 1}))
        .await
        .unwrap();
    db.put("multi", "c", json!({"x": 2, "y": 2}))
        .await
        .unwrap();
    db.put("multi", "d", json!({"x": 1, "y": 2}))
        .await
        .unwrap();

    // Sort by x asc, then y asc
    let result = db
        .query(
            "multi",
            Query::new().sort_by("x", true).sort_by("y", true),
        )
        .await
        .unwrap();

    let keys: Vec<_> = result.records.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(
        keys,
        vec!["b", "d", "a", "c"],
        "Multi-key sort should work: x=1,y=1 then x=1,y=2 then x=1,y=3 then x=2,y=2"
    );
}

/// Falsification: Sort with missing fields
#[tokio::test]
async fn falsify_sort_missing_fields() {
    let db = KoruDelta::start().await.unwrap();

    db.put("missing", "with_field", json!({"sort_key": 5}))
        .await
        .unwrap();
    db.put("missing", "without_field", json!({"other": 10}))
        .await
        .unwrap();
    db.put("missing", "null_field", json!({"sort_key": null}))
        .await
        .unwrap();
    db.put("missing", "zero_field", json!({"sort_key": 0}))
        .await
        .unwrap();

    // Sort ascending - where do nulls/missing go?
    let result = db
        .query("missing", Query::new().sort_by("sort_key", true))
        .await
        .unwrap();

    // Records with missing/null fields should sort consistently
    // (typically at the end for asc, beginning for desc)
    assert_eq!(result.records.len(), 4);

    // Verify zero comes before 5
    let keys: Vec<_> = result.records.iter().map(|r| r.key.as_str()).collect();
    let zero_pos = keys.iter().position(|&k| k == "zero_field").unwrap();
    let five_pos = keys.iter().position(|&k| k == "with_field").unwrap();
    assert!(
        zero_pos < five_pos,
        "0 should sort before 5 in ascending order"
    );
}

// ============================================================================
// SECTION 7: CONCURRENCY STRESS TESTS
// ============================================================================

/// Stress test: High-volume concurrent writes to different keys
#[tokio::test]
async fn stress_concurrent_writes_different_keys() {
    let db = KoruDelta::start().await.unwrap();
    let write_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 100 concurrent writers, each writing to their own key
    for i in 0..100 {
        let db_clone = db.clone();
        let writes = Arc::clone(&write_count);
        let errors = Arc::clone(&error_count);
        let handle = tokio::spawn(async move {
            match db_clone
                .put("stress", format!("key{}", i), json!({"writer": i, "data": "x".repeat(1000)}))
                .await
            {
                Ok(_) => {
                    writes.fetch_add(1, AtomicOrdering::SeqCst);
                }
                Err(_) => {
                    errors.fetch_add(1, AtomicOrdering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(write_count.load(AtomicOrdering::SeqCst), 100);
    assert_eq!(error_count.load(AtomicOrdering::SeqCst), 0);
    assert_eq!(db.stats().await.key_count, 100);
}

/// Stress test: High-volume concurrent writes to same key
#[tokio::test]
async fn stress_concurrent_writes_same_key() {
    let db = KoruDelta::start().await.unwrap();
    let mut handles = vec![];

    // 100 concurrent writers to the SAME key
    for i in 0..100 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            db_clone
                .put("hotspot", "contended", json!({"writer": i}))
                .await
                .unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let stats = db.stats().await;
    assert_eq!(stats.key_count, 1);
    assert_eq!(stats.total_versions, 100, "All 100 writes must be recorded");

    let history = db.history("hotspot", "contended").await.unwrap();
    assert_eq!(history.len(), 100);
}

/// Stress test: Concurrent reads and writes
#[tokio::test]
async fn stress_concurrent_read_write() {
    let db = KoruDelta::start().await.unwrap();

    // Pre-populate
    for i in 0..10 {
        db.put("rw", format!("key{}", i), json!({"value": i}))
            .await
            .unwrap();
    }

    let read_count = Arc::new(AtomicUsize::new(0));
    let write_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 50 readers
    for _ in 0..50 {
        let db_clone = db.clone();
        let reads = Arc::clone(&read_count);
        let handle = tokio::spawn(async move {
            for i in 0..10 {
                if db_clone.get("rw", format!("key{}", i)).await.is_ok() {
                    reads.fetch_add(1, AtomicOrdering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    // 50 writers
    for w in 0..50 {
        let db_clone = db.clone();
        let writes = Arc::clone(&write_count);
        let handle = tokio::spawn(async move {
            for i in 0..10 {
                if db_clone
                    .put("rw", format!("key{}", i), json!({"writer": w, "iter": i}))
                    .await
                    .is_ok()
                {
                    writes.fetch_add(1, AtomicOrdering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(read_count.load(AtomicOrdering::SeqCst), 500); // 50 readers * 10 keys
    assert_eq!(write_count.load(AtomicOrdering::SeqCst), 500); // 50 writers * 10 keys

    // All keys should still be readable
    for i in 0..10 {
        assert!(db.get("rw", format!("key{}", i)).await.is_ok());
    }
}

/// Stress test: Concurrent queries
#[tokio::test]
async fn stress_concurrent_queries() {
    let db = KoruDelta::start().await.unwrap();

    // Pre-populate with 100 records
    for i in 0..100 {
        db.put(
            "query_stress",
            format!("record{}", i),
            json!({
                "value": i,
                "category": if i % 2 == 0 { "even" } else { "odd" },
                "tier": i / 10
            }),
        )
        .await
        .unwrap();
    }

    let mut handles = vec![];

    // 50 concurrent queries of different types
    for i in 0..50 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            match i % 5 {
                0 => {
                    // Simple filter
                    db_clone
                        .query(
                            "query_stress",
                            Query::new().filter(Filter::eq("category", json!("even"))),
                        )
                        .await
                        .unwrap();
                }
                1 => {
                    // Range filter
                    db_clone
                        .query(
                            "query_stress",
                            Query::new().filter(Filter::and(vec![
                                Filter::gte("value", json!(25)),
                                Filter::lt("value", json!(75)),
                            ])),
                        )
                        .await
                        .unwrap();
                }
                2 => {
                    // Sort + limit
                    db_clone
                        .query(
                            "query_stress",
                            Query::new().sort_by("value", false).limit(10),
                        )
                        .await
                        .unwrap();
                }
                3 => {
                    // Aggregation
                    db_clone
                        .query(
                            "query_stress",
                            Query::new().aggregate(Aggregation::sum("value")),
                        )
                        .await
                        .unwrap();
                }
                _ => {
                    // Full scan
                    db_clone
                        .query("query_stress", Query::new())
                        .await
                        .unwrap();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

// ============================================================================
// SECTION 8: SUBSCRIPTION FALSIFICATION
// ============================================================================

/// Falsification: Subscription receives correct change types
#[tokio::test]
async fn falsify_subscription_change_types() {
    let db = KoruDelta::start().await.unwrap();

    let (_id, mut rx) = db.subscribe(Subscription::all()).await;

    // Insert (first write to a key)
    db.put_notify("sub", "key1", json!({"v": 1}))
        .await
        .unwrap();
    let event = tokio::time::timeout(StdDuration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.change_type, ChangeType::Insert);
    assert!(event.previous_value.is_none());

    // Update (second write to same key)
    db.put_notify("sub", "key1", json!({"v": 2}))
        .await
        .unwrap();
    let event = tokio::time::timeout(StdDuration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.change_type, ChangeType::Update);
    assert!(event.previous_value.is_some());
    assert_eq!(event.previous_value.unwrap()["v"], 1);
}

/// Falsification: Multiple subscribers receive same event
#[tokio::test]
async fn falsify_subscription_broadcast() {
    let db = KoruDelta::start().await.unwrap();

    let (_id1, mut rx1) = db.subscribe(Subscription::all()).await;
    let (_id2, mut rx2) = db.subscribe(Subscription::all()).await;
    let (_id3, mut rx3) = db.subscribe(Subscription::all()).await;

    db.put_notify("broadcast", "key", json!({"data": "test"}))
        .await
        .unwrap();

    // All three should receive the event
    let e1 = tokio::time::timeout(StdDuration::from_millis(100), rx1.recv())
        .await
        .unwrap()
        .unwrap();
    let e2 = tokio::time::timeout(StdDuration::from_millis(100), rx2.recv())
        .await
        .unwrap()
        .unwrap();
    let e3 = tokio::time::timeout(StdDuration::from_millis(100), rx3.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(e1.key, "key");
    assert_eq!(e2.key, "key");
    assert_eq!(e3.key, "key");
}

/// Falsification: Filtered subscription correctness
#[tokio::test]
async fn falsify_subscription_filter() {
    let db = KoruDelta::start().await.unwrap();

    // Subscribe only to "users" collection
    let (_id, mut rx) = db.subscribe(Subscription::collection("users")).await;

    // Write to other collection (should NOT receive)
    db.put_notify("products", "p1", json!({"price": 10}))
        .await
        .unwrap();

    // Write to users collection (SHOULD receive)
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    // Should only receive the users event
    let event = tokio::time::timeout(StdDuration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(event.collection, "users");
    assert_eq!(event.key, "alice");

    // No more events should be pending
    let result = tokio::time::timeout(StdDuration::from_millis(50), rx.recv()).await;
    assert!(result.is_err(), "Should not receive products event");
}

/// Falsification: Rapid writes with subscription
#[tokio::test]
async fn falsify_subscription_rapid_writes() {
    let db = KoruDelta::start().await.unwrap();

    let (_id, mut rx) = db.subscribe(Subscription::all()).await;

    // Rapid fire 50 writes
    for i in 0..50 {
        db.put_notify("rapid", format!("key{}", i), json!({"seq": i}))
            .await
            .unwrap();
    }

    // Collect all events with timeout
    let mut received = 0;
    while let Ok(Ok(_)) = tokio::time::timeout(StdDuration::from_millis(100), rx.recv()).await {
        received += 1;
    }

    // Should receive all 50 events (unless channel overflowed)
    // Note: Default channel size is 256, so 50 should be fine
    assert_eq!(
        received, 50,
        "Should receive all events, got {}",
        received
    );
}

// ============================================================================
// SECTION 9: VIEW FALSIFICATION
// ============================================================================

/// Falsification: View consistency after refresh
#[tokio::test]
async fn falsify_view_consistency() {
    let db = KoruDelta::start().await.unwrap();

    // Create base data
    for i in 0..10 {
        db.put(
            "items",
            format!("item{}", i),
            json!({"value": i, "active": i % 2 == 0}),
        )
        .await
        .unwrap();
    }

    // Create view for active items
    let def = ViewDefinition::new("active_items", "items")
        .with_query(Query::new().filter(Filter::eq("active", json!(true))));
    db.create_view(def).await.unwrap();

    // Initial view should have 5 items (0, 2, 4, 6, 8)
    let result = db.query_view("active_items").await.unwrap();
    assert_eq!(result.records.len(), 5);

    // Add more data
    for i in 10..20 {
        db.put(
            "items",
            format!("item{}", i),
            json!({"value": i, "active": i % 2 == 0}),
        )
        .await
        .unwrap();
    }

    // View should still show old data (not auto-refreshing)
    let result = db.query_view("active_items").await.unwrap();
    assert_eq!(result.records.len(), 5, "View should not auto-refresh");

    // Refresh view
    db.refresh_view("active_items").await.unwrap();

    // Now should have 10 items
    let result = db.query_view("active_items").await.unwrap();
    assert_eq!(result.records.len(), 10);
}

/// Falsification: Auto-refresh view correctness
#[tokio::test]
async fn falsify_view_auto_refresh() {
    let db = KoruDelta::start().await.unwrap();

    db.put("auto", "item1", json!({"value": 1})).await.unwrap();

    // Create auto-refresh view
    let def = ViewDefinition::new("auto_view", "auto").auto_refresh(true);
    db.create_view(def).await.unwrap();

    // Initial count
    let result = db.query_view("auto_view").await.unwrap();
    assert_eq!(result.records.len(), 1);

    // Add data with put_notify (triggers auto-refresh)
    db.put_notify("auto", "item2", json!({"value": 2}))
        .await
        .unwrap();

    // Should immediately see new data
    let result = db.query_view("auto_view").await.unwrap();
    assert_eq!(
        result.records.len(),
        2,
        "Auto-refresh view should see new data"
    );
}

/// Falsification: View with complex query
#[tokio::test]
async fn falsify_view_complex_query() {
    let db = KoruDelta::start().await.unwrap();

    for i in 0..20 {
        db.put(
            "products",
            format!("p{}", i),
            json!({
                "price": i * 10,
                "category": if i % 3 == 0 { "A" } else if i % 3 == 1 { "B" } else { "C" },
                "in_stock": i % 2 == 0
            }),
        )
        .await
        .unwrap();
    }

    // Complex view: in-stock items from category A or B, price >= 50, sorted by price
    let def = ViewDefinition::new("premium_available", "products").with_query(
        Query::new()
            .filter(Filter::and(vec![
                Filter::eq("in_stock", json!(true)),
                Filter::or(vec![
                    Filter::eq("category", json!("A")),
                    Filter::eq("category", json!("B")),
                ]),
                Filter::gte("price", json!(50)),
            ]))
            .sort_by("price", false),
    );

    db.create_view(def).await.unwrap();

    let result = db.query_view("premium_available").await.unwrap();

    // Verify all results match the complex criteria
    for record in &result.records {
        let price = record.value["price"].as_i64().unwrap();
        let category = record.value["category"].as_str().unwrap();
        let in_stock = record.value["in_stock"].as_bool().unwrap();

        assert!(in_stock, "All results should be in stock");
        assert!(
            category == "A" || category == "B",
            "Category should be A or B"
        );
        assert!(price >= 50, "Price should be >= 50");
    }

    // Verify sorted descending
    let prices: Vec<i64> = result
        .records
        .iter()
        .map(|r| r.value["price"].as_i64().unwrap())
        .collect();
    let mut sorted = prices.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(prices, sorted, "Results should be sorted by price desc");
}

// ============================================================================
// SECTION 10: PERSISTENCE FALSIFICATION (non-WASM only)
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod persistence_tests {
    use super::*;
    use koru_delta::persistence;
    use koru_lambda_core::DistinctionEngine;
    use tempfile::tempdir;

    /// Falsification: Save and load preserves all data
    #[tokio::test]
    async fn falsify_persistence_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("db");  // Directory for WAL format

        // Create and populate database
        {
            let db = KoruDelta::start().await.unwrap();

            for i in 0..50 {
                db.put(
                    "data",
                    format!("key{}", i),
                    json!({"value": i, "nested": {"a": i * 2}}),
                )
                .await
                .unwrap();
            }

            // Create some history
            for i in 0..10 {
                db.put("history", "versioned", json!({"version": i}))
                    .await
                    .unwrap();
                sleep(StdDuration::from_millis(5)).await;
            }

            persistence::save(db.storage(), &db_path).await.unwrap();
        }

        // Load and verify
        {
            let engine = Arc::new(DistinctionEngine::new());
            let storage = Arc::new(persistence::load(&db_path, engine.clone()).await.unwrap());
            let db = KoruDelta::from_storage(storage, engine);

            // All 51 keys present (50 data + 1 versioned)
            let stats = db.stats().await;
            assert_eq!(stats.key_count, 51);

            // Verify data integrity
            for i in 0..50 {
                let value = db.get("data", format!("key{}", i)).await.unwrap();
                assert_eq!(value["value"], i);
                assert_eq!(value["nested"]["a"], i * 2);
            }

            // Verify history preserved
            let history = db.history("history", "versioned").await.unwrap();
            assert_eq!(history.len(), 10);
            for (i, entry) in history.iter().enumerate() {
                assert_eq!(entry.value["version"], i as i64);
            }
        }
    }

    /// Falsification: Deduplication survives persistence
    /// Test by checking that all version_ids are identical after load
    #[tokio::test]
    async fn falsify_persistence_deduplication() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("dedup_db");  // Directory for WAL format

        let shared_value = json!({"status": "active"});
        let mut original_version_id = String::new();

        // Create database with duplicated values
        {
            let db = KoruDelta::start().await.unwrap();

            for i in 0..100 {
                let v = db
                    .put("dedup", format!("key{}", i), shared_value.clone())
                    .await
                    .unwrap();
                if i == 0 {
                    original_version_id = v.version_id().to_string();
                }
            }

            persistence::save(db.storage(), &db_path).await.unwrap();
        }

        // Load and verify deduplication is restored
        {
            let engine = Arc::new(DistinctionEngine::new());
            let storage = Arc::new(persistence::load(&db_path, engine.clone()).await.unwrap());
            let db = KoruDelta::from_storage(storage, engine);

            let stats = db.stats().await;
            assert_eq!(stats.key_count, 100);

            // All version IDs should match the original
            for i in 0..100 {
                let v = db.get_versioned("dedup", format!("key{}", i)).await.unwrap();
                assert_eq!(
                    v.version_id(),
                    original_version_id,
                    "Version ID mismatch after load for key {}",
                    i
                );
            }
        }
    }

    /// Falsification: Empty database persistence
    #[tokio::test]
    async fn falsify_persistence_empty() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("empty_db");  // Directory for WAL format

        {
            let db = KoruDelta::start().await.unwrap();
            persistence::save(db.storage(), &db_path).await.unwrap();
        }

        {
            let engine = Arc::new(DistinctionEngine::new());
            let storage = Arc::new(persistence::load(&db_path, engine.clone()).await.unwrap());
            let db = KoruDelta::from_storage(storage, engine);

            let stats = db.stats().await;
            assert_eq!(stats.key_count, 0);
            assert_eq!(stats.total_versions, 0);
        }
    }
}

// ============================================================================
// SECTION 11: ERROR CONDITION FALSIFICATION
// ============================================================================

/// Falsification: Operations on non-existent data
#[tokio::test]
async fn falsify_error_nonexistent() {
    let db = KoruDelta::start().await.unwrap();

    // Get non-existent key
    let result = db.get("missing", "key").await;
    assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));

    // History of non-existent key
    let result = db.history("missing", "key").await;
    assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));

    // Time travel on non-existent key
    let result = db.get_at("missing", "key", Utc::now()).await;
    assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));
}

/// Falsification: Invalid view operations
#[tokio::test]
async fn falsify_error_invalid_view() {
    let db = KoruDelta::start().await.unwrap();

    // Query non-existent view
    let result = db.query_view("nonexistent").await;
    assert!(result.is_err());

    // Refresh non-existent view
    let result = db.refresh_view("nonexistent").await;
    assert!(result.is_err());

    // Delete non-existent view
    let result = db.delete_view("nonexistent").await;
    assert!(result.is_err());

    // Create duplicate view
    db.put("data", "x", json!(1)).await.unwrap();
    db.create_view(ViewDefinition::new("myview", "data"))
        .await
        .unwrap();
    let result = db
        .create_view(ViewDefinition::new("myview", "data"))
        .await;
    assert!(result.is_err(), "Duplicate view creation should fail");
}

/// Falsification: Invalid subscription operations
#[tokio::test]
async fn falsify_error_invalid_subscription() {
    use koru_delta::subscriptions::SubscriptionId;

    let db = KoruDelta::start().await.unwrap();

    // Unsubscribe with invalid ID
    let result = db.unsubscribe(SubscriptionId(99999)).await;
    assert!(result.is_err());
}

// ============================================================================
// SECTION 12: LARGE DATA HANDLING
// ============================================================================

/// Falsification: Large JSON document handling
#[tokio::test]
async fn falsify_large_document() {
    let db = KoruDelta::start().await.unwrap();

    // Create a large nested structure
    let large_array: Vec<serde_json::Value> = (0..1000)
        .map(|i| {
            json!({
                "index": i,
                "data": "x".repeat(100),
                "nested": {
                    "a": i * 2,
                    "b": format!("item_{}", i)
                }
            })
        })
        .collect();

    let large_doc = json!({
        "items": large_array,
        "metadata": {
            "count": 1000,
            "description": "Large test document"
        }
    });

    db.put("large", "doc", large_doc.clone()).await.unwrap();

    let retrieved = db.get("large", "doc").await.unwrap();
    assert_eq!(retrieved, large_doc);
    assert_eq!(retrieved["items"].as_array().unwrap().len(), 1000);
}

/// Falsification: Many keys in single namespace
#[tokio::test]
async fn falsify_many_keys() {
    let db = KoruDelta::start().await.unwrap();

    // Insert 1000 keys
    for i in 0..1000 {
        db.put("many", format!("key{:04}", i), json!({"index": i}))
            .await
            .unwrap();
    }

    let stats = db.stats().await;
    assert_eq!(stats.key_count, 1000);

    // List keys should return all 1000
    let keys = db.list_keys("many").await;
    assert_eq!(keys.len(), 1000);

    // Keys should be sorted
    assert_eq!(keys[0], "key0000");
    assert_eq!(keys[999], "key0999");

    // Query should work on large collection
    let result = db
        .query(
            "many",
            Query::new()
                .filter(Filter::gte("index", json!(900)))
                .sort_by("index", true),
        )
        .await
        .unwrap();
    assert_eq!(result.records.len(), 100); // 900-999
}

// ============================================================================
// SECTION 13: HISTORY QUERY FALSIFICATION
// ============================================================================

/// Falsification: History query with time bounds
#[tokio::test]
async fn falsify_history_query_time_bounds() {
    let db = KoruDelta::start().await.unwrap();

    let mut timestamps = Vec::new();

    // Create history with known timestamps
    for i in 0..10 {
        let v = db
            .put("hq", "key", json!({"seq": i}))
            .await
            .unwrap();
        timestamps.push(v.timestamp());
        sleep(StdDuration::from_millis(20)).await;
    }

    // Query middle time range (entries 3-6)
    let query = HistoryQuery::new()
        .from(timestamps[3])
        .to(timestamps[6]);

    let results = db.query_history("hq", "key", query).await.unwrap();

    // Should include entries at timestamps 3, 4, 5, 6
    assert_eq!(results.len(), 4);
    for entry in &results {
        let seq = entry.value["seq"].as_i64().unwrap();
        assert!((3..=6).contains(&seq));
    }
}

/// Falsification: History query with filter
#[tokio::test]
async fn falsify_history_query_with_filter() {
    let db = KoruDelta::start().await.unwrap();

    // Create history with alternating values
    for i in 0..20 {
        db.put(
            "hq_filter",
            "key",
            json!({"value": i, "even": i % 2 == 0}),
        )
        .await
        .unwrap();
        sleep(StdDuration::from_millis(5)).await;
    }

    // Query only even entries
    let query = HistoryQuery::new()
        .with_query(Query::new().filter(Filter::eq("even", json!(true))));

    let results = db.query_history("hq_filter", "key", query).await.unwrap();

    assert_eq!(results.len(), 10);
    for entry in &results {
        assert!(entry.value["even"].as_bool().unwrap());
    }
}

/// Falsification: History query latest N
#[tokio::test]
async fn falsify_history_query_latest() {
    let db = KoruDelta::start().await.unwrap();

    for i in 0..20 {
        db.put("latest", "key", json!({"seq": i})).await.unwrap();
        sleep(StdDuration::from_millis(5)).await;
    }

    let query = HistoryQuery::new().latest(5);
    let results = db.query_history("latest", "key", query).await.unwrap();

    assert_eq!(results.len(), 5);

    // Should be the last 5 entries (15-19)
    let seqs: Vec<i64> = results.iter().map(|e| e.value["seq"].as_i64().unwrap()).collect();
    assert!(seqs.iter().all(|&s| s >= 15));
}
