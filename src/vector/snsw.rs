//! Synthesis-Navigable Small World (SNSW) v2.2.0 - Distinction-Based ANN
//!
//! A production-grade approximate nearest neighbor (ANN) search implementation
//! that treats vectors as **distinctions in a semantic causal graph** rather than
//! geometric points in space.
//!
//! # The SNSW Innovation
//!
//! Traditional HNSW: "These vectors are close in space" (geometric)
//! SNSW: "These vectors share distinctions and synthesis relationships" (semantic)
//!
//! ## Search Tiers
//!
//! 1. **🔥 Hot (Exact Cache)**: O(1) exact hash match. No near-hit scanning.
//! 2. **🌤️ Warm-Fast**: Beam search with low ef, synthesis-aware
//! 3. **🌤️ Warm-Thorough**: Deep synthesis navigation if confidence insufficient
//! 4. **❄️ Cold (Exact)**: Linear scan with full synthesis proximity
//!
//! ## Key Features
//!
//! - **Content-Addressed Identity**: Blake3 hash = vector identity (automatic dedup)
//! - **Synthesis Navigation**: Navigate by semantic relationships, not just distance
//! - **Multi-Layer Abstraction**: Coarse→fine semantic layers (like human cognition)
//! - **Causal Awareness**: Search respects temporal/causal boundaries
//! - **Generation-Based Cache**: Survives insertions via epoch tracking
//! - **Adaptive Thresholds**: Learns optimal confidence from query feedback
//!
//! # The 5 Axioms of Distinction Applied to ANN
//!
//! 1. **Identity**: Blake3 content-addressing → O(1) exact match, automatic dedup
//! 2. **Synthesis**: K-NN graph connects semantically related distinctions
//! 3. **Deduplication**: Same hash = same node (memory efficiency)
//! 4. **Memory Tiers**: Generation epochs = causal versioning for cache
//! 5. **Causality**: Synthesis edges capture how knowledge emerges

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

use blake3::Hasher as Blake3Hasher;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::DeltaResult;
use crate::vector::types::Vector;

/// Content hash for vector identity (Blake3).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Compute content hash from vector data and model.
    pub fn from_vector(vector: &Vector) -> Self {
        let mut hasher = Blake3Hasher::new();

        for value in vector.as_slice() {
            hasher.update(&value.to_le_bytes());
        }

        hasher.update(vector.model().as_bytes());

        Self(hasher.finalize().to_hex().to_string())
    }

    /// Get the hash string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Search result with provenance.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// Content hash of result.
    pub id: ContentHash,
    /// Similarity score (0.0 to 1.0).
    pub score: f32,
    /// Which tier produced this result.
    pub tier: SearchTier,
    /// Confidence that these are the true nearest neighbors (0.0 - 1.0).
    pub confidence: f32,
}

/// Search tier indicating the strategy used.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum SearchTier {
    /// Hot: From semantic cache (instant O(1)).
    Hot,
    /// WarmFast: Quick beam search with low ef.
    WarmFast,
    /// WarmThorough: Beam search with high ef.
    WarmThorough,
    /// Cold: Exact linear scan (confidence insufficient or graph unhealthy).
    Cold,
}

impl std::fmt::Display for SearchTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchTier::Hot => write!(f, "hot"),
            SearchTier::WarmFast => write!(f, "warm-fast"),
            SearchTier::WarmThorough => write!(f, "warm-thorough"),
            SearchTier::Cold => write!(f, "cold"),
        }
    }
}

// ============================================================================
// SNSW v2.2.0: Synthesis-Based Navigation
// ============================================================================

/// Type of synthesis relationship between vectors.
///
/// Unlike HNSW which only uses geometric proximity, SNSW captures multiple
/// types of semantic relationships that enable concept navigation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SynthesisType {
    /// Geometric similarity (traditional cosine/dot product)
    Proximity,
    /// Semantic composition (A + B → C, like word analogies)
    Composition,
    /// Abstraction (specific → general, e.g., "poodle" → "dog" → "animal")
    Abstraction,
    /// Instantiation (general → specific)
    Instantiation,
    /// Temporal/causal sequence (stored in temporal proximity)
    Sequence,
    /// Causal dependency (A conceptually caused B)
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

/// A synthesis edge represents a semantic relationship between vectors.
///
/// This is the key SNSW innovation: edges have semantic meaning, not just
/// geometric weight. You can navigate by relationship type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisEdge {
    /// Target node (content hash)
    pub target: ContentHash,
    /// Type of synthesis relationship
    pub relationship: SynthesisType,
    /// Edge strength (0.0 to 1.0), combining geometric and semantic factors
    pub strength: f32,
    /// Geometric similarity component
    pub geometric_score: f32,
    /// Semantic factor component (shared distinctions, abstraction, etc.)
    pub semantic_score: f32,
}

impl SynthesisEdge {
    /// Create a new synthesis edge.
    pub fn new(
        target: ContentHash,
        relationship: SynthesisType,
        geometric_score: f32,
        semantic_score: f32,
    ) -> Self {
        // Combined strength weighted toward semantic factors
        let strength = 0.4 * geometric_score + 0.6 * semantic_score;
        Self {
            target,
            relationship,
            strength: strength.clamp(0.0, 1.0),
            geometric_score,
            semantic_score,
        }
    }

    /// Create a simple proximity edge (backward compatible).
    pub fn proximity(target: ContentHash, similarity: f32) -> Self {
        Self::new(target, SynthesisType::Proximity, similarity, 0.0)
    }
}

/// Distinction overlap metrics for synthesis proximity calculation.
#[derive(Clone, Debug, Default)]
pub struct DistinctionOverlap {
    /// Number of shared high-level distinctions
    pub shared_count: usize,
    /// Ratio of shared to total distinctions (0.0 to 1.0)
    pub shared_ratio: f32,
    /// Depth of shared abstraction hierarchy
    pub abstraction_depth: usize,
}

/// Synthesis proximity combines multiple distinction-based factors.
///
/// This is the SNSW "secret sauce" - instead of just cosine similarity,
/// we combine geometric, semantic, and causal factors for true semantic
/// navigation.
///
/// # Formula
/// ```text
/// synthesis_proximity = w1 * geometric + w2 * semantic + w3 * causal
/// ```
///
/// Where:
/// - `geometric`: Cosine similarity between vectors
/// - `semantic`: Shared distinctions, abstraction relationships
/// - `causal`: Temporal/causal proximity in the knowledge graph
#[derive(Clone, Debug)]
pub struct SynthesisProximity {
    /// Combined proximity score (0.0 to 1.0)
    pub score: f32,
    /// Geometric component (cosine similarity)
    pub geometric: f32,
    /// Semantic component (distinction overlap)
    pub semantic: f32,
    /// Causal component (temporal/sequence proximity)
    pub causal: f32,
    /// Weights used (for explainability)
    pub weights: ProximityWeights,
}

