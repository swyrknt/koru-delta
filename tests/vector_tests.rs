//! Integration tests for vector/embedding functionality.
//!
//! These tests verify the end-to-end vector storage and search API.

use koru_delta::prelude::*;
use koru_delta::vector::{Vector, VectorSearchOptions};

/// Test basic vector storage and retrieval
#[tokio::test]
async fn test_basic_vector_storage() {
    let db = KoruDelta::start().await.unwrap();

    // Store a vector
    let vector = Vector::new(vec![0.1, 0.2, 0.3, 0.4], "test-model");
    let result = db.embed("embeddings", "vec1", vector.clone(), None).await;
    assert!(result.is_ok());

    // Retrieve the vector
    let retrieved = db.get_embed("embeddings", "vec1").await.unwrap();
    assert!(retrieved.is_some());
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.dimensions(), 4);
    assert_eq!(retrieved.model(), "test-model");
}

/// Test vector storage with metadata
#[tokio::test]
async fn test_vector_with_metadata() {
    let db = KoruDelta::start().await.unwrap();

    let vector = Vector::new(vec![0.1, 0.2, 0.3], "test-model");
    let metadata = json!({"title": "Test Document", "category": "AI"});
    
    let result = db.embed("docs", "doc1", vector, Some(metadata)).await;
    assert!(result.is_ok());

    // Verify the value was stored with metadata
    let stored = db.get("docs", "doc1").await.unwrap();
    let value = stored.value();
    assert!(value.get("metadata").is_some());
    assert_eq!(value["metadata"]["title"], "Test Document");
}

/// Test basic similarity search
#[tokio::test]
async fn test_vector_similarity_search() {
    let db = KoruDelta::start().await.unwrap();

    // Store several vectors
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let v2 = Vector::new(vec![0.0, 1.0, 0.0], "test-model");
    let v3 = Vector::new(vec![0.9, 0.1, 0.0], "test-model"); // Similar to v1

    db.embed("vectors", "vec1", v1, None).await.unwrap();
    db.embed("vectors", "vec2", v2, None).await.unwrap();
    db.embed("vectors", "vec3", v3, None).await.unwrap();

    // Search with v1
    let query = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let results = db.embed_search(Some("vectors"), &query, VectorSearchOptions::new().top_k(2))
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].key, "vec1"); // Most similar to itself
    assert_eq!(results[1].key, "vec3"); // Second most similar
    
    // v1 and v3 should have high similarity
    assert!(results[0].score > 0.99);
    assert!(results[1].score > 0.9);
}

/// Test vector search with threshold filtering
#[tokio::test]
async fn test_vector_search_with_threshold() {
    let db = KoruDelta::start().await.unwrap();

    // Store vectors
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let v2 = Vector::new(vec![0.0, 1.0, 0.0], "test-model"); // Orthogonal to query

    db.embed("vectors", "vec1", v1, None).await.unwrap();
    db.embed("vectors", "vec2", v2, None).await.unwrap();

    // Search with high threshold
    let query = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let results = db
        .embed_search(
            Some("vectors"),
            &query,
            VectorSearchOptions::new().top_k(10).threshold(0.9),
        )
        .await
        .unwrap();

    // Only vec1 should match (similarity 1.0)
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].key, "vec1");
}

/// Test vector search across all namespaces
#[tokio::test]
async fn test_vector_search_all_namespaces() {
    let db = KoruDelta::start().await.unwrap();

    // Store vectors in different namespaces
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let v2 = Vector::new(vec![0.9, 0.1, 0.0], "test-model");

    db.embed("namespace1", "vec1", v1, None).await.unwrap();
    db.embed("namespace2", "vec2", v2, None).await.unwrap();

    // Search without namespace filter
    let query = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let results = db
        .embed_search(None, &query, VectorSearchOptions::new().top_k(10))
        .await
        .unwrap();

    // Should find both vectors
    assert_eq!(results.len(), 2);
}

/// Test vector versioning
#[tokio::test]
async fn test_vector_versioning() {
    let db = KoruDelta::start().await.unwrap();

    // Store initial vector
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    db.embed("vectors", "vec1", v1, None).await.unwrap();

    // Update with new vector
    let v2 = Vector::new(vec![0.0, 1.0, 0.0], "test-model");
    db.embed("vectors", "vec1", v2, None).await.unwrap();

    // Check history
    let history = db.history("vectors", "vec1").await.unwrap();
    assert_eq!(history.len(), 2);

    // Current value should be v2
    let current = db.get_embed("vectors", "vec1").await.unwrap().unwrap();
    assert!((current.as_slice()[0] - 0.0).abs() < 1e-6);
    assert!((current.as_slice()[1] - 1.0).abs() < 1e-6);
}

/// Test vector deletion
#[tokio::test]
async fn test_vector_deletion() {
    let db = KoruDelta::start().await.unwrap();

    // Store and then delete
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    db.embed("vectors", "vec1", v1, None).await.unwrap();

    // Should be in search
    let query = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    let results = db
        .embed_search(Some("vectors"), &query, VectorSearchOptions::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    // Delete
    db.delete_embed("vectors", "vec1").await.unwrap();

    // Should not be in search anymore
    let results = db
        .embed_search(Some("vectors"), &query, VectorSearchOptions::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 0);
}

/// Test vector search with model filtering
#[tokio::test]
async fn test_vector_search_model_filter() {
    let db = KoruDelta::start().await.unwrap();

    // Store vectors with different models
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "model-a");
    let v2 = Vector::new(vec![0.9, 0.1, 0.0], "model-b");

    db.embed("vectors", "vec1", v1, None).await.unwrap();
    db.embed("vectors", "vec2", v2, None).await.unwrap();

    // Search with model filter
    let query = Vector::new(vec![1.0, 0.0, 0.0], "model-a");
    let results = db
        .embed_search(
            Some("vectors"),
            &query,
            VectorSearchOptions::new().top_k(10).model_filter("model-a"),
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].key, "vec1");
}

/// Test vector operations with concurrent access
#[tokio::test]
async fn test_vector_concurrent_access() {
    let db = KoruDelta::start().await.unwrap();
    let db_clone = db.clone();

    // Spawn concurrent writes
    let handle1 = tokio::spawn(async move {
        for i in 0..10 {
            let v = Vector::new(vec![i as f32, 0.0, 0.0], "test-model");
            db.embed("concurrent", &format!("vec{}", i), v, None).await.unwrap();
        }
    });

    // Concurrent reads
    let handle2 = tokio::spawn(async move {
        for _ in 0..10 {
            let query = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
            let _ = db_clone.embed_search(Some("concurrent"), &query, VectorSearchOptions::new().top_k(5)).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    });

    let (r1, r2) = tokio::join!(handle1, handle2);
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}

/// Test getting non-existent vector
#[tokio::test]
async fn test_get_nonexistent_vector() {
    let db = KoruDelta::start().await.unwrap();

    let result = db.get_embed("nonexistent", "vec1").await.unwrap();
    assert!(result.is_none());
}

/// Test vector dimension mismatch handling
#[tokio::test]
async fn test_vector_dimension_mismatch() {
    let db = KoruDelta::start().await.unwrap();

    // Store 3D vector
    let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test-model");
    db.embed("vectors", "vec1", v1, None).await.unwrap();

    // Search with 2D vector (should return empty, not error)
    let query = Vector::new(vec![1.0, 0.0], "test-model");
    let results = db
        .embed_search(Some("vectors"), &query, VectorSearchOptions::new())
        .await
        .unwrap();

    // Should be empty because dimensions don't match
    assert!(results.is_empty());
}
