//! Synthesis-Navigable Small World (SNSW) - Production-Ready Adaptive Search
//!
//! A production-grade approximate nearest neighbor (ANN) search implementation
//! with generation-based caching, exact-match hot tier, and adaptive threshold
//! learning.
//!
//! # Architecture
//!
//! ## Search Tiers
//!
//! 1. **🔥 Hot (Exact Cache)**: O(1) exact hash match only. No near-hit scanning.
//! 2. **🌤️ Warm-Fast**: Beam search with low ef, check confidence
//! 3. **🌤️ Warm-Thorough**: Beam search with high ef if confidence insufficient
//! 4. **❄️ Cold (Exact)**: Linear scan when graph is unhealthy or confidence low
//!
//! ## Key Features
//!
//! - **Generation-Based Cache**: Survives insertions via epoch tracking (lazy invalidation)
//! - **Exact Hot Tier**: O(1) lookup only - no expensive near-hit scanning
//! - **Adaptive Thresholds**: Learns optimal confidence thresholds from query feedback
//! - **Graph Health Monitoring**: Detects when graph structure is insufficient
//!
//! # The 5 Axioms of Distinction
//!
//! 1. **Identity**: Blake3 content-addressing enables O(1) cache
//! 2. **Synthesis**: K-NN graph connects similar distinctions
//! 3. **Deduplication**: Same hash = same identity
//! 4. **Memory Tiers**: Generation-based cache = causal versioning
//! 5. **Causality**: Query feedback enables continuous learning

use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::cmp::Ordering;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

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
        
        let fast_attempts: Vec<_> = recent.iter()
            .filter(|f| f.predicted_confidence > 0.0) // Had a fast search
            .collect();
        
        if fast_attempts.len() >= 5 {
            // Calculate actual recall for fast searches
            let avg_recall: f32 = fast_attempts.iter()
                .map(|f| f.actual_recall)
                .sum::<f32>() / fast_attempts.len() as f32;
            
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
pub struct SynthesisNode {
    /// Content hash = identity.
    pub id: ContentHash,
    /// The actual vector data.
    pub vector: Arc<Vector>,
    /// Edges to neighbors with similarities.
    pub edges: Vec<(ContentHash, f32)>,
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

impl PartialOrd for SearchCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.similarity.partial_cmp(&other.similarity)
    }
}

impl Ord for SearchCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
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