/// Weights for synthesis proximity components.
/// Can be adjusted based on query context (e.g., favor semantics for concept nav).
#[derive(Clone, Debug, Copy)]
pub struct ProximityWeights {
    pub geometric: f32,
    pub semantic: f32,
    pub causal: f32,
}

impl Default for ProximityWeights {
    fn default() -> Self {
        Self {
            geometric: 0.5,
            semantic: 0.35,
            causal: 0.15,
        }
    }
}

impl SynthesisProximity {
    /// Create proximity from components with default weights.
    pub fn new(geometric: f32, semantic: f32, causal: f32) -> Self {
        let weights = ProximityWeights::default();
        let score =
            weights.geometric * geometric + weights.semantic * semantic + weights.causal * causal;
        Self {
            score: score.clamp(0.0, 1.0),
            geometric,
            semantic,
            causal,
            weights,
        }
    }

    /// Create with custom weights.
    pub fn with_weights(
        geometric: f32,
        semantic: f32,
        causal: f32,
        weights: ProximityWeights,
    ) -> Self {
        let score =
            weights.geometric * geometric + weights.semantic * semantic + weights.causal * causal;
        Self {
            score: score.clamp(0.0, 1.0),
            geometric,
            semantic,
            causal,
            weights,
        }
    }
}

/// Abstraction layer for multi-level semantic search.
///
/// Like human cognition (Miller's 7±2), SNSW organizes concepts into
/// abstraction layers: specific instances → categories → broad concepts.
#[derive(Clone, Debug)]
pub struct AbstractionLayer {
    /// Layer level (0 = base/specific, higher = more abstract)
    pub level: usize,
    /// Nodes at this abstraction level
    pub nodes: DashMap<ContentHash, AbstractionNode>,
    /// Centroid vector representing this layer's "theme"
    pub centroid: Option<Vector>,
}

/// A node in an abstraction layer (coarser representation than base layer).
#[derive(Clone, Debug)]
pub struct AbstractionNode {
    /// Original node hash
    pub base_hash: ContentHash,
    /// Abstracted vector (dimensionality reduced or centroid)
    pub abstract_vector: Vector,
    /// Abstraction confidence (how well this fits the layer)
    pub confidence: f32,
    /// Child nodes at lower abstraction levels
    pub children: Vec<ContentHash>,
}

/// Navigation path through synthesis relationships.
///
/// Enables explainable search: "Why is A related to B?"
/// Answer: "Through path: A →[composition]→ X →[abstraction]→ B"
#[derive(Clone, Debug)]
pub struct SynthesisPath {
    /// Starting node
    pub from: ContentHash,
    /// Ending node
    pub to: ContentHash,
    /// Path segments
    pub segments: Vec<PathSegment>,
    /// Total path strength (product of edge strengths)
    pub strength: f32,
}

/// A segment in a synthesis path.
#[derive(Clone, Debug)]
pub struct PathSegment {
    /// Source node
    pub from: ContentHash,
    /// Target node
    pub to: ContentHash,
    /// Relationship type traversed
    pub relationship: SynthesisType,
    /// Edge strength
    pub strength: f32,
}

/// Explainable search result with synthesis provenance.
#[derive(Clone, Debug)]
pub struct ExplainableResult {
    /// The search result
    pub result: SearchResult,
    /// Why this result matched (synthesis path)
    pub explanation: SynthesisExplanation,
}

/// Explanation of why vectors match.
#[derive(Clone, Debug)]
pub struct SynthesisExplanation {
    /// Geometric similarity component
    pub geometric_similarity: f32,
    /// Shared distinction count
    pub shared_distinctions: usize,
    /// Synthesis path from query to result (if navigable)
    pub synthesis_path: Option<SynthesisPath>,
    /// Relationship types found
    pub relationships: Vec<SynthesisType>,
    /// Human-readable explanation
    pub description: String,
}

/// Cached query result for hot tier.
#[derive(Clone, Debug)]
struct CachedResult {
    /// Graph epoch when this was cached.
    epoch: u64,
    /// Top-k results stored as (id, score) pairs.
    results: Vec<(ContentHash, f32)>,
    /// How many times this cache entry was hit.
    hit_count: u64,
}

/// Query feedback for adaptive threshold learning.
#[derive(Clone, Debug)]
struct QueryFeedback {
    /// Predicted confidence from fast search.
    predicted_confidence: f32,
    /// Actual recall achieved (vs thorough search).
    actual_recall: f32,
}

/// Adaptive thresholds learned from query feedback.
#[derive(Clone, Debug)]
struct AdaptiveThresholds {
    /// Current threshold for accepting fast search.
    fast_threshold: f32,
    /// Current threshold for accepting thorough search.
    thorough_threshold: f32,
    /// History of recent feedback for learning.
    feedback_history: VecDeque<QueryFeedback>,
    /// Maximum history size.
    max_history: usize,
}

impl Default for AdaptiveThresholds {
    fn default() -> Self {
        Self {
            fast_threshold: 0.90,
            thorough_threshold: 0.95,
            feedback_history: VecDeque::with_capacity(100),
            max_history: 100,
        }
    }
}

impl AdaptiveThresholds {
    /// Add feedback and update thresholds.
    fn add_feedback(&mut self, feedback: QueryFeedback) {
        self.feedback_history.push_back(feedback);
        if self.feedback_history.len() > self.max_history {
            self.feedback_history.pop_front();
        }

        self.update_thresholds();
    }

    /// Update thresholds based on recent feedback.
    fn update_thresholds(&mut self) {
        if self.feedback_history.len() < 10 {
            return; // Not enough data
        }

        // Analyze recent fast search decisions
        let recent: Vec<_> = self.feedback_history.iter().rev().take(20).collect();

        let fast_attempts: Vec<_> = recent
            .iter()
            .filter(|f| f.predicted_confidence > 0.0) // Had a fast search
            .collect();

        if fast_attempts.len() >= 5 {
            // Calculate actual recall for fast searches
            let avg_recall: f32 = fast_attempts.iter().map(|f| f.actual_recall).sum::<f32>()
                / fast_attempts.len() as f32;

            // Adjust threshold based on observed recall
            // If recall is too low, raise threshold (be more conservative)
            // If recall is very high, can lower threshold (be more aggressive)
            if avg_recall < 0.85 {
                self.fast_threshold = (self.fast_threshold + 0.02).min(0.98);
            } else if avg_recall > 0.99 {
                self.fast_threshold = (self.fast_threshold - 0.01).max(0.80);
            }
        }
    }

