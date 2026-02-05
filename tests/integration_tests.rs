/// Integration tests for KoruDelta.
///
/// These tests verify end-to-end functionality of the database,
/// including all major features and edge cases.
use chrono::Utc;
use koru_delta::{json, DeltaError, KoruDelta};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_basic_put_get_workflow() {
    let db = KoruDelta::start().await.unwrap();

    // Store a user
    db.put(
        "users",
        "alice",
        json!({
            "name": "Alice",
            "email": "alice@example.com",
            "age": 30
        }),
    )
    .await
    .unwrap();

    // Retrieve the user
    let user = db.get("users", "alice").await.unwrap();
    assert_eq!(user["name"], "Alice");
    assert_eq!(user["email"], "alice@example.com");
    assert_eq!(user["age"], 30);
}

#[tokio::test]
async fn test_multiple_namespaces() {
    let db = KoruDelta::start().await.unwrap();

    // Store data in different namespaces
    db.put("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();
    db.put("sessions", "s123", json!({"user": "alice"}))
        .await
        .unwrap();
    db.put("config", "theme", json!("dark")).await.unwrap();

    // Retrieve from each namespace
    let user = db.get("users", "alice").await.unwrap();
    let session = db.get("sessions", "s123").await.unwrap();
    let config = db.get("config", "theme").await.unwrap();

    assert_eq!(user["name"], "Alice");
    assert_eq!(session["user"], "alice");
    assert_eq!(config, "dark");

    // Verify namespace isolation (same key in different namespace)
    db.put("data", "key1", json!(1)).await.unwrap();
    db.put("other", "key1", json!(2)).await.unwrap();

    assert_eq!(db.get("data", "key1").await.unwrap(), json!(1));
    assert_eq!(db.get("other", "key1").await.unwrap(), json!(2));
}

#[tokio::test]
async fn test_update_tracking() {
    let db = KoruDelta::start().await.unwrap();

    // Initial write
    let v1 = db.put("counter", "value", json!(1)).await.unwrap();
    assert!(v1.previous_version().is_none()); // First version has no predecessor

    sleep(Duration::from_millis(10)).await;

    // Update
    let v2 = db.put("counter", "value", json!(2)).await.unwrap();
    // previous_version() returns write_id, not version_id (distinction_id)
    assert_eq!(v2.previous_version(), Some(v1.write_id())); // Links to v1

    sleep(Duration::from_millis(10)).await;

    // Another update
    let v3 = db.put("counter", "value", json!(3)).await.unwrap();
    assert_eq!(v3.previous_version(), Some(v2.write_id())); // Links to v2

    // Current value should be the latest
    let current = db.get("counter", "value").await.unwrap();
    assert_eq!(current, json!(3));
}

#[tokio::test]
async fn test_history_chronological_order() {
    let db = KoruDelta::start().await.unwrap();

    // Make several writes
    for i in 1..=5 {
        db.put("log", "events", json!({"event": i})).await.unwrap();
        sleep(Duration::from_millis(10)).await;
    }

    // Get history
    let history = db.history("log", "events").await.unwrap();
    assert_eq!(history.len(), 5);

    // Verify chronological order
    for i in 0..5 {
        assert_eq!(history[i].value["event"], json!(i + 1));
        if i > 0 {
            assert!(history[i].timestamp >= history[i - 1].timestamp);
        }
    }
}

#[tokio::test]
async fn test_time_travel_precision() {
    let db = KoruDelta::start().await.unwrap();

    // Write with precise timing, capturing timestamps from the writes
    let v0 = db
        .put("doc", "content", json!({"version": "v0"}))
        .await
        .unwrap();
    let t0 = v0.timestamp();

    sleep(Duration::from_millis(100)).await;
    let v1 = db
        .put("doc", "content", json!({"version": "v1"}))
        .await
        .unwrap();
    let t1 = v1.timestamp();

    sleep(Duration::from_millis(100)).await;
    let v2 = db
        .put("doc", "content", json!({"version": "v2"}))
        .await
        .unwrap();
    let t2 = v2.timestamp();

    sleep(Duration::from_millis(100)).await;
    let v3 = db
        .put("doc", "content", json!({"version": "v3"}))
        .await
        .unwrap();
    let t3 = v3.timestamp();

    // Time travel to each point
    let v_at_t0 = db.get_at("doc", "content", t0).await.unwrap();
    assert_eq!(v_at_t0["version"], "v0");

    let v_at_t1 = db.get_at("doc", "content", t1).await.unwrap();
    assert_eq!(v_at_t1["version"], "v1");

    let v_at_t2 = db.get_at("doc", "content", t2).await.unwrap();
    assert_eq!(v_at_t2["version"], "v2");

    let v_at_t3 = db.get_at("doc", "content", t3).await.unwrap();
    assert_eq!(v_at_t3["version"], "v3");

    // Current should be v3
    let current = db.get("doc", "content").await.unwrap();
    assert_eq!(current["version"], "v3");
}

#[tokio::test]
async fn test_time_travel_before_existence() {
    let db = KoruDelta::start().await.unwrap();

    let before = Utc::now();
    sleep(Duration::from_millis(50)).await;
    db.put("data", "key", json!(1)).await.unwrap();

    // Try to get value before it existed
    let result = db.get_at("data", "key", before).await;
    assert!(matches!(result, Err(DeltaError::NoValueAtTimestamp { .. })));
}

#[tokio::test]
async fn test_error_handling_nonexistent_key() {
    let db = KoruDelta::start().await.unwrap();

    // Get nonexistent key
    let result = db.get("users", "nonexistent").await;
    match result {
        Err(DeltaError::KeyNotFound { namespace, key }) => {
            assert_eq!(namespace, "users");
            assert_eq!(key, "nonexistent");
        }
        _ => panic!("Expected KeyNotFound error"),
    }

    // History for nonexistent key
    let result = db.history("users", "nonexistent").await;
    assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));

    // Time travel for nonexistent key
    let result = db.get_at("users", "nonexistent", Utc::now()).await;
    assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));
}