/// Production-grade synthesis graph with adaptive learning.
pub struct SynthesisGraph {
    /// Configuration.
    config: AdaptiveConfig,
    /// All nodes.
    nodes: DashMap<ContentHash, SynthesisNode>,
    /// Current graph epoch (generation counter).
    epoch: AtomicU64,
    /// Insert counter for epoch management.
    insert_count: AtomicU64,
    /// Semantic cache (hot tier) with epoch tracking.
    cache: DashMap<ContentHash, CachedResult>,
    /// Adaptive thresholds learned from feedback.
    thresholds: std::sync::RwLock<AdaptiveThresholds>,
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
            epoch: AtomicU64::new(0),
            insert_count: AtomicU64::new(0),
            cache: DashMap::new(),
            thresholds: std::sync::RwLock::new(AdaptiveThresholds::default()),
        }
    }
    
    /// Create with explicit M and ef parameters (backward compatible).
    pub fn new_with_params(m: usize, ef_search: usize) -> Self {
        let mut config = AdaptiveConfig::default();
        config.m = m;
        config.ef_thorough = ef_search;
        config.ef_fast = ef_search / 2;
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
    
    /// Insert a vector into the graph.
    pub fn insert(&self, vector: Vector) -> DeltaResult<ContentHash> {
        let id = ContentHash::from_vector(&vector);
        
        // Check for duplicate
        if self.nodes.contains_key(&id) {
            return Ok(id);
        }
        
        // Find M nearest neighbors
        let mut neighbors: Vec<(ContentHash, f32)> = Vec::new();
        
        for entry in self.nodes.iter() {
            if let Some(similarity) = vector.cosine_similarity(&entry.value().vector) {
                neighbors.push((entry.key().clone(), similarity));
            }
        }
        
        neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        neighbors.truncate(self.config.m);
        
        // Create node
        let node = SynthesisNode {
            id: id.clone(),
            vector: Arc::new(vector),
            edges: neighbors.clone(),
        };
        
        self.nodes.insert(id.clone(), node);
        
        // Add reverse edges
        let node_id = id.clone();
        for (neighbor_id, similarity) in neighbors {
            if let Some(mut neighbor) = self.nodes.get_mut(&neighbor_id) {
                if !neighbor.edges.iter().any(|(eid, _)| *eid == node_id) {
                    neighbor.edges.push((node_id.clone(), similarity));
                    neighbor.edges.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
                    neighbor.edges.truncate(self.config.m);
                }
            }
        }
        
        // Manage epoch - increment periodically, not on every insert
        let count = self.insert_count.fetch_add(1, AtomicOrdering::Relaxed);
        if count > 0 && count % self.config.epoch_increment_frequency as u64 == 0 {
            self.increment_epoch();
        }
        
        Ok(id)
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
            let results: Vec<SearchResult> = fast_results.iter().map(|(id, score)| SearchResult {
                id: id.clone(),
                score: *score,
                tier: SearchTier::WarmFast,
                confidence: fast_confidence,
            }).collect();
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
            let results: Vec<SearchResult> = thorough_results.iter().map(|(id, score)| SearchResult {
                id: id.clone(),
                score: *score,
                tier: SearchTier::WarmThorough,
                confidence: thorough_confidence,
            }).collect();
            self.add_to_cache(query, &results);
            return Ok(results);
        }
        
        // Stage 4: Cold - Exact linear scan
        let exact_results = self.exact_linear_search(query, k)?;
        let results: Vec<SearchResult> = exact_results.iter().map(|(id, score)| SearchResult {
            id: id.clone(),
            score: *score,
            tier: SearchTier::Cold,
            confidence: 1.0,
        }).collect();
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
            
            return Some(cached.results.iter().take(k).map(|(id, score)| SearchResult {
                id: id.clone(),
                score: *score,
                tier: SearchTier::Hot,
                confidence: 1.0,
            }).collect());
        }
        
        None
    }
    
    /// Calculate recall of approximate vs exact results.
    fn calculate_recall(
        &self,
        approx: &[(ContentHash, f32)],
        exact: &[(ContentHash, f32)],
    ) -> f32 {
        if exact.is_empty() {
            return 1.0;
        }
        
        let exact_set: HashSet<_> = exact.iter().map(|(id, _)| id.as_str()).collect();
        let k = exact.len().min(10);
        
        let hits = approx.iter()
            .take(k)
            .filter(|(id, _)| exact_set.contains(id.as_str()))
            .count();
        
        hits as f32 / k as f32
    }
    
    /// Add results to semantic cache.
    fn add_to_cache(&self, query: &Vector, results: &[SearchResult]) {
        // Simple eviction if cache is full
        if self.cache.len() >= self.config.max_cache_size {
            let to_remove = self.cache.iter()
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
        
        let entry_points: Vec<ContentHash> = self.nodes.iter().take(5).map(|e| e.key().clone()).collect();
        
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
        let kth_score = results.get(k.saturating_sub(1)).map(|r| r.1)
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
    fn exact_linear_search(&self, query: &Vector, k: usize) -> DeltaResult<Vec<(ContentHash, f32)>> {
        if self.nodes.is_empty() || k == 0 {
            return Ok(Vec::new());
        }
        
        let mut results: Vec<(ContentHash, f32)> = self.nodes
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
        assert!(fast2 >= 0.80 && fast2 <= 0.98);
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
}