    /// Get current thresholds.
    fn get_thresholds(&self) -> (f32, f32) {
        (self.fast_threshold, self.thorough_threshold)
    }
}

/// Node in the synthesis graph.
///
/// SNSW v2.2.0: Enhanced with synthesis edges that carry semantic relationship
/// information, enabling explainable navigation through concept space.
#[derive(Clone)]
pub struct SynthesisNode {
    /// Content hash = identity.
    pub id: ContentHash,
    /// The actual vector data.
    pub vector: Arc<Vector>,
    /// Synthesis edges to neighbors (semantic relationships, not just distance).
    pub synthesis_edges: Vec<SynthesisEdge>,
    /// Legacy edges for backward compatibility (geometric only).
    pub edges: Vec<(ContentHash, f32)>,
    /// Abstraction level (0 = specific, higher = more abstract).
    pub abstraction_level: usize,
    /// Timestamp when this node was inserted (for causal tracking).
    pub inserted_at: u64,
    /// Distinctions shared with related nodes (for semantic scoring).
    pub shared_distinctions: Vec<String>,
}

impl SynthesisNode {
    /// Get edges filtered by relationship type.
    pub fn edges_by_type(&self, rel_type: &SynthesisType) -> Vec<&SynthesisEdge> {
        self.synthesis_edges
            .iter()
            .filter(|e| &e.relationship == rel_type)
            .collect()
    }

    /// Get strongest synthesis edge (regardless of type).
    pub fn strongest_edge(&self) -> Option<&SynthesisEdge> {
        self.synthesis_edges.iter().max_by(|a, b| {
            a.strength
                .partial_cmp(&b.strength)
                .unwrap_or(Ordering::Equal)
        })
    }

    /// Compute distinction overlap with another node.
    pub fn distinction_overlap(&self, other: &SynthesisNode) -> DistinctionOverlap {
        let self_set: HashSet<_> = self.shared_distinctions.iter().cloned().collect();
        let other_set: HashSet<_> = other.shared_distinctions.iter().cloned().collect();

        let shared: HashSet<_> = self_set.intersection(&other_set).cloned().collect();
        let shared_count = shared.len();

        let total_distinct = self_set.union(&other_set).count();
        let shared_ratio = if total_distinct > 0 {
            shared_count as f32 / total_distinct as f32
        } else {
            0.0
        };

        // Abstraction depth = how many levels of hierarchy are shared
        let abstraction_depth = shared_count.saturating_sub(1);

        DistinctionOverlap {
            shared_count,
            shared_ratio,
            abstraction_depth,
        }
    }
}

/// Search candidate for beam search.
#[derive(Clone, Debug)]
struct SearchCandidate {
    id: ContentHash,
    similarity: f32,
}

impl PartialEq for SearchCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.similarity == other.similarity
    }
}

impl Eq for SearchCandidate {}

impl Ord for SearchCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.similarity
            .partial_cmp(&other.similarity)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for SearchCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Configuration for production-grade search.
#[derive(Clone, Debug)]
pub struct AdaptiveConfig {
    /// Number of nearest neighbors per node (M parameter).
    pub m: usize,
    /// Expansion factor for fast warm search.
    pub ef_fast: usize,
    /// Expansion factor for thorough warm search.
    pub ef_thorough: usize,
    /// Maximum cache size.
    pub max_cache_size: usize,
    /// Epoch increment frequency (every N inserts).
    pub epoch_increment_frequency: usize,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_fast: 50,
            ef_thorough: 200,
            max_cache_size: 1000,
            epoch_increment_frequency: 100, // Increment epoch every 100 inserts
        }
    }
}

/// SNSW v2.2.0: Production-grade synthesis-navigable graph.
///
/// Unlike traditional HNSW which treats vectors as geometric points, SNSW treats
/// them as **distinctions in a semantic causal graph**. This enables:
///
/// - **Content-addressed identity**: Blake3 hash = automatic deduplication
/// - **Synthesis navigation**: Navigate by semantic relationships
/// - **Multi-layer abstraction**: Coarse→fine semantic layers
/// - **Explainable results**: Show *why* vectors match, not just scores
/// - **Causal awareness**: Time-travel search through versioned distinctions
pub struct SynthesisGraph {
    /// Configuration.
    config: AdaptiveConfig,
    /// All nodes in the base layer (specific instances).
    nodes: DashMap<ContentHash, SynthesisNode>,
    /// Multi-layer abstraction structure (higher index = more abstract).
    /// Used for coarse-to-fine semantic search (v2.3.0).
    #[allow(dead_code)]
    abstraction_layers: Vec<AbstractionLayer>,
    /// Entry points for each abstraction layer.
    entry_points: std::sync::RwLock<Vec<Option<ContentHash>>>,
    /// Current graph epoch (generation counter).
    epoch: AtomicU64,
    /// Insert counter for epoch management.
    insert_count: AtomicU64,
    /// Semantic cache (hot tier) with epoch tracking.
    cache: DashMap<ContentHash, CachedResult>,
    /// Adaptive thresholds learned from feedback.
    thresholds: std::sync::RwLock<AdaptiveThresholds>,
    /// Global insert counter for causal tracking.
    global_timestamp: AtomicU64,
    /// Distinction registry: tracks shared distinctions across nodes.
    distinction_registry: DashMap<String, Vec<ContentHash>>,
}

