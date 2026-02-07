//! Vector indexing for approximate nearest neighbor search.
//!
//! This module provides indexing structures for efficient vector search.
//! Currently implements a flat (brute-force) index optimized for small to
//! medium datasets (up to ~100K vectors).
//!
//! Future: HNSW or IVF indexes for larger datasets.

use super::types::{Vector, VectorSearchOptions, VectorSearchResult};
use crate::types::FullKey;
use dashmap::DashMap;
use std::sync::Arc;

/// An approximate nearest neighbor index for vectors.
///
/// This trait abstracts over different indexing strategies (flat, HNSW, IVF, etc.)
/// to allow swapping implementations based on dataset size and performance needs.
pub trait AnnIndex: Send + Sync {
    /// Add a vector to the index.
    fn add(&self, key: FullKey, vector: Vector);

    /// Remove a vector from the index.
    fn remove(&self, namespace: &str, key: &str);

    /// Search for nearest neighbors.
    fn search(&self, query: &Vector, opts: &VectorSearchOptions) -> Vec<VectorSearchResult>;

    /// Get the number of vectors in the index.
    fn len(&self) -> usize;

    /// Check if the index is empty.
    fn is_empty(&self) -> bool;

    /// Clear all vectors from the index.
    fn clear(&self);
}

/// A flat (brute-force) vector index.
///
/// Stores all vectors in memory and performs exact k-NN search by comparing
/// the query against every vector. This is simple and memory-efficient but
/// has O(n) query complexity.
///
/// Suitable for:
/// - Small datasets (< 10K vectors)
/// - When exact results are required
/// - When simplicity is preferred over speed
///
/// For larger datasets, consider future HNSW implementation.
#[derive(Debug)]
pub struct FlatIndex {
    /// namespace -> (key -> Vector)
    vectors: DashMap<String, DashMap<String, Vector>>,
}

impl FlatIndex {
    /// Create a new empty flat index.
    pub fn new() -> Self {
        Self {
            vectors: DashMap::new(),
        }
    }

    /// Get all vectors in a namespace.
    pub fn get_namespace(&self, namespace: &str) -> Option<Arc<DashMap<String, Vector>>> {
        self.vectors.get(namespace).map(|entry| Arc::new(entry.clone()))
    }
}

impl Default for FlatIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnIndex for FlatIndex {
    fn add(&self, key: FullKey, vector: Vector) {
        let namespace_entry = self.vectors.entry(key.namespace).or_default();
        namespace_entry.insert(key.key, vector);
    }

    fn remove(&self, namespace: &str, key: &str) {
        if let Some(namespace_entry) = self.vectors.get(namespace) {
            namespace_entry.remove(key);
            // Clean up empty namespaces
            if namespace_entry.is_empty() {
                drop(namespace_entry);
                self.vectors.remove(namespace);
            }
        }
    }

    fn search(&self, query: &Vector, opts: &VectorSearchOptions) -> Vec<VectorSearchResult> {
        let mut results: Vec<VectorSearchResult> = Vec::new();

        // Iterate over all namespaces and vectors
        for namespace_entry in self.vectors.iter() {
            let namespace = namespace_entry.key();

            for vector_entry in namespace_entry.value().iter() {
                let key = vector_entry.key();
                let vector = vector_entry.value();

                // Skip if model filter doesn't match
                if let Some(ref model_filter) = opts.model_filter {
                    if vector.model() != model_filter {
                        continue;
                    }
                }

                // Skip incompatible dimensions
                if !query.is_compatible_with(vector) {
                    continue;
                }

                // Compute similarity (cosine)
                if let Some(similarity) = query.cosine_similarity(vector) {
                    // Apply threshold
                    if similarity >= opts.threshold {
                        results.push(VectorSearchResult::new(
                            namespace.clone(),
                            key.clone(),
                            similarity,
                            vector.clone(),
                        ));
                    }
                }
            }
        }

        // Sort by similarity (highest first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Take top_k
        results.truncate(opts.top_k);

        results
    }

    fn len(&self) -> usize {
        self.vectors
            .iter()
            .map(|entry| entry.value().len())
            .sum()
    }

    fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    fn clear(&self) {
        self.vectors.clear();
    }
}

/// A thread-safe wrapper around an ANN index.
pub struct VectorIndex {
    inner: Arc<dyn AnnIndex>,
}

impl std::fmt::Debug for VectorIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorIndex")
            .field("len", &self.len())
            .field("is_empty", &self.is_empty())
            .finish()
    }
}

