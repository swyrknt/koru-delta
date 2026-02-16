//! Vector Agent - Embedding and similarity search with LCA architecture.
//!
//! This module implements the Vector agent following the Local Causal Agent pattern.
//! From LCA perspective, vectors are not "stored objects" but distinctions
//! synthesized from the vector root. Similarity is not "calculated" but
//! discovered through structural relationships in the field.
//!
//! # Key Insight
//!
//! A vector embedding is not "stored" in a database. From LCA perspective:
//! - Vectors are distinctions synthesized from the vector root
//! - Similarity emerges from structural relationships (synthesis paths)
//! - Search is navigation through the field's causal structure
//!
//! Formula for vector operations:
//! ```text
//! ΔNew = ΔVector_Root ⊕ ΔVector_Data
//! ```
//!
//! Formula for indexing:
//! ```text
//! ΔNew = ΔVector_Local_Root ⊕ ΔEmbedding ⊕ ΔKey
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use serde::{Deserialize, Serialize};

use crate::actions::VectorAction;

/// Convert bytes to distinction via byte-wise synthesis.
fn bytes_to_distinction(bytes: &[u8], engine: &DistinctionEngine) -> Distinction {
    bytes
        .iter()
        .map(|&byte| byte.to_canonical_structure(engine))
        .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
}

/// Convert a vector to distinction via float-wise synthesis.
fn vector_to_distinction(vector: &[f32], engine: &DistinctionEngine) -> Distinction {
    vector
        .iter()
        .map(|&f| {
            // Convert f32 to bytes for canonicalization
            let bytes = f.to_be_bytes();
            bytes_to_distinction(&bytes, engine)
        })
        .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
}

/// A synthesized vector embedding distinction.
///
/// Represents a vector that has been synthesized into the field.
#[derive(Debug, Clone)]
pub struct SynthesizedVector {
    /// The canonical distinction representing this vector.
    pub distinction: Distinction,

    /// The vector data.
    pub vector: Vec<f32>,

    /// The key associated with this vector.
    pub key: String,

    /// The model used to generate this embedding.
    pub model: String,

    /// When this vector was synthesized.
    pub synthesized_at: DateTime<Utc>,
}

/// Metadata for a vector embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    /// Vector identifier.
    pub id: String,

    /// Associated key.
    pub key: String,

    /// Model identifier.
    pub model: String,

    /// Vector dimensions.
    pub dimensions: usize,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Content hash for verification.
    pub content_hash: String,
}

impl VectorMetadata {
    /// Create new metadata for a vector.
    pub fn new(
        id: impl Into<String>,
        key: impl Into<String>,
        model: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            model: model.into(),
            dimensions,
            created_at: Utc::now(),
            content_hash: String::new(),
        }
    }
}

impl Canonicalizable for VectorMetadata {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let bytes = bincode::serialize(self).expect("VectorMetadata should serialize");
        bytes_to_distinction(&bytes, engine)
    }
}

/// Vector search result.
#[derive(Debug, Clone)]
pub struct VectorSearchItem {
    /// The key of the matched vector.
    pub key: String,

    /// The vector data.
    pub vector: Vec<f32>,

    /// Similarity score (0.0 to 1.0).
    pub score: f32,

    /// The synthesized distinction.
    pub distinction: Distinction,
}

/// Vector agent - manages embeddings and similarity search.
///
/// The vector agent follows the LCA pattern:
/// - It has a vector root distinction
/// - Each operation synthesizes from the current local root
/// - Similarity emerges from causal structure
///
/// # Example
///
/// ```rust
/// use koru_delta::vector_agent::VectorAgent;
/// use koru_delta::roots::KoruRoots;
/// use koru_lambda_core::DistinctionEngine;
/// use std::sync::Arc;
///
/// let engine = Arc::new(DistinctionEngine::new());
/// let roots = KoruRoots::initialize(&engine);
/// let mut agent = VectorAgent::new(roots.vector.clone(), engine);
///
/// // Index a vector
/// agent.index("doc1", vec![0.1, 0.2, 0.3], "text-embedding-3-small");
///
/// // Search
/// let results = agent.search(&[0.1, 0.2, 0.3], 5, 0.0);
/// ```
pub struct VectorAgent {
    /// The engine for synthesis.
    engine: Arc<DistinctionEngine>,