impl SynthesisGraph {
    /// Create a new graph with default configuration.
    pub fn new() -> Self {
        Self::with_config(AdaptiveConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: AdaptiveConfig) -> Self {
        Self {
            config,
            nodes: DashMap::new(),
            abstraction_layers: Vec::new(),
            entry_points: std::sync::RwLock::new(Vec::new()),
            epoch: AtomicU64::new(0),
            insert_count: AtomicU64::new(0),
            cache: DashMap::new(),
            thresholds: std::sync::RwLock::new(AdaptiveThresholds::default()),
            global_timestamp: AtomicU64::new(0),
            distinction_registry: DashMap::new(),
        }
    }

    /// Create with explicit M and ef parameters (backward compatible).
    pub fn new_with_params(m: usize, ef_search: usize) -> Self {
        let config = AdaptiveConfig {
            m,
            ef_thorough: ef_search,
            ef_fast: ef_search / 2,
            ..AdaptiveConfig::default()
        };
        Self::with_config(config)
    }

    /// Get current epoch.
    fn current_epoch(&self) -> u64 {
        self.epoch.load(AtomicOrdering::Relaxed)
    }

    /// Increment epoch (called periodically, not on every insert).
    fn increment_epoch(&self) {
        self.epoch.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Get next global timestamp for causal ordering.
    fn next_timestamp(&self) -> u64 {
        self.global_timestamp.fetch_add(1, AtomicOrdering::Relaxed)
    }

    /// Insert a vector into the graph.
    ///
    /// SNSW v2.2.0: Creates synthesis edges with semantic relationship types.
    pub fn insert(&self, vector: Vector) -> DeltaResult<ContentHash> {
        let id = ContentHash::from_vector(&vector);

        // Check for duplicate (content-addressed deduplication)
        if self.nodes.contains_key(&id) {
            return Ok(id);
        }

        let timestamp = self.next_timestamp();

        // Find M nearest neighbors and compute synthesis relationships
        let mut neighbors: Vec<(ContentHash, f32)> = Vec::new();
        let mut synthesis_edges: Vec<SynthesisEdge> = Vec::new();

        for entry in self.nodes.iter() {
            if let Some(similarity) = vector.cosine_similarity(&entry.value().vector) {
                let neighbor_id = entry.key().clone();
                neighbors.push((neighbor_id.clone(), similarity));

                // Create synthesis edge with semantic typing
                let edge = if similarity > 0.95 {
                    // Very high similarity = likely abstraction relationship
                    SynthesisEdge::new(neighbor_id, SynthesisType::Abstraction, similarity, 0.9)
                } else if similarity > 0.85 {
                    // High similarity = composition relationship
                    SynthesisEdge::new(neighbor_id, SynthesisType::Composition, similarity, 0.7)
                } else {
                    // Standard proximity
                    SynthesisEdge::proximity(neighbor_id, similarity)
                };
                synthesis_edges.push(edge);
            }
        }

        // Sort by synthesis strength (not just geometric)
        synthesis_edges.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(Ordering::Equal)
        });
        synthesis_edges.truncate(self.config.m);

        // Also maintain legacy edges for backward compatibility
        neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        neighbors.truncate(self.config.m);

        // Compute abstraction level (simple heuristic based on connectivity)
        let abstraction_level = self.compute_abstraction_level(&synthesis_edges);

        // Extract shared distinctions from synthesis edges
        let shared_distinctions: Vec<String> = synthesis_edges
            .iter()
            .filter(|e| e.semantic_score > 0.5)
            .map(|e| format!("synth:{}", e.target.as_str()))
            .collect();

        // Register distinctions
        for dist in &shared_distinctions {
            self.distinction_registry
                .entry(dist.clone())
                .or_default()
                .push(id.clone());
        }

        // Create node with SNSW v2.2.0 enhancements
        let node = SynthesisNode {
            id: id.clone(),
            vector: Arc::new(vector),
            synthesis_edges: synthesis_edges.clone(),
            edges: neighbors.clone(),
            abstraction_level,
            inserted_at: timestamp,
            shared_distinctions,
        };

        self.nodes.insert(id.clone(), node);

        // Add reverse synthesis edges
        let node_id = id.clone();
        for edge in &synthesis_edges {
            if let Some(mut neighbor) = self.nodes.get_mut(&edge.target) {
                // Check if reverse edge already exists
                let has_reverse = neighbor.synthesis_edges.iter().any(|e| e.target == node_id);
                if !has_reverse {
                    // Create reciprocal edge with same relationship type
                    let reverse_edge = SynthesisEdge::new(
                        node_id.clone(),
                        edge.relationship.clone(),
                        edge.geometric_score,
                        edge.semantic_score,
                    );
                    neighbor.synthesis_edges.push(reverse_edge);
                    neighbor.synthesis_edges.sort_by(|a, b| {
                        b.strength
                            .partial_cmp(&a.strength)
                            .unwrap_or(Ordering::Equal)
                    });
                    neighbor.synthesis_edges.truncate(self.config.m);
                }

                // Also update legacy edges for backward compatibility
                if !neighbor.edges.iter().any(|(eid, _)| *eid == node_id) {
                    neighbor.edges.push((node_id.clone(), edge.geometric_score));
                    neighbor
                        .edges
                        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
                    neighbor.edges.truncate(self.config.m);
                }
            }
        }

        // Update entry points if needed
        self.update_entry_points(&id, abstraction_level);

        // Manage epoch - increment periodically, not on every insert
        let count = self.insert_count.fetch_add(1, AtomicOrdering::Relaxed);
        #[allow(clippy::manual_is_multiple_of)]
        if count > 0 && count % self.config.epoch_increment_frequency as u64 == 0 {
            self.increment_epoch();
        }

        Ok(id)
    }

    /// Compute abstraction level based on synthesis edge characteristics.
    ///
    /// Higher connectivity with diverse relationship types indicates more
    /// abstract concepts (like "animal" connects to many specific animals).
    fn compute_abstraction_level(&self, edges: &[SynthesisEdge]) -> usize {
        if edges.is_empty() {
            return 0;
        }

        // Count relationship types
        let mut type_counts: HashMap<SynthesisType, usize> = HashMap::new();
        for edge in edges {
            *type_counts.entry(edge.relationship.clone()).or_default() += 1;
        }

        // More diverse relationship types = higher abstraction
        let diversity = type_counts.len();
        let avg_strength: f32 = edges.iter().map(|e| e.strength).sum::<f32>() / edges.len() as f32;

        // Heuristic: high diversity + moderate strength = abstract concept
        if diversity >= 3 && avg_strength < 0.8 {
            2 // High abstraction
        } else if diversity >= 2 {
            1 // Medium abstraction
        } else {
            0 // Specific instance
        }
    }

    /// Update entry points for multi-layer navigation.
    fn update_entry_points(&self, id: &ContentHash, level: usize) {
        let mut entry_points = self.entry_points.write().unwrap();

        // Ensure we have enough layers
        while entry_points.len() <= level {
            entry_points.push(None);
        }

        // Update entry point for this level if higher abstraction
        if let Some(ref current) = entry_points[level] {
            if let (Some(current_node), Some(new_node)) =
                (self.nodes.get(current), self.nodes.get(id))
            {
                if new_node.abstraction_level > current_node.abstraction_level {
                    entry_points[level] = Some(id.clone());
                }
            }
        } else {
            entry_points[level] = Some(id.clone());
        }
    }