#[tokio::test]
async fn test_contains_key() {
    let db = KoruDelta::start().await.unwrap();

    // Key doesn't exist initially
    assert!(!db.contains("users", "alice").await);

    // Add the key
    db.put("users", "alice", json!({})).await.unwrap();

    // Now it exists
    assert!(db.contains("users", "alice").await);

    // Different namespace
    assert!(!db.contains("sessions", "alice").await);
}

#[tokio::test]
async fn test_statistics_tracking() {
    let db = KoruDelta::start().await.unwrap();

    // Initial stats
    let stats = db.stats().await;
    assert_eq!(stats.key_count, 0);
    assert_eq!(stats.total_versions, 0);
    assert_eq!(stats.namespace_count, 0);

    // Add data
    db.put("users", "alice", json!(1)).await.unwrap();
    db.put("users", "alice", json!(2)).await.unwrap(); // Update
    db.put("users", "bob", json!(1)).await.unwrap();
    db.put("sessions", "s1", json!({})).await.unwrap();

    let stats = db.stats().await;
    assert_eq!(stats.key_count, 3); // alice, bob, s1
    assert_eq!(stats.total_versions, 4); // 2 for alice, 1 for bob, 1 for s1
    assert_eq!(stats.namespace_count, 2); // users, sessions
}

#[tokio::test]
async fn test_list_namespaces() {
    let db = KoruDelta::start().await.unwrap();

    // No namespaces initially
    assert_eq!(db.list_namespaces().await.len(), 0);

    // Add data to multiple namespaces
    db.put("users", "alice", json!({})).await.unwrap();
    db.put("sessions", "s1", json!({})).await.unwrap();
    db.put("config", "app", json!({})).await.unwrap();
    db.put("users", "bob", json!({})).await.unwrap(); // Same namespace

    let namespaces = db.list_namespaces().await;
    assert_eq!(namespaces, vec!["config", "sessions", "users"]);
}