impl VectorIndex {
    /// Create a new vector index with a flat index backend.
    pub fn new_flat() -> Self {
        Self {
            inner: Arc::new(FlatIndex::new()),
        }
    }

    /// Add a vector to the index.
    pub fn add(&self, key: FullKey, vector: Vector) {
        self.inner.add(key, vector);
    }

    /// Remove a vector from the index.
    pub fn remove(&self, namespace: &str, key: &str) {
        self.inner.remove(namespace, key);
    }

    /// Search for nearest neighbors.
    pub fn search(&self, query: &Vector, opts: &VectorSearchOptions) -> Vec<VectorSearchResult> {
        self.inner.search(query, opts)
    }

    /// Get the number of vectors in the index.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all vectors from the index.
    pub fn clear(&self) {
        self.inner.clear();
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new_flat()
    }
}

impl Clone for VectorIndex {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_index_add_and_search() {
        let index = FlatIndex::new();

        // Add some vectors
        let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test");
        let v2 = Vector::new(vec![0.0, 1.0, 0.0], "test");
        let v3 = Vector::new(vec![0.0, 0.0, 1.0], "test");

        index.add(FullKey::new("docs", "doc1"), v1);
        index.add(FullKey::new("docs", "doc2"), v2);
        index.add(FullKey::new("docs", "doc3"), v3);

        // Search
        let query = Vector::new(vec![0.9, 0.1, 0.0], "test");
        let opts = VectorSearchOptions::new().top_k(2);
        let results = index.search(&query, &opts);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, "doc1"); // Most similar
        assert!(results[0].score > 0.9);
    }

    #[test]
    fn test_flat_index_threshold() {
        let index = FlatIndex::new();

        let v1 = Vector::new(vec![1.0, 0.0], "test");
        let v2 = Vector::new(vec![0.0, 1.0], "test");

        index.add(FullKey::new("docs", "doc1"), v1);
        index.add(FullKey::new("docs", "doc2"), v2);

        // Search with high threshold
        let query = Vector::new(vec![1.0, 0.0], "test");
        let opts = VectorSearchOptions::new().top_k(10).threshold(0.9);
        let results = index.search(&query, &opts);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "doc1");
    }

    #[test]
    fn test_flat_index_model_filter() {
        let index = FlatIndex::new();

        let v1 = Vector::new(vec![1.0, 0.0], "model-a");
        let v2 = Vector::new(vec![0.0, 1.0], "model-b");

        index.add(FullKey::new("docs", "doc1"), v1);
        index.add(FullKey::new("docs", "doc2"), v2);

        // Search with model filter
        let query = Vector::new(vec![1.0, 0.0], "model-a");
        let opts = VectorSearchOptions::new()
            .top_k(10)
            .model_filter("model-a");
        let results = index.search(&query, &opts);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "doc1");
    }

    #[test]
    fn test_flat_index_remove() {
        let index = FlatIndex::new();

        let v1 = Vector::new(vec![1.0, 0.0], "test");
        index.add(FullKey::new("docs", "doc1"), v1);
        assert_eq!(index.len(), 1);

        index.remove("docs", "doc1");
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_flat_index_mismatched_dims() {
        let index = FlatIndex::new();

        let v1 = Vector::new(vec![1.0, 0.0], "test");
        index.add(FullKey::new("docs", "doc1"), v1);

        // Query with different dimensions
        let query = Vector::new(vec![1.0, 0.0, 0.0], "test");
        let opts = VectorSearchOptions::new();
        let results = index.search(&query, &opts);

        assert!(results.is_empty());
    }

    #[test]
    fn test_vector_index_wrapper() {
        let index = VectorIndex::new_flat();

        let v1 = Vector::new(vec![1.0, 0.0], "test");
        index.add(FullKey::new("docs", "doc1"), v1);

        let query = Vector::new(vec![1.0, 0.0], "test");
        let opts = VectorSearchOptions::new();
        let results = index.search(&query, &opts);

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_vector_index_clone() {
        let index = VectorIndex::new_flat();

        let v1 = Vector::new(vec![1.0, 0.0], "test");
        index.add(FullKey::new("docs", "doc1"), v1);

        let cloned = index.clone();
        assert_eq!(cloned.len(), 1);

        cloned.remove("docs", "doc1");
        assert_eq!(index.len(), 0); // Both see the change (shared Arc)
    }
}
