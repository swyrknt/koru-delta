#[cfg(target_arch = "wasm32")]
use koru_delta::storage::CausalStorage;
/// WASM Compatibility Tests for KoruDelta
///
/// These tests verify that core database features work correctly
/// when compiled for WebAssembly targets.
///
/// Run with: cargo test --target wasm32-unknown-unknown --features wasm --no-default-features

// WASM-only imports
#[cfg(target_arch = "wasm32")]
use koru_delta::{KoruDelta, json};
#[cfg(target_arch = "wasm32")]
use koru_lambda_core::DistinctionEngine;
#[cfg(target_arch = "wasm32")]
use std::sync::Arc;

/// Test that basic put/get operations work on WASM.
///
/// This is the most fundamental database operation - storing and retrieving values.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_put_get() {
    let db = KoruDelta::start().await.expect("Failed to start database");

    // Store a value
    db.put(
        "users",
        "alice",
        json!({
            "name": "Alice",
            "email": "alice@example.com"
        }),
    )
    .await
    .expect("Failed to put value");

    // Retrieve the value
    let value = db.get("users", "alice").await.expect("Failed to get value");

    assert_eq!(value.value()["name"], "Alice");
    assert_eq!(value.value()["email"], "alice@example.com");
}

/// Test that history/time-travel works on WASM.
///
/// History tracking is a core feature of KoruDelta - every change is versioned.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_history_and_time_travel() {
    let db = KoruDelta::start().await.expect("Failed to start database");

    // Store initial value
    db.put("docs", "readme", json!({"version": 1, "content": "Hello"}))
        .await
        .expect("Failed to put initial value");

    // Store updated value
    db.put(
        "docs",
        "readme",
        json!({"version": 2, "content": "Hello World"}),
    )
    .await
    .expect("Failed to put updated value");

    // Get history
    let history = db
        .history("docs", "readme")
        .await
        .expect("Failed to get history");

    // Should have 2 entries
    assert_eq!(history.len(), 2, "History should have 2 entries");

    // Latest value should be version 2
    assert_eq!(history[0].value["version"], 2);

    // Oldest value should be version 1
    assert_eq!(history[1].value["version"], 1);
}

/// Test that namespace management works on WASM.
///
/// Namespaces provide logical separation of data.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_namespaces() {
    let db = KoruDelta::start().await.expect("Failed to start database");

    // Put values in different namespaces
    db.put("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();
    db.put("users", "bob", json!({"name": "Bob"}))
        .await
        .unwrap();
    db.put("config", "theme", json!("dark")).await.unwrap();

    // List namespaces
    let namespaces = db.list_namespaces().await;
    assert!(
        namespaces.contains(&"users".to_string()),
        "Should have 'users' namespace"
    );
    assert!(
        namespaces.contains(&"config".to_string()),
        "Should have 'config' namespace"
    );

    // List keys in namespace
    let user_keys = db.list_keys("users").await;
    assert!(
        user_keys.contains(&"alice".to_string()),
        "Should have 'alice' key"
    );
    assert!(
        user_keys.contains(&"bob".to_string()),
        "Should have 'bob' key"
    );

    // Keys should be isolated by namespace
    let config_keys = db.list_keys("config").await;
    assert!(
        config_keys.contains(&"theme".to_string()),
        "Should have 'theme' key"
    );
    assert!(
        !config_keys.contains(&"alice".to_string()),
        "Should not have 'alice' in config"
    );
}

