//! Vector types and mathematical operations.
//!
//! This module provides the core vector types used for embeddings and
//! similarity search in KoruDelta.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A vector embedding with metadata.
///
/// Vectors are stored as `f32` arrays and are used for semantic search,
/// similarity comparisons, and AI agent memory retrieval.
///
/// # Example
///
/// ```ignore
/// let v1 = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
/// let v2 = Vector::new(vec![0.2, 0.3, 0.4], "text-embedding-3-small");
/// let similarity = v1.cosine_similarity(&v2);
/// ```
#[derive(Debug, Clone)]
pub struct Vector {
    /// The vector data (f32 for memory efficiency vs f64)
    data: Arc<[f32]>,
    /// The embedding model used to generate this vector
    model: String,
    /// Pre-computed magnitude for cosine similarity (cached)
    magnitude: Option<f32>,
}

impl Serialize for Vector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Vector", 3)?;
        state.serialize_field("data", &self.data.as_ref())?;
        state.serialize_field("model", &self.model)?;
        state.serialize_field("dimensions", &self.dimensions())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Vector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VectorData {
            data: Vec<f32>,
            model: String,
        }

        let helper = VectorData::deserialize(deserializer)?;
        Ok(Vector::new(helper.data, helper.model))
    }
}

impl Vector {
    /// Create a new vector with the given data and model.
    ///
    /// # Arguments
    ///
    /// * `data` - The vector components as f32 values
    /// * `model` - The embedding model identifier (e.g., "text-embedding-3-small")
    ///
    /// # Panics
    ///
    /// Panics if `data` is empty.
    pub fn new(data: Vec<f32>, model: impl Into<String>) -> Self {
        assert!(!data.is_empty(), "Vector data cannot be empty");
        Self {
            data: Arc::from(data.into_boxed_slice()),
            model: model.into(),
            magnitude: None,
        }
    }

    /// Create a new vector with pre-computed magnitude.
    ///
    /// This is useful when deserializing vectors where we want to
    /// avoid recomputing the magnitude.
    #[allow(dead_code)]
    pub(crate) fn with_magnitude(data: Vec<f32>, model: impl Into<String>, magnitude: f32) -> Self {
        assert!(!data.is_empty(), "Vector data cannot be empty");
        Self {
            data: Arc::from(data.into_boxed_slice()),
            model: model.into(),
            magnitude: Some(magnitude),
        }
    }

    /// Get the vector data as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Get the number of dimensions.
    pub fn dimensions(&self) -> usize {
        self.data.len()
    }

    /// Get the embedding model identifier.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Compute the Euclidean magnitude (L2 norm) of the vector.
    #[allow(dead_code)]
    fn compute_magnitude(&self) -> f32 {
        self.data.iter().map(|&x| x * x).sum::<f32>().sqrt()
    }

    /// Compute the magnitude without caching (for immutable refs).
    fn magnitude_uncached(&self) -> f32 {
        self.magnitude
            .unwrap_or_else(|| self.data.iter().map(|&x| x * x).sum::<f32>().sqrt())
    }

    /// Compute cosine similarity with another vector.
    ///
    /// Cosine similarity ranges from -1.0 (opposite) to 1.0 (identical).
    /// For normalized embeddings (common in AI), values are typically 0.0 to 1.0.
    ///
    /// # Arguments
    ///
    /// * `other` - The other vector to compare with
    ///
    /// # Returns
    ///
    /// The cosine similarity as an f32, or None if dimensions don't match.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let v1 = Vector::new(vec![1.0, 0.0], "test");
    /// let v2 = Vector::new(vec![0.0, 1.0], "test");
    /// assert_eq!(v1.cosine_similarity(&v2), Some(0.0));
    /// ```
    pub fn cosine_similarity(&self, other: &Vector) -> Option<f32> {
        if self.dimensions() != other.dimensions() {
            return None;
        }

        let dot_product: f32 = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .sum();

        let mag_a = self.magnitude_uncached();
        let mag_b = other.magnitude_uncached();

        // Handle zero vectors
        if mag_a == 0.0 || mag_b == 0.0 {
            return Some(0.0);
        }

        Some(dot_product / (mag_a * mag_b))
    }

    /// Compute Euclidean distance to another vector.
    ///
    /// Returns None if dimensions don't match.
    pub fn euclidean_distance(&self, other: &Vector) -> Option<f32> {
        if self.dimensions() != other.dimensions() {
            return None;
        }

        let sum_sq_diff: f32 = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| {
                let diff = a - b;
                diff * diff
            })
            .sum();

        Some(sum_sq_diff.sqrt())
    }

    /// Compute dot product with another vector.
    ///
    /// Returns None if dimensions don't match.
    pub fn dot_product(&self, other: &Vector) -> Option<f32> {
        if self.dimensions() != other.dimensions() {
            return None;
        }

        Some(
            self.data
                .iter()
                .zip(other.data.iter())
                .map(|(a, b)| a * b)
                .sum(),
        )
    }

    /// Check if this vector can be compared with another.
    ///
    /// Vectors must have the same dimensions and ideally the same model.
    pub fn is_compatible_with(&self, other: &Vector) -> bool {
        self.dimensions() == other.dimensions()
    }
}

impl PartialEq for Vector {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.model == other.model
    }
}

impl Eq for Vector {}

impl Hash for Vector {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash each f32 by its bit representation for consistency
        for &value in self.data.iter() {
            value.to_bits().hash(state);
        }
        self.model.hash(state);
    }
}

impl PartialOrd for Vector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.dimensions() != other.dimensions() {
            return None;
        }
        self.data.partial_cmp(&other.data)
    }
}

impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Vector(dims={}, model={})",
            self.dimensions(),
            self.model
        )
    }
}

/// A search result containing a vector and its similarity score.
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// The namespace of the matched vector
    pub namespace: String,
    /// The key of the matched vector
    pub key: String,
    /// The similarity score (higher = more similar)
    pub score: f32,
    /// The vector data
    pub vector: Vector,
}

impl VectorSearchResult {
    /// Create a new search result.
    pub fn new(namespace: impl Into<String>, key: impl Into<String>, score: f32, vector: Vector) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
            score,
            vector,
        }
    }
}

/// Options for vector search operations.
#[derive(Debug, Clone)]
pub struct VectorSearchOptions {
    /// Number of results to return
    pub top_k: usize,
    /// Minimum similarity threshold (0.0 to 1.0 for cosine)
    pub threshold: f32,
    /// Filter by model (optional)
    pub model_filter: Option<String>,
}

impl VectorSearchOptions {
    /// Create new search options with defaults.
    ///
    /// Defaults:
    /// - top_k: 10
    /// - threshold: 0.0 (no filtering)
    /// - model_filter: None
    pub fn new() -> Self {
        Self {
            top_k: 10,
            threshold: 0.0,
            model_filter: None,
        }
    }

    /// Set the number of results to return.
    pub fn top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Set the minimum similarity threshold.
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Filter by specific embedding model.
    pub fn model_filter(mut self, model: impl Into<String>) -> Self {
        self.model_filter = Some(model.into());
        self
    }
}

impl Default for VectorSearchOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_creation() {
        let v = Vector::new(vec![1.0, 2.0, 3.0], "test-model");
        assert_eq!(v.dimensions(), 3);
        assert_eq!(v.model(), "test-model");
        assert_eq!(v.as_slice(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v1 = Vector::new(vec![1.0, 0.0, 0.0], "test");
        let v2 = Vector::new(vec![1.0, 0.0, 0.0], "test");
        let sim = v1.cosine_similarity(&v2).unwrap();
        assert!((sim - 1.0).abs() < 1e-6, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let v1 = Vector::new(vec![1.0, 0.0], "test");
        let v2 = Vector::new(vec![0.0, 1.0], "test");
        let sim = v1.cosine_similarity(&v2).unwrap();
        assert!((sim - 0.0).abs() < 1e-6, "Orthogonal vectors should have similarity 0.0");
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let v1 = Vector::new(vec![1.0, 0.0], "test");
        let v2 = Vector::new(vec![-1.0, 0.0], "test");
        let sim = v1.cosine_similarity(&v2).unwrap();
        assert!((sim - (-1.0)).abs() < 1e-6, "Opposite vectors should have similarity -1.0");
    }

    #[test]
    fn test_cosine_similarity_mismatched_dims() {
        let v1 = Vector::new(vec![1.0, 0.0], "test");
        let v2 = Vector::new(vec![1.0, 0.0, 0.0], "test");
        assert!(v1.cosine_similarity(&v2).is_none(), "Mismatched dimensions should return None");
    }

    #[test]
    fn test_euclidean_distance() {
        let v1 = Vector::new(vec![0.0, 0.0], "test");
        let v2 = Vector::new(vec![3.0, 4.0], "test");
        let dist = v1.euclidean_distance(&v2).unwrap();
        assert!((dist - 5.0).abs() < 1e-6, "Distance should be 5.0");
    }

    #[test]
    fn test_euclidean_distance_mismatched_dims() {
        let v1 = Vector::new(vec![1.0, 0.0], "test");
        let v2 = Vector::new(vec![1.0, 0.0, 0.0], "test");
        assert!(v1.euclidean_distance(&v2).is_none(), "Mismatched dimensions should return None");
    }

    #[test]
    fn test_dot_product() {
        let v1 = Vector::new(vec![1.0, 2.0, 3.0], "test");
        let v2 = Vector::new(vec![4.0, 5.0, 6.0], "test");
        let dot = v1.dot_product(&v2).unwrap();
        assert!((dot - 32.0).abs() < 1e-6, "Dot product should be 32.0");
    }

    #[test]
    fn test_vector_equality() {
        let v1 = Vector::new(vec![1.0, 2.0, 3.0], "test");
        let v2 = Vector::new(vec![1.0, 2.0, 3.0], "test");
        let v3 = Vector::new(vec![1.0, 2.0, 4.0], "test");
        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_vector_hash() {
        use std::collections::HashSet;
        let v1 = Vector::new(vec![1.0, 2.0, 3.0], "test");
        let v2 = Vector::new(vec![1.0, 2.0, 3.0], "test");
        let mut set = HashSet::new();
        set.insert(v1);
        assert!(set.contains(&v2));
    }

    #[test]
    fn test_zero_vector() {
        let v1 = Vector::new(vec![1.0, 0.0], "test");
        let v2 = Vector::new(vec![0.0, 0.0], "test");
        let sim = v1.cosine_similarity(&v2).unwrap();
        assert_eq!(sim, 0.0, "Similarity with zero vector should be 0");
    }

    #[test]
    fn test_search_options() {
        let opts = VectorSearchOptions::new()
            .top_k(5)
            .threshold(0.8)
            .model_filter("text-embedding-3-small");
        
        assert_eq!(opts.top_k, 5);
        assert!((opts.threshold - 0.8).abs() < 1e-6);
        assert_eq!(opts.model_filter, Some("text-embedding-3-small".to_string()));
    }

    #[test]
    fn test_vector_display() {
        let v = Vector::new(vec![1.0, 2.0, 3.0], "test-model");
        let s = format!("{}", v);
        assert!(s.contains("dims=3"));
        assert!(s.contains("test-model"));
    }
}