    /// Indexed vectors by key.
    vectors: RwLock<HashMap<String, SynthesizedVector>>,

    /// Current local root for the vector agent.
    local_root: Distinction,

    /// Operation sequence counter.
    sequence: AtomicU64,

    /// Metrics tracking.
    metrics: RwLock<VectorMetrics>,
}

/// Metrics for vector operations.
#[derive(Debug, Clone, Default)]
pub struct VectorMetrics {
    /// Number of vectors indexed.
    pub vectors_indexed: u64,

    /// Number of searches performed.
    pub searches_performed: u64,

    /// Number of embeddings created.
    pub embeddings_created: u64,
}

impl VectorAgent {
    /// Create a new vector agent.
    ///
    /// # Arguments
    ///
    /// * `vector_root` - The canonical vector root (becomes initial local_root)
    /// * `engine` - The distinction engine for synthesis
    pub fn new(vector_root: Distinction, engine: Arc<DistinctionEngine>) -> Self {
        Self {
            engine,
            vectors: RwLock::new(HashMap::new()),
            local_root: vector_root,
            sequence: AtomicU64::new(0),
            metrics: RwLock::new(VectorMetrics::default()),
        }
    }

    /// Get the current local root.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Get current metrics.
    pub fn metrics(&self) -> VectorMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Index a vector.
    ///
    /// Formula: ΔNew = ΔLocal_Root ⊕ ΔVector_Data ⊕ ΔKey
    pub fn index(
        &mut self,
        key: impl Into<String>,
        vector: Vec<f32>,
        model: impl Into<String>,
    ) -> SynthesizedVector {
        let key = key.into();
        let model = model.into();
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let local_root = self.local_root.clone();

        // Synthesize the vector data
        let vector_distinction = vector_to_distinction(&vector, &self.engine);

        // Include key in synthesis
        let key_distinction = bytes_to_distinction(key.as_bytes(), &self.engine);
        let combined = self
            .engine
            .synthesize(&vector_distinction, &key_distinction);

        // Include model in synthesis
        let model_distinction = bytes_to_distinction(model.as_bytes(), &self.engine);
        let combined = self.engine.synthesize(&combined, &model_distinction);

        // Include sequence for uniqueness
        let seq_bytes = bincode::serialize(&seq).unwrap();
        let seq_distinction = bytes_to_distinction(&seq_bytes, &self.engine);
        let combined = self.engine.synthesize(&combined, &seq_distinction);

        // Synthesize from local root
        let distinction = self.engine.synthesize(&local_root, &combined);

        let synthesized_vector = SynthesizedVector {
            distinction: distinction.clone(),
            vector: vector.clone(),
            key: key.clone(),
            model: model.clone(),
            synthesized_at: Utc::now(),
        };

        // Update local root
        self.local_root = distinction;

        // Store the vector
        self.vectors
            .write()
            .unwrap()
            .insert(key, synthesized_vector.clone());

        self.metrics.write().unwrap().vectors_indexed += 1;

        synthesized_vector
    }

    /// Get a vector by key.
    pub fn get_vector(&self, key: &str) -> Option<SynthesizedVector> {
        self.vectors.read().unwrap().get(key).cloned()
    }

    /// List all indexed vectors.
    pub fn list_vectors(&self) -> Vec<SynthesizedVector> {
        self.vectors.read().unwrap().values().cloned().collect()
    }

    /// Calculate cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Search for similar vectors.
    ///
    /// Performs brute-force cosine similarity search.
    /// For production use, integrate with SNSW/HNSW indices.
    pub fn search(&self, query: &[f32], top_k: usize, threshold: f32) -> Vec<VectorSearchItem> {
        let vectors = self.vectors.read().unwrap();

        let mut results: Vec<VectorSearchItem> = vectors
            .values()
            .map(|v| {
                let score = Self::cosine_similarity(query, &v.vector);
                VectorSearchItem {
                    key: v.key.clone(),
                    vector: v.vector.clone(),
                    score,
                    distinction: v.distinction.clone(),
                }
            })
            .filter(|r| r.score >= threshold)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Take top_k
        results.truncate(top_k);

        self.metrics.write().unwrap().searches_performed += 1;

        results
    }

