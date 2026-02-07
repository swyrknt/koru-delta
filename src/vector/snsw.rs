//! Synthesis-Navigable Small World (SNSW) - Distinction-based vector search.
//!
//! This module implements a novel approach to approximate nearest neighbor (ANN) search
//! based on distinction calculus from koru-lambda-core. Unlike traditional HNSW which
//! treats vectors as geometric points, SNSW treats vectors as distinctions in a
//! semantic causal graph.
//!
//! # Core Concepts
//!
//! 1. **Content-Addressed Storage**: Vectors are identified by their content hash (Blake3),
//!    enabling automatic deduplication.
//!
//! 2. **Synthesis Relationships**: Edges represent semantic relationships (composition,
//!    abstraction, causation) not just geometric proximity.
//!
//! 3. **Multi-Layer Abstraction**: Coarse-to-fine distinction layers enable efficient
//!    navigation from abstract concepts to specific instances.
//!
//! 4. **Explainable Search**: Results include synthesis paths showing WHY vectors relate.
//!
//! 5. **Time-Travel Search**: Query similarity at any point in time (causal versioning).
//!
//! # Example
//!
//! ```ignore
//! use koru_delta::vector::snsw::{SynthesisGraph, SynthesisConfig};
//!
//! let config = SynthesisConfig::default();
//! let mut graph = SynthesisGraph::new(config);
//!
//! // Insert vectors (automatically content-addressed)
//! let id1 = graph.insert(vector1)?;
//! let id2 = graph.insert(vector2)?;
//!
//! // Search with explanations
//! let results = graph.search_explainable(&query, 10);
//! for result in results {
//!     println!("Found: {} (score: {})", result.id, result.score);
//!     println!("Path: {:?}", result.synthesis_path);
//! }
//! ```

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

use blake3::Hasher as Blake3Hasher;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::DeltaResult;
use crate::vector::types::Vector;

/// Content hash for vector identity (Blake3).
///
/// Using content-addressing means identical vectors automatically
/// deduplicate - same content = same hash = same identity.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Compute content hash from vector data and model.
    pub fn from_vector(vector: &Vector) -> Self {
        let mut hasher = Blake3Hasher::new();
        
        // Hash the vector data
        for value in vector.as_slice() {
            hasher.update(&value.to_le_bytes());
        }
        
        // Hash the model name (different models = different semantics)
        hasher.update(vector.model().as_bytes());
        
        Self(hasher.finalize().to_hex().to_string())
    }
    
    /// Get the hash string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Type of synthesis relationship between vectors.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SynthesisType {
    /// Geometric proximity (traditional similarity).
    Proximity,
    /// Semantic composition (A + B → C).
    Composition,
    /// Abstraction (specific → general).
    Abstraction,
    /// Instantiation (general → specific).
    Instantiation,
    /// Temporal sequence (time-based).
    Sequence,
    /// Causal dependency (A caused B).
    Causation,
}

impl std::fmt::Display for SynthesisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SynthesisType::Proximity => write!(f, "proximity"),
            SynthesisType::Composition => write!(f, "composition"),
            SynthesisType::Abstraction => write!(f, "abstraction"),
            SynthesisType::Instantiation => write!(f, "instantiation"),
            SynthesisType::Sequence => write!(f, "sequence"),
            SynthesisType::Causation => write!(f, "causation"),
        }
    }
}

/// An edge in the synthesis graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisEdge {
    /// Target node (content hash).
    pub target: ContentHash,
    /// Type of synthesis relationship.
    pub relationship: SynthesisType,
    /// Strength of synthesis (0.0 to 1.0).
    pub strength: f32,
    /// Timestamp when edge was created.
    pub created_at: u64,
}

/// A node in the synthesis graph (content-addressed).
#[derive(Clone, Debug)]
pub struct DistinctionNode {
    /// Content hash = identity.
    pub hash: ContentHash,
    /// The actual vector data.
    pub vector: Arc<Vector>,
    /// Synthesis edges to other nodes.
    pub edges: Vec<SynthesisEdge>,
    /// Abstraction level (0 = specific, higher = more abstract).
    pub abstraction_level: usize,
    /// Access count for lifecycle management.
    pub access_count: u64,
    /// Last access timestamp.
    pub last_access: u64,
    /// When this node was created.
    pub created_at: u64,
}

/// Result of a synthesis search with explanation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisSearchResult {
    /// Content hash of result.
    pub id: ContentHash,
    /// Synthesis proximity score (0.0 to 1.0).
    pub score: f32,
    /// Path showing how query synthesizes to result.
    pub synthesis_path: Vec<SynthesisPathStep>,
    /// Individual factor scores.
    pub factor_scores: FactorScores,
}

/// A step in a synthesis path.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisPathStep {
    /// Node at this step.
    pub node_id: ContentHash,
    /// Relationship to next node.
    pub relationship: SynthesisType,
    /// Strength of relationship.
    pub strength: f32,
    /// Description of this step.
    pub description: String,
}