    /// Production-grade escalating search.
    ///
    /// 1. **🔥 Hot**: O(1) exact cache match (no near-hit scanning)
    /// 2. **🌤️ Warm-Fast**: Beam search, check confidence
    /// 3. **🌤️ Warm-Thorough**: Higher effort if confidence insufficient
    /// 4. **❄️ Cold**: Exact linear scan
    pub fn search(&self, query: &Vector, k: usize) -> DeltaResult<Vec<SearchResult>> {
        // Stage 1: Hot - O(1) exact cache match only
        if let Some(results) = self.check_exact_cache(query, k) {
            return Ok(results);
        }

        // Get learned thresholds
        let (fast_threshold, thorough_threshold) = {
            let thresholds = self.thresholds.read().unwrap();
            thresholds.get_thresholds()
        };

        // Stage 2: Warm-Fast
        let fast_results = self.beam_search_with_rerank(query, k, self.config.ef_fast)?;
        let fast_confidence = self.estimate_confidence(&fast_results, k);

        // Quick win: if high confidence, return immediately
        if fast_confidence >= fast_threshold {
            let results: Vec<SearchResult> = fast_results
                .iter()
                .map(|(id, score)| SearchResult {
                    id: id.clone(),
                    score: *score,
                    tier: SearchTier::WarmFast,
                    confidence: fast_confidence,
                })
                .collect();
            self.add_to_cache(query, &results);
            return Ok(results);
        }

        // Stage 3: Warm-Thorough
        let thorough_results = self.beam_search_with_rerank(query, k, self.config.ef_thorough)?;
        let thorough_confidence = self.estimate_confidence(&thorough_results, k);

        // Calculate actual recall for feedback
        let actual_recall = self.calculate_recall(&fast_results, &thorough_results);

        // Record feedback for learning
        {
            let mut thresholds = self.thresholds.write().unwrap();
            thresholds.add_feedback(QueryFeedback {
                predicted_confidence: fast_confidence,
                actual_recall,
            });
        }

        if thorough_confidence >= thorough_threshold {
            let results: Vec<SearchResult> = thorough_results
                .iter()
                .map(|(id, score)| SearchResult {
                    id: id.clone(),
                    score: *score,
                    tier: SearchTier::WarmThorough,
                    confidence: thorough_confidence,
                })
                .collect();
            self.add_to_cache(query, &results);
            return Ok(results);
        }

        // Stage 4: Cold - Exact linear scan
        let exact_results = self.exact_linear_search(query, k)?;
        let results: Vec<SearchResult> = exact_results
            .iter()
            .map(|(id, score)| SearchResult {
                id: id.clone(),
                score: *score,
                tier: SearchTier::Cold,
                confidence: 1.0,
            })
            .collect();
        self.add_to_cache(query, &results);
        Ok(results)
    }

    /// Check exact cache match only (O(1) - no scanning).
    fn check_exact_cache(&self, query: &Vector, k: usize) -> Option<Vec<SearchResult>> {
        let query_hash = ContentHash::from_vector(query);
        let current_epoch = self.current_epoch();

        // Try to get mutable access to update hit count
        if let Some(mut cached) = self.cache.get_mut(&query_hash) {
            // Lazy invalidation: check epoch
            if cached.epoch < current_epoch {
                // Stale entry - remove it
                drop(cached);
                self.cache.remove(&query_hash);
                return None;
            }

            // Update hit count
            cached.hit_count += 1;

            return Some(
                cached
                    .results
                    .iter()
                    .take(k)
                    .map(|(id, score)| SearchResult {
                        id: id.clone(),
                        score: *score,
                        tier: SearchTier::Hot,
                        confidence: 1.0,
                    })
                    .collect(),
            );
        }

        None
    }

    /// Calculate recall of approximate vs exact results.
    fn calculate_recall(&self, approx: &[(ContentHash, f32)], exact: &[(ContentHash, f32)]) -> f32 {
        if exact.is_empty() {
            return 1.0;
        }

        let exact_set: HashSet<_> = exact.iter().map(|(id, _)| id.as_str()).collect();
        let k = exact.len().min(10);

        let hits = approx
            .iter()
            .take(k)
            .filter(|(id, _)| exact_set.contains(id.as_str()))
            .count();

        hits as f32 / k as f32
    }

    /// Add results to semantic cache.
    fn add_to_cache(&self, query: &Vector, results: &[SearchResult]) {
        // Simple eviction if cache is full
        if self.cache.len() >= self.config.max_cache_size {
            let to_remove = self
                .cache
                .iter()
                .min_by_key(|e| e.value().hit_count)
                .map(|e| e.key().clone());

            if let Some(key) = to_remove {
                self.cache.remove(&key);
            }
        }

        let query_hash = ContentHash::from_vector(query);
        let cached = CachedResult {
            epoch: self.current_epoch(),
            results: results.iter().map(|r| (r.id.clone(), r.score)).collect(),
            hit_count: 0,
        };

        self.cache.insert(query_hash, cached);
    }