/// Test that vector storage and search works on WASM.
///
/// Vector search is a key AI/ML feature.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_vector_search() {
    use koru_delta::Vector;

    let db = KoruDelta::start().await.expect("Failed to start database");

    // Store some vectors
    let vec1 = Vector::new(vec![1.0, 0.0, 0.0], None, "test");
    let vec2 = Vector::new(vec![0.0, 1.0, 0.0], None, "test");
    let vec3 = Vector::new(vec![0.9, 0.1, 0.0], None, "test");

    db.store_vector("doc1", vec1.clone())
        .await
        .expect("Failed to store vector");
    db.store_vector("doc2", vec2.clone())
        .await
        .expect("Failed to store vector");
    db.store_vector("doc3", vec3.clone())
        .await
        .expect("Failed to store vector");

    // Search for similar vectors
    let query = Vector::new(vec![1.0, 0.0, 0.0], None, "test");
    let results = db
        .search_vectors(&query, 2)
        .await
        .expect("Failed to search vectors");

    // Should find at least 1 result
    assert!(!results.is_empty(), "Should find at least one vector");

    // The most similar should be doc1 or doc3 (both close to [1,0,0])
    let first_result = &results[0];
    assert!(
        first_result.id == "doc1" || first_result.id == "doc3",
        "First result should be doc1 or doc3"
    );
}

/// Test that causal storage works directly on WASM.
///
/// This tests the storage layer directly without the full KoruDelta wrapper.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
fn test_wasm_causal_storage_direct() {
    let engine = Arc::new(DistinctionEngine::new());
    let storage = Arc::new(CausalStorage::new(engine));

    // Put a value
    storage
        .put("test", "key1", json!("value1"))
        .expect("Failed to put");

    // Get the value
    let value = storage.get("test", "key1").expect("Failed to get");
    assert_eq!(value.value(), &json!("value1"));

    // Update the value
    storage
        .put("test", "key1", json!("value2"))
        .expect("Failed to put");

    // Get history
    let history = storage
        .history("test", "key1")
        .expect("Failed to get history");
    assert_eq!(history.len(), 2, "Should have 2 history entries");
}

/// Test that query engine works on WASM.
///
/// The query engine provides filtering and aggregation.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_query_engine() {
    let db = KoruDelta::start().await.expect("Failed to start database");

    // Insert test data
    db.put(
        "products",
        "p1",
        json!({"name": "Laptop", "price": 999, "category": "electronics"}),
    )
    .await
    .unwrap();
    db.put(
        "products",
        "p2",
        json!({"name": "Mouse", "price": 29, "category": "electronics"}),
    )
    .await
    .unwrap();
    db.put(
        "products",
        "p3",
        json!({"name": "Desk", "price": 300, "category": "furniture"}),
    )
    .await
    .unwrap();

    // Query with filter
    let query = koru_delta::Query::new().filter(koru_delta::Filter::Eq(
        "category".to_string(),
        json!("electronics"),
    ));

    let results = db.query("products", query).await.expect("Failed to query");

    // Should find 2 electronics products
    assert_eq!(results.len(), 2, "Should find 2 electronics products");
}

/// Test that views work on WASM.
///
/// Views provide materialized query results.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_views() {
    let db = KoruDelta::start().await.expect("Failed to start database");

    // Insert test data
    db.put("sales", "s1", json!({"product": "A", "amount": 100}))
        .await
        .unwrap();
    db.put("sales", "s2", json!({"product": "B", "amount": 200}))
        .await
        .unwrap();
    db.put("sales", "s3", json!({"product": "A", "amount": 150}))
        .await
        .unwrap();

    // Create a view
    let view_def = koru_delta::ViewDefinition::new()
        .aggregate(koru_delta::Aggregation::Sum("amount".to_string()));

    db.create_view("total_sales", "sales", view_def)
        .await
        .expect("Failed to create view");

    // Query the view
    let view_data = db
        .query_view("total_sales")
        .await
        .expect("Failed to query view");

    // Total should be 450
    assert_eq!(view_data.value, json!(450), "Total sales should be 450");
}

/// Native-only tests - these should compile but not run on WASM.
/// They verify that the conditional compilation works correctly.
#[cfg(not(target_arch = "wasm32"))]
mod native_only_tests {
    use koru_delta::KoruDelta;

    #[tokio::test]
    async fn test_native_features_available() {
        // On native platforms, all features should be available
        let db = KoruDelta::start().await.expect("Failed to start database");

        // Verify the database started successfully
        // (Subscriptions and clustering are tested in cluster_tests.rs)
        let _ = db.stats().await;
    }
}