#[tokio::test]
async fn test_list_keys() {
    let db = KoruDelta::start().await.unwrap();

    // Add keys to a namespace
    db.put("users", "charlie", json!({})).await.unwrap();
    db.put("users", "alice", json!({})).await.unwrap();
    db.put("users", "bob", json!({})).await.unwrap();
    db.put("sessions", "s1", json!({})).await.unwrap(); // Different namespace

    let user_keys = db.list_keys("users").await;
    assert_eq!(user_keys, vec!["alice", "bob", "charlie"]); // Sorted

    let session_keys = db.list_keys("sessions").await;
    assert_eq!(session_keys, vec!["s1"]);

    // Empty namespace
    let empty = db.list_keys("nonexistent").await;
    assert_eq!(empty.len(), 0);
}

#[tokio::test]
async fn test_complex_json_structures() {
    let db = KoruDelta::start().await.unwrap();

    let complex_data = json!({
        "user": {
            "id": 123,
            "name": "Alice",
            "email": "alice@example.com",
            "roles": ["admin", "developer", "reviewer"],
            "metadata": {
                "created": "2025-01-01T00:00:00Z",
                "last_login": "2025-01-15T12:30:00Z",
                "preferences": {
                    "theme": "dark",
                    "notifications": true,
                    "language": "en-US"
                }
            },
            "stats": {
                "login_count": 42,
                "posts": 15,
                "comments": 89
            }
        }
    });

    db.put("profiles", "alice", complex_data.clone())
        .await
        .unwrap();
    let retrieved = db.get("profiles", "alice").await.unwrap();

    assert_eq!(retrieved, complex_data);
    assert_eq!(retrieved["user"]["name"], "Alice");
    assert_eq!(retrieved["user"]["roles"][0], "admin");
    assert_eq!(
        retrieved["user"]["metadata"]["preferences"]["theme"],
        "dark"
    );
    assert_eq!(retrieved["user"]["stats"]["login_count"], 42);
}

#[tokio::test]
async fn test_various_json_types() {
    let db = KoruDelta::start().await.unwrap();

    // String
    db.put("data", "string", "Hello, World!").await.unwrap();
    assert_eq!(
        db.get("data", "string").await.unwrap(),
        json!("Hello, World!")
    );

    // Number (integer)
    db.put("data", "int", 42).await.unwrap();
    assert_eq!(db.get("data", "int").await.unwrap(), json!(42));

    // Number (float)
    db.put("data", "float", 3.15).await.unwrap();
    assert_eq!(db.get("data", "float").await.unwrap(), json!(3.15));

    // Boolean
    db.put("data", "bool_true", true).await.unwrap();
    db.put("data", "bool_false", false).await.unwrap();
    assert_eq!(db.get("data", "bool_true").await.unwrap(), json!(true));
    assert_eq!(db.get("data", "bool_false").await.unwrap(), json!(false));

    // Array
    db.put("data", "array", vec![1, 2, 3, 4, 5]).await.unwrap();
    assert_eq!(
        db.get("data", "array").await.unwrap(),
        json!([1, 2, 3, 4, 5])
    );

    // Null
    db.put("data", "null", json!(null)).await.unwrap();
    assert_eq!(db.get("data", "null").await.unwrap(), json!(null));

    // Nested
    db.put(
        "data",
        "nested",
        json!({
            "a": [1, 2, {"b": "c"}],
            "d": {"e": {"f": true}}
        }),
    )
    .await
    .unwrap();
    let nested = db.get("data", "nested").await.unwrap();
    assert_eq!(nested["a"][2]["b"], "c");
    assert_eq!(nested["d"]["e"]["f"], true);
}