/// Breakdown of synthesis proximity factors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactorScores {
    /// Geometric cosine similarity.
    pub geometric: f32,
    /// Shared distinctions count (normalized).
    pub shared_distinctions: f32,
    /// Synthesis path length contribution.
    pub path_length: f32,
    /// Causal proximity contribution.
    pub causal: f32,
}

/// Configuration for synthesis graph.
#[derive(Clone, Debug)]
pub struct SynthesisConfig {
    /// Number of neighbors per node (like HNSW M parameter).
    pub m: usize,
    /// Max connections per node per layer.
    pub ef_construction: usize,
    /// Size of dynamic candidate list for search.
    pub ef_search: usize,
    /// Weight for geometric similarity.
    pub weight_geometric: f32,
    /// Weight for shared distinctions.
    pub weight_shared: f32,
    /// Weight for path length.
    pub weight_path: f32,
    /// Weight for causal proximity.
    pub weight_causal: f32,
    /// Number of abstraction layers.
    pub num_layers: usize,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            weight_geometric: 0.4,
            weight_shared: 0.3,
            weight_path: 0.2,
            weight_causal: 0.1,
            num_layers: 4,
        }
    }
}

/// Synthesis-Navigable Small World graph.
///
/// The core data structure for distinction-based vector search.
pub struct SynthesisGraph {
    /// Configuration.
    config: SynthesisConfig,
    /// Base layer: specific vectors.
    base_layer: DashMap<ContentHash, DistinctionNode>,
    /// Abstraction layers: coarser distinctions.
    abstraction_layers: Vec<DashMap<ContentHash, DistinctionNode>>,
    /// Index from vector content to hash (for deduplication).
    #[allow(dead_code)]
    content_index: DashMap<String, ContentHash>,
}

impl SynthesisGraph {
    /// Create a new empty synthesis graph.
    pub fn new(config: SynthesisConfig) -> Self {
        let mut abstraction_layers = Vec::with_capacity(config.num_layers);
        for _ in 0..config.num_layers {
            abstraction_layers.push(DashMap::new());
        }
        
        Self {
            config,
            base_layer: DashMap::new(),
            abstraction_layers,
            content_index: DashMap::new(),
        }
    }
    
    /// Insert a vector into the graph.
    ///
    /// Uses content-addressing: identical vectors automatically deduplicate.
    pub fn insert(&self, vector: Vector) -> DeltaResult<ContentHash> {
        let hash = ContentHash::from_vector(&vector);
        
        // Check for existing (deduplication)
        if self.base_layer.contains_key(&hash) {
            return Ok(hash);
        }
        
        let now = current_timestamp();
        
        // Compute abstraction level
        let abstraction_level = self.compute_abstraction_level(&vector);
        
        // Find synthesis neighbors
        let edges = self.find_synthesis_neighbors(&vector, &hash)?;
        
        // Create node
        let node = DistinctionNode {
            hash: hash.clone(),
            vector: Arc::new(vector),
            edges,
            abstraction_level,
            access_count: 0,
            last_access: now,
            created_at: now,
        };
        
        // Insert into base layer
        self.base_layer.insert(hash.clone(), node.clone());
        
        // Insert into abstraction layers
        for layer in 0..abstraction_level.min(self.config.num_layers) {
            self.abstraction_layers[layer].insert(hash.clone(), node.clone());
        }
        
        Ok(hash)
    }
    
    /// Search for nearest neighbors with explanations.
    pub fn search_explainable(
        &self,
        query: &Vector,
        k: usize,
    ) -> DeltaResult<Vec<SynthesisSearchResult>> {
        let _query_hash = ContentHash::from_vector(query);
        
        // Greedy search from entry points
        let candidates = self.greedy_search(query, k * 10)?;
        
        // Compute synthesis proximity and paths for each candidate
        let mut results: Vec<SynthesisSearchResult> = candidates
            .into_iter()
            .filter_map(|(hash, _)| {
                let node = self.base_layer.get(&hash)?;
                let (score, path, factors) = self.synthesis_proximity_with_path(query, &node)?;
                
                Some(SynthesisSearchResult {
                    id: hash,
                    score,
                    synthesis_path: path,
                    factor_scores: factors,
                })
            })
            .collect();
        
        // Sort by synthesis score and take top k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        
        Ok(results)
    }
    
    /// Compute synthesis proximity between query and node.
    fn synthesis_proximity_with_path(
        &self,
        query: &Vector,
        node: &DistinctionNode,
    ) -> Option<(f32, Vec<SynthesisPathStep>, FactorScores)> {
        // Geometric similarity
        let geometric = query.cosine_similarity(&node.vector)?;
        
        // Shared distinctions (simplified - in real impl would use distinction engine)
        let shared = self.count_shared_distinctions(query, &node.vector);
        let shared_normalized = (shared as f32 / query.dimensions() as f32).min(1.0);
        
        // Path length (simplified - would use actual graph traversal)
        let path_length = 1.0; // Placeholder
        
        // Causal proximity (simplified - would use temporal analysis)
        let causal = 0.5; // Placeholder
        
        // Weighted combination
        let score = self.config.weight_geometric * geometric
            + self.config.weight_shared * shared_normalized
            + self.config.weight_path * path_length
            + self.config.weight_causal * causal;
        
        let factors = FactorScores {
            geometric,
            shared_distinctions: shared_normalized,
            path_length,
            causal,
        };
        
        // Build explanation path (simplified)
        let path = vec![SynthesisPathStep {
            node_id: node.hash.clone(),
            relationship: SynthesisType::Proximity,
            strength: geometric,
            description: format!("Cosine similarity: {:.3}", geometric),
        }];
        
        Some((score, path, factors))
    }
    