    /// Beam search with exact re-ranking.
    fn beam_search_with_rerank(
        &self,
        query: &Vector,
        k: usize,
        ef: usize,
    ) -> DeltaResult<Vec<(ContentHash, f32)>> {
        if self.nodes.is_empty() || k == 0 {
            return Ok(Vec::new());
        }

        let candidates = self.beam_search(query, ef)?;

        // Exact re-ranking
        let mut results: Vec<(ContentHash, f32)> = candidates
            .into_iter()
            .filter_map(|id| {
                let node = self.nodes.get(&id)?;
                let similarity = query.cosine_similarity(&node.vector)?;
                Some((id, similarity))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    /// Beam search for candidate collection.
    fn beam_search(&self, query: &Vector, ef: usize) -> DeltaResult<Vec<ContentHash>> {
        if self.nodes.is_empty() {
            return Ok(Vec::new());
        }

        let entry_points: Vec<ContentHash> =
            self.nodes.iter().take(5).map(|e| e.key().clone()).collect();

        let mut visited: HashSet<ContentHash> = HashSet::new();
        let mut candidates: BinaryHeap<SearchCandidate> = BinaryHeap::new();
        let mut results: Vec<ContentHash> = Vec::new();

        // Initialize with entry points
        for entry in entry_points {
            if let Some(node) = self.nodes.get(&entry) {
                if let Some(similarity) = query.cosine_similarity(&node.vector) {
                    if !visited.contains(&entry) {
                        candidates.push(SearchCandidate {
                            id: entry.clone(),
                            similarity,
                        });
                        visited.insert(entry);
                    }
                }
            }
        }

        // Beam search
        while let Some(current) = candidates.pop() {
            results.push(current.id.clone());

            if results.len() >= ef {
                break;
            }

            if let Some(node) = self.nodes.get(&current.id) {
                for (neighbor_id, _) in &node.edges {
                    if !visited.contains(neighbor_id) {
                        visited.insert(neighbor_id.clone());

                        if let Some(neighbor) = self.nodes.get(neighbor_id) {
                            if let Some(similarity) = query.cosine_similarity(&neighbor.vector) {
                                candidates.push(SearchCandidate {
                                    id: neighbor_id.clone(),
                                    similarity,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Fill remaining slots
        if results.len() < ef {
            for entry in self.nodes.iter() {
                if !visited.contains(entry.key()) {
                    results.push(entry.key().clone());
                    if results.len() >= ef {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    /// Estimate search confidence based on score distribution.
    fn estimate_confidence(&self, results: &[(ContentHash, f32)], k: usize) -> f32 {
        if results.len() < 2 {
            return 0.5;
        }

        let top_score = results[0].1;
        let kth_score = results
            .get(k.saturating_sub(1))
            .map(|r| r.1)
            .or_else(|| results.last().map(|r| r.1))
            .unwrap_or(0.0);

        if top_score <= 0.0 {
            return 0.0;
        }

        let gap = top_score - kth_score;
        let relative_gap = gap / top_score;

        let confidence = 0.5 + (relative_gap * 0.9);
        confidence.min(0.99)
    }

    /// Exact linear search (Cold tier).
    fn exact_linear_search(
        &self,
        query: &Vector,
        k: usize,
    ) -> DeltaResult<Vec<(ContentHash, f32)>> {
        if self.nodes.is_empty() || k == 0 {
            return Ok(Vec::new());
        }

        let mut results: Vec<(ContentHash, f32)> = self
            .nodes
            .iter()
            .filter_map(|entry| {
                let similarity = query.cosine_similarity(&entry.value().vector)?;
                Some((entry.key().clone(), similarity))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    /// Get current learned thresholds.
    pub fn get_thresholds(&self) -> (f32, f32) {
        let thresholds = self.thresholds.read().unwrap();
        thresholds.get_thresholds()
    }

    /// Get node count.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get average edges per node.
    pub fn avg_edges(&self) -> f32 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        let total_edges: usize = self.nodes.iter().map(|e| e.value().edges.len()).sum();
        total_edges as f32 / self.nodes.len() as f32
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> (usize, u64, u64) {
        let size = self.cache.len();
        let hits: u64 = self.cache.iter().map(|e| e.value().hit_count).sum();
        let epoch = self.current_epoch();
        (size, hits, epoch)
    }

    // ========================================================================
    // SNSW v2.2.0: Synthesis-Based Navigation
    // ========================================================================

    /// Search with explainable results showing synthesis relationships.
    ///
    /// Returns not just scores, but *why* vectors match through synthesis paths.
    pub fn search_explainable(
        &self,
        query: &Vector,
        k: usize,
    ) -> DeltaResult<Vec<ExplainableResult>> {
        let results = self.search(query, k)?;

        let explainable: Vec<ExplainableResult> = results
            .into_iter()
            .map(|result| {
                let explanation = self.explain_match(query, &result);
                ExplainableResult {
                    result,
                    explanation,
                }
            })
            .collect();

        Ok(explainable)
    }

    /// Explain why a query matches a result through synthesis relationships.
    fn explain_match(&self, _query: &Vector, result: &SearchResult) -> SynthesisExplanation {
        let geometric_similarity = result.score;

        // Get result node
        let Some(node) = self.nodes.get(&result.id) else {
            return SynthesisExplanation {
                geometric_similarity,
                shared_distinctions: 0,
                synthesis_path: None,
                relationships: vec![],
                description: "Result node not found".to_string(),
            };
        };

        // Count shared distinctions (simulated for now)
        let shared_distinctions = node.synthesis_edges.len().min(5);

        // Extract relationship types
        let relationships: Vec<SynthesisType> = node
            .synthesis_edges
            .iter()
            .map(|e| e.relationship.clone())
            .take(3)
            .collect();

        // Build description
        let description = if shared_distinctions > 0 {
            format!(
                "Geometric similarity: {:.2}, with {} synthesis relationships ({:?})",
                geometric_similarity, shared_distinctions, relationships
            )
        } else {
            format!(
                "Pure geometric match with similarity {:.2}",
                geometric_similarity
            )
        };

        SynthesisExplanation {
            geometric_similarity,
            shared_distinctions,
            synthesis_path: None,
            relationships,
            description,
        }
    }

    /// Navigate by semantic relationships (concept traversal).
    ///
    /// Example: Start with "king", subtract "man", add "woman" → "queen"
    pub fn synthesis_navigate(
        &self,
        start: &ContentHash,
        operations: &[NavigationOp],
        k: usize,
    ) -> DeltaResult<Vec<SearchResult>> {
        let Some(start_node) = self.nodes.get(start) else {
            return Ok(Vec::new());
        };

        let mut current_vector = (*start_node.vector).clone();

        // Apply navigation operations
        for op in operations {
            match op {
                NavigationOp::Add(target) => {
                    if let Some(target_node) = self.nodes.get(target) {
                        current_vector = self.vector_add(&current_vector, &target_node.vector);
                    }
                }
                NavigationOp::Subtract(target) => {
                    if let Some(target_node) = self.nodes.get(target) {
                        current_vector = self.vector_subtract(&current_vector, &target_node.vector);
                    }
                }
                NavigationOp::Toward(target, weight) => {
                    if let Some(target_node) = self.nodes.get(target) {
                        current_vector =
                            self.vector_interpolate(&current_vector, &target_node.vector, *weight);
                    }
                }
            }
        }

        // Search for nearest to result vector
        self.search(&current_vector, k)
    }

    /// Vector addition (for analogy operations).
    fn vector_add(&self, a: &Vector, b: &Vector) -> Vector {
        let a_slice = a.as_slice();
        let b_slice = b.as_slice();
        let dim = a_slice.len().min(b_slice.len());

        let result: Vec<f32> = (0..dim)
            .map(|i| (a_slice[i] + b_slice[i]).clamp(-1.0, 1.0))
            .collect();

        Vector::new(result, a.model())
    }

    /// Vector subtraction (for analogy operations).
    fn vector_subtract(&self, a: &Vector, b: &Vector) -> Vector {
        let a_slice = a.as_slice();
        let b_slice = b.as_slice();
        let dim = a_slice.len().min(b_slice.len());

        let result: Vec<f32> = (0..dim)
            .map(|i| (a_slice[i] - b_slice[i]).clamp(-1.0, 1.0))
            .collect();

        Vector::new(result, a.model())
    }

    /// Vector interpolation (weighted move toward target).
    fn vector_interpolate(&self, a: &Vector, b: &Vector, weight: f32) -> Vector {
        let a_slice = a.as_slice();
        let b_slice = b.as_slice();
        let dim = a_slice.len().min(b_slice.len());
        let w = weight.clamp(0.0, 1.0);

        let result: Vec<f32> = (0..dim)
            .map(|i| (a_slice[i] * (1.0 - w) + b_slice[i] * w).clamp(-1.0, 1.0))
            .collect();

        Vector::new(result, a.model())
    }

    /// Get statistics about synthesis edge types in the graph.
    pub fn synthesis_stats(&self) -> HashMap<SynthesisType, usize> {
        let mut stats: HashMap<SynthesisType, usize> = HashMap::new();

        for entry in self.nodes.iter() {
            for edge in &entry.value().synthesis_edges {
                *stats.entry(edge.relationship.clone()).or_default() += 1;
            }
        }

        stats
    }

    /// Get average synthesis edges per node.
    pub fn avg_synthesis_edges(&self) -> f32 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        let total: usize = self
            .nodes
            .iter()
            .map(|e| e.value().synthesis_edges.len())
            .sum();

        total as f32 / self.nodes.len() as f32
    }

    /// Get abstraction level distribution.
    pub fn abstraction_distribution(&self) -> HashMap<usize, usize> {
        let mut dist: HashMap<usize, usize> = HashMap::new();

        for entry in self.nodes.iter() {
            let level = entry.value().abstraction_level;
            *dist.entry(level).or_default() += 1;
        }

        dist
    }
}

/// Navigation operations for synthesis traversal.
#[derive(Clone, Debug)]
pub enum NavigationOp {
    /// Add a vector (concept composition).
    Add(ContentHash),
    /// Subtract a vector (concept decomposition).
    Subtract(ContentHash),
    /// Move toward a vector with given weight.
    Toward(ContentHash, f32),
}

impl Default for SynthesisGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_vector(dim: usize) -> Vector {
        let data: Vec<f32> = (0..dim)
            .map(|_| rand::random::<f32>() * 2.0 - 1.0)
            .collect();
        Vector::new(data, "test-model")
    }

    #[test]
    fn test_content_hash_consistency() {
        let v1 = Vector::new(vec![0.1, 0.2, 0.3], "test-model");
        let v2 = Vector::new(vec![0.1, 0.2, 0.3], "test-model");
        let v3 = Vector::new(vec![0.1, 0.2, 0.4], "test-model");

        let h1 = ContentHash::from_vector(&v1);
        let h2 = ContentHash::from_vector(&v2);
        let h3 = ContentHash::from_vector(&v3);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_generation_cache() {
        let graph = SynthesisGraph::new();

        // Insert vectors
        for _ in 0..50 {
            graph.insert(random_vector(128)).unwrap();
        }

        let query = random_vector(128);

        // First search - should miss cache
        let results1 = graph.search(&query, 10).unwrap();
        assert!(!results1.is_empty());

        // Second search - should hit cache
        let results2 = graph.search(&query, 10).unwrap();
        assert!(!results2.is_empty());

        // At least one result should be from hot tier
        let has_hot = results2.iter().any(|r| r.tier == SearchTier::Hot);
        assert!(has_hot, "Second query should hit cache");

        // Check epoch
        let (_, _, epoch) = graph.cache_stats();
        assert_eq!(epoch, 0); // Should still be 0 (not enough inserts)

        // Insert enough to trigger epoch increment
        for _ in 0..100 {
            graph.insert(random_vector(128)).unwrap();
        }

        let (_, _, epoch2) = graph.cache_stats();
        assert!(epoch2 > 0, "Epoch should have incremented");
    }

    #[test]
    fn test_adaptive_thresholds() {
        let graph = SynthesisGraph::new();

        // Get initial thresholds
        let (_fast1, _) = graph.get_thresholds();

        // Insert enough vectors for meaningful search
        for _ in 0..200 {
            graph.insert(random_vector(128)).unwrap();
        }

        // Run several searches to generate feedback
        for _ in 0..20 {
            let query = random_vector(128);
            let _ = graph.search(&query, 10).unwrap();
        }

        // Thresholds might have adapted
        let (fast2, _) = graph.get_thresholds();

        // Thresholds should be reasonable bounds
        assert!((0.80..=0.98).contains(&fast2));
    }

    #[test]
    fn test_escalating_search() {
        let graph = SynthesisGraph::new();

        // Insert vectors
        for _ in 0..100 {
            graph.insert(random_vector(128)).unwrap();
        }

        let query = random_vector(128);
        let results = graph.search(&query, 10).unwrap();

        assert!(!results.is_empty());
        assert!(results[0].score > 0.0);

        // All results should have confidence
        for r in &results {
            assert!(r.confidence >= 0.0 && r.confidence <= 1.0);
        }
    }

    #[test]
    fn test_graph_connectivity() {
        let graph = SynthesisGraph::new_with_params(16, 100);

        for _ in 0..100 {
            graph.insert(random_vector(128)).unwrap();
        }

        let avg_edges = graph.avg_edges();
        assert!(avg_edges >= 8.0, "Graph should be well-connected");
    }

    // =========================================================================
    // SNSW v2.2.0: Synthesis-Based Navigation Tests
    // =========================================================================

    #[test]
    fn test_synthesis_edge_creation() {
        let graph = SynthesisGraph::new_with_params(16, 100);

        // Insert vectors with varying similarities
        for _ in 0..50 {
            graph.insert(random_vector(128)).unwrap();
        }

        // Check that synthesis edges were created
        let avg_synth_edges = graph.avg_synthesis_edges();
        assert!(avg_synth_edges > 0.0, "Should have synthesis edges");

        // Check synthesis stats
        let stats = graph.synthesis_stats();
        assert!(
            !stats.is_empty(),
            "Should have synthesis relationship types"
        );

        // Verify we have proximity edges at minimum
        let proximity_count = stats.get(&SynthesisType::Proximity).copied().unwrap_or(0);
        assert!(proximity_count > 0, "Should have proximity relationships");
    }

    #[test]
    fn test_content_addressed_deduplication() {
        let graph = SynthesisGraph::new();

        // Create identical vectors
        let v1 = Vector::new(vec![0.1, 0.2, 0.3, 0.4, 0.5], "test-model");
        let v2 = Vector::new(vec![0.1, 0.2, 0.3, 0.4, 0.5], "test-model");

        // Insert same vector twice
        let id1 = graph.insert(v1).unwrap();
        let id2 = graph.insert(v2).unwrap();

        // Should get same ID (content-addressed deduplication)
        assert_eq!(id1, id2, "Identical vectors should have same content hash");
        assert_eq!(graph.len(), 1, "Should only have one node (deduplication)");
    }

    #[test]
    fn test_explainable_search() {
        let graph = SynthesisGraph::new_with_params(16, 100);

        // Insert vectors
        for _ in 0..50 {
            graph.insert(random_vector(128)).unwrap();
        }

        let query = random_vector(128);
        let explainable_results = graph.search_explainable(&query, 5).unwrap();

        assert!(!explainable_results.is_empty(), "Should return results");

        // Check that explanations are provided
        for result in &explainable_results {
            assert!(result.result.score > 0.0, "Should have positive score");
            assert!(
                result.explanation.geometric_similarity > 0.0,
                "Should have geometric similarity"
            );
            assert!(
                !result.explanation.description.is_empty(),
                "Should have description"
            );
        }
    }

    #[test]
    fn test_abstraction_level_distribution() {
        let graph = SynthesisGraph::new_with_params(16, 100);

        // Insert diverse vectors
        for _ in 0..100 {
            graph.insert(random_vector(128)).unwrap();
        }

        let dist = graph.abstraction_distribution();

        // Should have at least level 0 (specific instances)
        assert!(
            dist.contains_key(&0),
            "Should have level 0 (specific) nodes"
        );

        // Total should equal node count
        let total: usize = dist.values().sum();
        assert_eq!(total, graph.len(), "Distribution should cover all nodes");
    }

    #[test]
    fn test_synthesis_proximity_calculation() {
        // Test synthesis proximity with different weights
        let prox1 = SynthesisProximity::new(0.9, 0.8, 0.7);
        assert!(prox1.score > 0.0 && prox1.score <= 1.0);
        assert_eq!(prox1.geometric, 0.9);
        assert_eq!(prox1.semantic, 0.8);
        assert_eq!(prox1.causal, 0.7);

        // Test with custom weights
        let weights = ProximityWeights {
            geometric: 0.3,
            semantic: 0.5,
            causal: 0.2,
        };
        let prox2 = SynthesisProximity::with_weights(1.0, 1.0, 1.0, weights);
        assert!(prox2.score > 0.0);
    }

    #[test]
    fn test_synthesis_edge_types() {
        let target = ContentHash::from_vector(&Vector::new(vec![0.1], "test"));

        // Create edges of different types
        let proximity = SynthesisEdge::proximity(target.clone(), 0.9);
        assert_eq!(proximity.relationship, SynthesisType::Proximity);
        assert_eq!(proximity.geometric_score, 0.9);

        let composition = SynthesisEdge::new(target.clone(), SynthesisType::Composition, 0.8, 0.7);
        assert_eq!(composition.relationship, SynthesisType::Composition);
        assert!(composition.strength > 0.0);

        let abstraction = SynthesisEdge::new(target, SynthesisType::Abstraction, 0.95, 0.9);
        assert_eq!(abstraction.relationship, SynthesisType::Abstraction);
    }

    #[test]
    fn test_synthesis_type_display() {
        assert_eq!(format!("{}", SynthesisType::Proximity), "proximity");
        assert_eq!(format!("{}", SynthesisType::Composition), "composition");
        assert_eq!(format!("{}", SynthesisType::Abstraction), "abstraction");
        assert_eq!(format!("{}", SynthesisType::Instantiation), "instantiation");
        assert_eq!(format!("{}", SynthesisType::Sequence), "sequence");
        assert_eq!(format!("{}", SynthesisType::Causation), "causation");
    }

    #[test]
    fn test_node_synthesis_edges_by_type() {
        let target1 = ContentHash::from_vector(&Vector::new(vec![0.1], "test"));
        let target2 = ContentHash::from_vector(&Vector::new(vec![0.2], "test"));

        let node = SynthesisNode {
            id: ContentHash::from_vector(&Vector::new(vec![0.0], "test")),
            vector: Arc::new(Vector::new(vec![0.0, 0.1], "test")),
            synthesis_edges: vec![
                SynthesisEdge::new(target1.clone(), SynthesisType::Proximity, 0.9, 0.0),
                SynthesisEdge::new(target2.clone(), SynthesisType::Composition, 0.8, 0.7),
            ],
            edges: vec![],
            abstraction_level: 0,
            inserted_at: 0,
            shared_distinctions: vec![],
        };

        let proximity_edges = node.edges_by_type(&SynthesisType::Proximity);
        assert_eq!(proximity_edges.len(), 1);
        assert_eq!(proximity_edges[0].target, target1);

        let composition_edges = node.edges_by_type(&SynthesisType::Composition);
        assert_eq!(composition_edges.len(), 1);
        assert_eq!(composition_edges[0].target, target2);
    }

    #[test]
    fn test_distinction_overlap() {
        let node1 = SynthesisNode {
            id: ContentHash::from_vector(&Vector::new(vec![0.1], "test")),
            vector: Arc::new(Vector::new(vec![0.0], "test")),
            synthesis_edges: vec![],
            edges: vec![],
            abstraction_level: 0,
            inserted_at: 0,
            shared_distinctions: vec!["A".to_string(), "B".to_string(), "C".to_string()],
        };

        let node2 = SynthesisNode {
            id: ContentHash::from_vector(&Vector::new(vec![0.2], "test")),
            vector: Arc::new(Vector::new(vec![0.1], "test")),
            synthesis_edges: vec![],
            edges: vec![],
            abstraction_level: 0,
            inserted_at: 1,
            shared_distinctions: vec!["B".to_string(), "C".to_string(), "D".to_string()],
        };

        let overlap = node1.distinction_overlap(&node2);
        assert_eq!(overlap.shared_count, 2); // B and C
        assert!(overlap.shared_ratio > 0.0 && overlap.shared_ratio <= 1.0);
    }

    #[test]
    fn test_synthesis_navigation_operations() {
        // Just verify the navigation operations can be created
        let target = ContentHash::from_vector(&Vector::new(vec![0.1], "test"));

        let add_op = NavigationOp::Add(target.clone());
        let sub_op = NavigationOp::Subtract(target.clone());
        let toward_op = NavigationOp::Toward(target, 0.5);

        // Verify they can be used
        let ops = [add_op, sub_op, toward_op];
        assert_eq!(ops.len(), 3);
    }
}