    /// Create an embedding (placeholder - actual embedding requires ML model).
    ///
    /// In a real implementation, this would call an embedding model.
    /// For now, it just creates a deterministic vector from the data.
    pub fn embed(&self, data: &[u8], _model: impl Into<String>) -> Vec<f32> {
        // Simple deterministic embedding: use hash of data as seed
        // In production, this would call an actual embedding model
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }

        // Generate a 128-dimensional vector
        let dimensions = 128;
        let mut vector = Vec::with_capacity(dimensions);
        let mut rng = hash;

        for _ in 0..dimensions {
            // Simple LCG
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let value = ((rng >> 16) & 0x7FFF) as f32 / 32768.0;
            vector.push(value * 2.0 - 1.0); // Scale to [-1, 1]
        }

        // Normalize
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vector {
                *x /= norm;
            }
        }

        self.metrics.write().unwrap().embeddings_created += 1;

        vector
    }

    /// Execute a vector action.
    ///
    /// This is the main entry point for vector operations.
    pub fn execute(&mut self, action: VectorAction) -> VectorResult {
        match action {
            VectorAction::Embed {
                data_json,
                model,
                dimensions: _,
            } => {
                let data = serde_json::to_vec(&data_json).unwrap_or_default();
                let vector = self.embed(&data, model);
                VectorResult::Vector(vector)
            }
            VectorAction::Search {
                query_vector,
                top_k,
                threshold,
            } => {
                let results = self.search(&query_vector, top_k, threshold);
                VectorResult::SearchResults(results)
            }
            VectorAction::Index { vector, key, model } => {
                let indexed = self.index(key, vector, model);
                VectorResult::IndexedVector(indexed)
            }
        }
    }
}

/// Result of a vector operation.
#[derive(Debug, Clone)]
pub enum VectorResult {
    /// A vector embedding.
    Vector(Vec<f32>),

    /// Search results.
    SearchResults(Vec<VectorSearchItem>),

    /// An indexed vector.
    IndexedVector(SynthesizedVector),

    /// An error occurred.
    Error(String),
}

impl fmt::Display for VectorResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VectorResult::Vector(v) => write!(f, "Vector({} dims)", v.len()),
            VectorResult::SearchResults(r) => write!(f, "SearchResults({} items)", r.len()),
            VectorResult::IndexedVector(v) => write!(f, "IndexedVector({})", v.key),
            VectorResult::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

// LCA Trait Implementation
impl LocalCausalAgent for VectorAgent {
    type ActionData = VectorAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: VectorAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_agent() -> (VectorAgent, Arc<DistinctionEngine>) {
        let engine = Arc::new(DistinctionEngine::new());
        let vector_root = engine.synthesize(&engine.d0().clone(), &engine.d1().clone());
        let agent = VectorAgent::new(vector_root, engine.clone());
        (agent, engine)
    }

    #[test]
    fn test_index_vector() {
        let (mut agent, _) = setup_agent();

        let vector = vec![0.1, 0.2, 0.3, 0.4];
        let indexed = agent.index("doc1", vector.clone(), "text-embedding-3-small");

        assert_eq!(indexed.key, "doc1");
        assert_eq!(indexed.vector, vector);
        assert_eq!(indexed.model, "text-embedding-3-small");
    }