#[tokio::test]
async fn test_database_clone_shares_state() {
    let db1 = KoruDelta::start().await.unwrap();
    let db2 = db1.clone();

    // Write with db1
    db1.put("shared", "key1", json!(1)).await.unwrap();

    // Read with db2 (should see the same data)
    let value = db2.get("shared", "key1").await.unwrap();
    assert_eq!(value, json!(1));

    // Write with db2
    db2.put("shared", "key2", json!(2)).await.unwrap();

    // Read with db1 (should see the same data)
    let value = db1.get("shared", "key2").await.unwrap();
    assert_eq!(value, json!(2));

    // Stats should be consistent
    assert_eq!(db1.stats().await.key_count, db2.stats().await.key_count);
}

#[tokio::test]
async fn test_concurrent_writes_different_keys() {
    let db = KoruDelta::start().await.unwrap();
    let mut handles = vec![];

    // Spawn 20 tasks writing to different keys
    for i in 0..20 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            db_clone
                .put("concurrent", format!("key{}", i), json!(i))
                .await
                .unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // All keys should exist
    assert_eq!(db.stats().await.key_count, 20);
    for i in 0..20 {
        assert!(db.contains("concurrent", format!("key{}", i)).await);
        let value = db.get("concurrent", format!("key{}", i)).await.unwrap();
        assert_eq!(value, json!(i));
    }
}

#[tokio::test]
async fn test_concurrent_updates_same_key() {
    let db = KoruDelta::start().await.unwrap();
    let mut handles = vec![];

    // Spawn 30 tasks updating the same key
    for i in 0..30 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            db_clone.put("counter", "value", json!(i)).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Should have exactly 1 key
    assert_eq!(db.stats().await.key_count, 1);

    // Should have 30 versions
    assert_eq!(db.stats().await.total_versions, 30);

    // History should contain all updates
    let history = db.history("counter", "value").await.unwrap();
    assert_eq!(history.len(), 30);
}

#[tokio::test]
async fn test_versioned_value_metadata() {
    let db = KoruDelta::start().await.unwrap();

    let before = Utc::now();
    let versioned = db.put("data", "key", json!({"test": true})).await.unwrap();
    let after = Utc::now();

    // Verify metadata
    assert!(!versioned.version_id().is_empty());
    assert!(versioned.timestamp() >= before);
    assert!(versioned.timestamp() <= after);
    assert!(versioned.previous_version().is_none()); // First version

    // Second version should link to first
    let versioned2 = db.put("data", "key", json!({"test": false})).await.unwrap();
    // previous_version() returns write_id (unique per write), version_id() returns distinction_id (content hash)
    assert_eq!(versioned2.previous_version(), Some(versioned.write_id()));
    // Different content = different distinction_ids
    assert_ne!(versioned2.version_id(), versioned.version_id());
}

#[tokio::test]
async fn test_deterministic_version_ids() {
    let db = KoruDelta::start().await.unwrap();

    // Same data should produce same version ID (content-addressed)
    let v1 = db
        .put("test", "key1", json!({"data": "identical"}))
        .await
        .unwrap();
    let v2 = db
        .put("test", "key2", json!({"data": "identical"}))
        .await
        .unwrap();

    // Version IDs should be identical (content-addressed)
    assert_eq!(v1.version_id(), v2.version_id());

    // Different data should produce different IDs
    let v3 = db
        .put("test", "key3", json!({"data": "different"}))
        .await
        .unwrap();
    assert_ne!(v1.version_id(), v3.version_id());
}

#[tokio::test]
async fn test_empty_database_operations() {
    let db = KoruDelta::start().await.unwrap();

    // Empty database stats
    let stats = db.stats().await;
    assert_eq!(stats.key_count, 0);
    assert_eq!(stats.total_versions, 0);
    assert_eq!(stats.namespace_count, 0);

    // Empty lists
    assert_eq!(db.list_namespaces().await.len(), 0);
    assert_eq!(db.list_keys("any").await.len(), 0);

    // Contains returns false
    assert!(!db.contains("any", "key").await);
}