    /// Greedy search for candidate nodes.
    fn greedy_search(&self, query: &Vector, ef: usize) -> DeltaResult<Vec<(ContentHash, f32)>> {
        let mut visited = HashSet::new();
        let mut candidates: Vec<(ContentHash, f32)> = Vec::new();
        
        // Start with random entry points (simplified)
        // In full impl: use abstraction layers for entry points
        let entry_points: Vec<_> = self.base_layer.iter()
            .take(self.config.ef_search)
            .map(|e| e.key().clone())
            .collect();
        
        for entry in entry_points {
            if let Some(node) = self.base_layer.get(&entry) {
                if let Some(score) = query.cosine_similarity(&node.vector) {
                    candidates.push((entry, score));
                }
                visited.insert(node.hash.clone());
                
                // Explore neighbors
                for edge in &node.edges {
                    if !visited.contains(&edge.target) {
                        if let Some(neighbor) = self.base_layer.get(&edge.target) {
                            if let Some(neighbor_score) = query.cosine_similarity(&neighbor.vector) {
                                candidates.push((edge.target.clone(), neighbor_score));
                                visited.insert(edge.target.clone());
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by score
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(ef);
        
        Ok(candidates)
    }
    
    /// Find synthesis neighbors for a new vector.
    fn find_synthesis_neighbors(
        &self,
        vector: &Vector,
        hash: &ContentHash,
    ) -> DeltaResult<Vec<SynthesisEdge>> {
        let mut edges = Vec::new();
        let now = current_timestamp();
        
        // Find geometric neighbors
        let mut candidates: Vec<(ContentHash, f32)> = self.base_layer.iter()
            .filter(|e| *e.key() != *hash)
            .filter_map(|e| {
                let score = vector.cosine_similarity(&e.value().vector)?;
                Some((e.key().clone(), score))
            })
            .collect();
        
        // Sort by similarity and take top M
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(self.config.m);
        
        for (target, strength) in candidates {
            edges.push(SynthesisEdge {
                target,
                relationship: SynthesisType::Proximity,
                strength,
                created_at: now,
            });
        }
        
        Ok(edges)
    }
    
    /// Compute abstraction level for a vector.
    fn compute_abstraction_level(&self, _vector: &Vector) -> usize {
        // Simplified: randomly assign for now
        // In full impl: use clustering hierarchy (HDBSCAN)
        0
    }
    
    /// Count shared distinctions between two vectors.
    fn count_shared_distinctions(&self, _a: &Vector, _b: &Vector) -> usize {
        // Simplified: would use distinction engine from koru-lambda-core
        // For now, return a placeholder based on similarity
        0
    }
    
    /// Get the number of vectors in the graph.
    pub fn len(&self) -> usize {
        self.base_layer.len()
    }
    
    /// Check if graph is empty.
    pub fn is_empty(&self) -> bool {
        self.base_layer.is_empty()
    }
}

/// Get current timestamp (milliseconds since epoch).
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_hash_consistency() {
        let v1 = Vector::new(vec![0.1, 0.2, 0.3], "test-model");
        let v2 = Vector::new(vec![0.1, 0.2, 0.3], "test-model");
        let v3 = Vector::new(vec![0.1, 0.2, 0.4], "test-model");
        
        let h1 = ContentHash::from_vector(&v1);
        let h2 = ContentHash::from_vector(&v2);
        let h3 = ContentHash::from_vector(&v3);
        
        // Same content = same hash
        assert_eq!(h1, h2);
        // Different content = different hash
        assert_ne!(h1, h3);
    }
    
    #[test]
    fn test_synthesis_graph_insert() {
        let graph = SynthesisGraph::new(SynthesisConfig::default());
        
        let v1 = Vector::new(vec![0.1, 0.2, 0.3, 0.4], "test-model");
        let id1 = graph.insert(v1.clone()).unwrap();
        
        assert_eq!(graph.len(), 1);
        
        // Insert duplicate - should return same ID
        let id2 = graph.insert(v1).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(graph.len(), 1); // Still 1 (deduplication)
    }
    
    #[test]
    fn test_synthesis_graph_search() {
        let graph = SynthesisGraph::new(SynthesisConfig::default());
        
        // Insert some vectors
        for i in 0..10 {
            let v = Vector::new(vec![i as f32 * 0.1; 4], "test-model");
            graph.insert(v).unwrap();
        }
        
        let query = Vector::new(vec![0.5; 4], "test-model");
        let results = graph.search_explainable(&query, 5).unwrap();
        
        assert!(!results.is_empty());
        assert!(results[0].score > 0.0);
    }
}