    #[test]
    fn test_get_vector() {
        let (mut agent, _) = setup_agent();

        let vector = vec![0.1, 0.2, 0.3];
        agent.index("doc1", vector, "model");

        let retrieved = agent.get_vector("doc1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().key, "doc1");

        assert!(agent.get_vector("nonexistent").is_none());
    }

    #[test]
    fn test_cosine_similarity() {
        // Same vector should have similarity 1.0
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorAgent::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        // Orthogonal vectors should have similarity 0.0
        let c = vec![1.0, 0.0];
        let d = vec![0.0, 1.0];
        assert!(VectorAgent::cosine_similarity(&c, &d).abs() < 0.001);

        // Opposite vectors should have similarity -1.0
        let e = vec![1.0, 0.0];
        let f = vec![-1.0, 0.0];
        assert!((VectorAgent::cosine_similarity(&e, &f) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_search() {
        let (mut agent, _) = setup_agent();

        // Index some vectors
        agent.index("doc1", vec![1.0, 0.0, 0.0], "model");
        agent.index("doc2", vec![0.0, 1.0, 0.0], "model");
        agent.index("doc3", vec![0.9, 0.1, 0.0], "model"); // Close to doc1

        // Search for something similar to doc1
        let query = vec![1.0, 0.0, 0.0];
        let results = agent.search(&query, 2, 0.0);

        assert_eq!(results.len(), 2);
        // doc1 and doc3 should be most similar
        assert!(results.iter().any(|r| r.key == "doc1"));
    }

    #[test]
    fn test_search_with_threshold() {
        let (mut agent, _) = setup_agent();

        agent.index("doc1", vec![1.0, 0.0, 0.0], "model");
        agent.index("doc2", vec![0.0, 1.0, 0.0], "model");

        let query = vec![1.0, 0.0, 0.0];
        let results = agent.search(&query, 10, 0.9);

        // Only doc1 should meet the threshold
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "doc1");
    }

    #[test]
    fn test_embed() {
        let (agent, _) = setup_agent();

        let data = b"test data";
        let vector = agent.embed(data, "model");

        assert_eq!(vector.len(), 128); // Default dimensions

        // Vector should be normalized
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_embed_deterministic() {
        let (agent, _) = setup_agent();

        let data = b"test data";
        let vector1 = agent.embed(data, "model");
        let vector2 = agent.embed(data, "model");

        assert_eq!(vector1, vector2);
    }

    #[test]
    fn test_metrics() {
        let (mut agent, _) = setup_agent();

        agent.index("doc1", vec![0.1, 0.2], "model");
        agent.embed(b"data", "model");
        agent.search(&[0.1, 0.2], 10, 0.0);

        let metrics = agent.metrics();
        assert_eq!(metrics.vectors_indexed, 1);
        assert_eq!(metrics.embeddings_created, 1);
        assert_eq!(metrics.searches_performed, 1);
    }

    #[test]
    fn test_execute_embed() {
        let (mut agent, _) = setup_agent();

        let action = VectorAction::Embed {
            data_json: serde_json::json!({"text": "hello"}),
            model: "text-embedding-3-small".to_string(),
            dimensions: 128,
        };

        let result = agent.execute(action);
        match result {
            VectorResult::Vector(v) => assert_eq!(v.len(), 128),
            _ => panic!("Expected Vector result"),
        }
    }

    #[test]
    fn test_execute_search() {
        let (mut agent, _) = setup_agent();

        agent.index("doc1", vec![1.0, 0.0], "model");

        let action = VectorAction::Search {
            query_vector: vec![1.0, 0.0],
            top_k: 5,
            threshold: 0.0,
        };

        let result = agent.execute(action);
        match result {
            VectorResult::SearchResults(r) => assert!(!r.is_empty()),
            _ => panic!("Expected SearchResults"),
        }
    }

    #[test]
    fn test_execute_index() {
        let (mut agent, _) = setup_agent();

        let action = VectorAction::Index {
            vector: vec![0.1, 0.2, 0.3],
            key: "doc1".to_string(),
            model: "text-embedding-3-small".to_string(),
        };

        let result = agent.execute(action);
        match result {
            VectorResult::IndexedVector(v) => {
                assert_eq!(v.key, "doc1");
                assert_eq!(v.vector, vec![0.1, 0.2, 0.3]);
            }
            _ => panic!("Expected IndexedVector result"),
        }
    }

    #[test]
    fn test_vector_synthesizes_distinction() {
        let (mut agent, _engine) = setup_agent();

        let local_root_before = agent.local_root().clone();

        let vector = agent.index("doc1", vec![0.1, 0.2, 0.3], "model");

        let local_root_after = agent.local_root();

        // Local root should change after indexing
        assert_ne!(local_root_before.id(), local_root_after.id());

        // The vector distinction should be different from local roots
        assert_ne!(vector.distinction.id(), local_root_before.id());
    }

    #[test]
    fn test_vectors_are_content_addressed() {
        let (mut agent, _engine) = setup_agent();

        // Indexing the same vector twice creates distinct distinctions
        // because sequence is included in synthesis
        let vector1 = agent.index("doc1", vec![0.1, 0.2, 0.3], "model");
        let vector2 = agent.index("doc2", vec![0.1, 0.2, 0.3], "model");

        // Different keys should create different distinctions
        assert_ne!(vector1.distinction.id(), vector2.distinction.id());
    }
}
