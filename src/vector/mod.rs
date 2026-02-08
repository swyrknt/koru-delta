//! Vector storage and similarity search for AI embeddings.
//!
//! This module provides native vector support for KoruDelta, enabling:
//! - Storage of embedding vectors (OpenAI, local models, etc.)
//! - Cosine similarity and Euclidean distance calculations
//! - Approximate nearest neighbor (ANN) search
//! - Integration with the causal storage system
//!
//! # Example
//!
//! ```ignore
//! use koru_delta::vector::{Vector, VectorSearchOptions};
//! use koru_delta::KoruDelta;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db = KoruDelta::start().await?;
//!
//!     // Store a vector
//!     let embedding = Vector::new(
//!         vec![0.1, 0.2, 0.3, 0.4],
//!         "text-embedding-3-small"
//!     );
//!     db.embed("documents", "doc1", embedding, None).await?;
//!
//!     // Search for similar vectors
//!     let query = Vector::new(vec![0.1, 0.2, 0.3, 0.4], "text-embedding-3-small");
//!     let results = db.embed_search("documents", &query, VectorSearchOptions::new().top_k(5)).await?;
//!
//!     for result in results {
//!         println!("{}: similarity = {}", result.key, result.score);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod causal_index;
mod distinction_integration;
mod hnsw;
mod index;
pub mod snsw;
mod types;

// Public exports
pub use causal_index::{CausalIndexConfig, CausalVectorIndex, IndexSnapshot, SnapshotStats};
pub use distinction_integration::{DistinctionBackedSNSW, DistinctionVector};
pub use hnsw::{HnswConfig, HnswIndex};
pub use index::{AnnIndex, FlatIndex, VectorIndex};
pub use snsw::{
    ContentHash, DistinctionOverlap, ExplainableResult, NavigationOp, ProximityWeights,
    SearchResult, SearchTier, SynthesisEdge, SynthesisExplanation, SynthesisGraph, SynthesisNode,
    SynthesisPath, SynthesisProximity, SynthesisType,
};
pub use types::{Vector, VectorSearchOptions, VectorSearchResult};

// Re-export snsw module for advanced usage
pub use snsw as synthesis_navigable;

use crate::error::DeltaResult;
use crate::types::VersionedValue;
use serde_json::json;

/// Extension trait for vector operations on KoruDelta.
///
/// This trait adds embedding storage and search capabilities to the core
/// KoruDelta database.
#[async_trait::async_trait]
pub trait VectorStorage {
    /// Store a vector embedding.
    ///
    /// Vectors are stored as versioned values, so changes to embeddings
    /// are tracked just like any other data in KoruDelta.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace (e.g., "embeddings", "documents")
    /// * `key` - The unique key for this vector
    /// * `vector` - The vector embedding
    /// * `metadata` - Optional JSON metadata to store with the vector
    ///
    /// # Example
    ///
    /// ```ignore
    /// let embedding = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
    /// db.embed("docs", "article1", embedding, Some(json!({"title": "AI"}))).await?;
    /// ```
    async fn embed(
        &self,
        namespace: impl Into<String> + Send,
        key: impl Into<String> + Send,
        vector: Vector,
        metadata: Option<serde_json::Value>,
    ) -> DeltaResult<VersionedValue>;

    /// Search for similar vectors.
    ///
    /// Performs approximate nearest neighbor search using cosine similarity.
    /// Returns the top-k most similar vectors that meet the threshold.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to search (optional - searches all if None)
    /// * `query` - The query vector
    /// * `options` - Search options (top_k, threshold, model_filter)
    async fn embed_search(
        &self,
        namespace: Option<impl Into<String> + Send>,
        query: &Vector,
        options: VectorSearchOptions,
    ) -> DeltaResult<Vec<VectorSearchResult>>;

    /// Get a stored vector by key.
    async fn get_embed(
        &self,
        namespace: impl Into<String> + Send,
        key: impl Into<String> + Send,
    ) -> DeltaResult<Option<Vector>>;

    /// Delete a vector.
    async fn delete_embed(
        &self,
        namespace: impl Into<String> + Send,
        key: impl Into<String> + Send,
    ) -> DeltaResult<Option<VersionedValue>>;
}

// Internal helper functions for serialization

/// Serialize a vector to JSON for storage.
#[allow(dead_code)]
pub(crate) fn vector_to_json(vector: &Vector, metadata: Option<serde_json::Value>) -> serde_json::Value {
    let mut obj = json!({
        "vector": vector.as_slice(),
        "model": vector.model(),
        "dimensions": vector.dimensions(),
    });

    if let Some(meta) = metadata {
        obj["metadata"] = meta;
    }

    obj
}

/// Deserialize a vector from JSON storage.
#[allow(dead_code)]
pub(crate) fn json_to_vector(value: &serde_json::Value) -> Option<Vector> {
    let vector_data = value.get("vector")?.as_array()?;
    let model = value.get("model")?.as_str()?;

    let data: Vec<f32> = vector_data
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    if data.is_empty() {
        return None;
    }

    Some(Vector::new(data, model))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_to_json() {
        let v = Vector::new(vec![0.1, 0.2, 0.3], "test-model");
        let json = vector_to_json(&v, Some(json!({"title": "Test"})));

        assert_eq!(json["model"], "test-model");
        assert_eq!(json["dimensions"], 3);
        assert!(json["vector"].is_array());
        assert_eq!(json["metadata"]["title"], "Test");
    }

    #[test]
    fn test_json_to_vector() {
        let json = json!({
            "vector": [0.1, 0.2, 0.3],
            "model": "test-model",
            "dimensions": 3
        });

        let v = json_to_vector(&json).unwrap();
        assert_eq!(v.dimensions(), 3);
        assert_eq!(v.model(), "test-model");
        assert!((v.as_slice()[0] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_json_to_vector_missing_fields() {
        let json = json!({"model": "test"});
        assert!(json_to_vector(&json).is_none());
    }

    #[test]
    fn test_json_to_vector_empty_vector() {
        let json = json!({
            "vector": [],
            "model": "test"
        });
        assert!(json_to_vector(&json).is_none());
    }
}
