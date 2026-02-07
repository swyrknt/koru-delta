//! Synthesis-Navigable Small World (SNSW) - Escalating Adaptive Search
//!
//! An intelligent approximate nearest neighbor (ANN) search implementation that
//! uses escalating search with confidence-based tier selection.
//!
//! # Escalating Search Architecture
//!
//! Instead of hardcoded thresholds, the system escalates through search strategies
//! based on result confidence:
//!
//! 1. **🔥 Hot (Cache)**: O(1) lookup for repeated/near-identical queries
//! 2. **🌤️ Warm-Fast**: O(log n) beam search with low ef (fast but approximate)
//! 3. **🌤️ Warm-Thorough**: O(log n) beam search with high ef (better recall)
//! 4. **❄️ Cold (Exact)**: O(n) brute force when confidence is insufficient
//!
//! # Confidence Estimation
//!
//! Search quality is estimated without ground truth using score distribution:
//! - High confidence: Large gap between top results (clear winner)
//! - Low confidence: Similar scores (uncertain about true nearest neighbors)
//!
//! # The 5 Axioms of Distinction
//!
//! 1. **Identity**: Content-addressing via Blake3 enables semantic caching
//! 2. **Synthesis**: K-NN graph connects similar distinctions
//! 3. **Deduplication**: Same content = same hash = same identity
//! 4. **Memory Tiers**: Escalating search matches computational cost to query difficulty
//! 5. **Causality**: Query feedback loop enables continuous optimization

use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;
use std::hash::Hash;
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
    /// Hot: From semantic cache (instant).
    Hot,
    /// WarmFast: Quick beam search with low ef.
    WarmFast,
    /// WarmThorough: Beam search with high ef.
    WarmThorough,
    /// Cold: Exact linear scan (confidence insufficient).
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
    /// Top-k results stored as (id, score) pairs.
    results: Vec<(ContentHash, f32)>,
    /// How many times this cache entry was hit.
    hit_count: u64,
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

/// Configuration for escalating search.
#[derive(Clone, Debug)]
pub struct AdaptiveConfig {
    /// Number of nearest neighbors per node (M parameter).
    pub m: usize,
    /// Expansion factor for fast warm search (low effort).
    pub ef_fast: usize,
    /// Expansion factor for thorough warm search (high effort).
    pub ef_thorough: usize,
    /// Confidence threshold to accept fast search results.
    pub confidence_threshold_fast: f32,
    /// Confidence threshold to accept thorough search results.
    pub confidence_threshold_thorough: f32,
    /// Maximum cache size.
    pub max_cache_size: usize,
    /// Similarity threshold for near-hit cache lookup.
    pub near_hit_threshold: f32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_fast: 50,
            ef_thorough: 200,
            confidence_threshold_fast: 0.90,
            confidence_threshold_thorough: 0.95,
            max_cache_size: 1000,
            near_hit_threshold: 0.98,
        }
    }
}

/// Escalating search graph with confidence-based tier selection.
pub struct SynthesisGraph {
    /// Configuration.
    config: AdaptiveConfig,
    /// All nodes.
    nodes: DashMap<ContentHash, SynthesisNode>,
    /// Semantic cache (hot tier).
    cache: DashMap<ContentHash, CachedResult>,
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
            cache: DashMap::new(),
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
        
        // Invalidate cache since graph changed
        self.invalidate_cache();
        
        Ok(id)
    }
    
    /// Escalating search - automatically selects optimal strategy based on confidence.
    ///
    /// # Escalation Stages
    ///
    /// 1. **Hot**: Check semantic cache
    /// 2. **Warm-Fast**: Beam search with ef_fast, check confidence
    /// 3. **Warm-Thorough**: If confidence low, beam search with ef_thorough
    /// 4. **Cold**: If still low confidence, exact linear scan
    pub fn search(&self, query: &Vector, k: usize) -> DeltaResult<Vec<SearchResult>> {
        // Stage 1: Hot - Check semantic cache
        if let Some(results) = self.check_cache(query, k) {
            return Ok(results);
        }
        
        // Stage 2: Warm-Fast - Quick beam search
        let fast_results = self.beam_search_with_rerank(query, k, self.config.ef_fast, SearchTier::WarmFast)?;
        let fast_confidence = self.estimate_confidence(&fast_results, k);
        
        if fast_confidence >= self.config.confidence_threshold_fast {
            let results: Vec<SearchResult> = fast_results.into_iter().map(|(id, score)| SearchResult {
                id,
                score,
                tier: SearchTier::WarmFast,
                confidence: fast_confidence,
            }).collect();
            self.add_to_cache(query, &results);
            return Ok(results);
        }
        
        // Stage 3: Warm-Thorough - Higher effort beam search
        let thorough_results = self.beam_search_with_rerank(query, k, self.config.ef_thorough, SearchTier::WarmThorough)?;
        let thorough_confidence = self.estimate_confidence(&thorough_results, k);
        
        if thorough_confidence >= self.config.confidence_threshold_thorough {
            let results: Vec<SearchResult> = thorough_results.into_iter().map(|(id, score)| SearchResult {
                id,
                score,
                tier: SearchTier::WarmThorough,
                confidence: thorough_confidence,
            }).collect();
            self.add_to_cache(query, &results);
            return Ok(results);
        }
        
        // Stage 4: Cold - Exact linear scan when confidence is insufficient
        let exact_results = self.exact_linear_search(query, k)?;
        let results: Vec<SearchResult> = exact_results.into_iter().map(|(id, score)| SearchResult {
            id,
            score,
            tier: SearchTier::Cold,
            confidence: 1.0, // Exact search has perfect confidence
        }).collect();
        self.add_to_cache(query, &results);
        Ok(results)
    }
    
    /// Check semantic cache for query.
    fn check_cache(&self, query: &Vector, k: usize) -> Option<Vec<SearchResult>> {
        let query_hash = ContentHash::from_vector(query);
        
        // Direct hit
        if let Some(mut cached) = self.cache.get_mut(&query_hash) {
            cached.hit_count += 1;
            
            return Some(cached.results.iter().take(k).map(|(id, score)| SearchResult {
                id: id.clone(),
                score: *score,
                tier: SearchTier::Hot,
                confidence: 1.0,
            }).collect());
        }
        
        // Near-hit: Check for similar queries in cache
        for entry in self.cache.iter().take(10) {
            if let Some(query_node) = self.nodes.get(entry.key()) {
                if let Some(similarity) = query.cosine_similarity(&query_node.vector) {
                    if similarity >= self.config.near_hit_threshold {
                        // Near-exact match - use cached results
                        if let Some(mut cached) = self.cache.get_mut(entry.key()) {
                            cached.hit_count += 1;
                            
                            return Some(cached.results.iter().take(k).map(|(id, score)| SearchResult {
                                id: id.clone(),
                                score: *score,
                                tier: SearchTier::Hot,
                                confidence: similarity,
                            }).collect());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Add results to semantic cache.
    fn add_to_cache(&self, query: &Vector, results: &[SearchResult]) {
        // Simple eviction if cache is full
        if self.cache.len() >= self.config.max_cache_size {
            // Find entry with lowest hit count to evict
            let to_remove = self.cache.iter()
                .min_by_key(|e| e.value().hit_count)
                .map(|e| e.key().clone());
            
            if let Some(key) = to_remove {
                self.cache.remove(&key);
            }
        }
        
        let query_hash = ContentHash::from_vector(query);
        let cached = CachedResult {
            results: results.iter().map(|r| (r.id.clone(), r.score)).collect(),
            hit_count: 0,
        };
        
        self.cache.insert(query_hash, cached);
    }
    
    /// Invalidate entire cache.
    fn invalidate_cache(&self) {
        self.cache.clear();
    }
    
    /// Beam search with exact re-ranking.
    fn beam_search_with_rerank(
        &self, 
        query: &Vector, 
        k: usize, 
        ef: usize,
        _tier: SearchTier
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
        
        // Fill remaining slots with unvisited nodes
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
    ///
    /// High confidence: Large gap between top results (clear ordering)
    /// Low confidence: Similar scores (uncertain about true nearest neighbors)
    fn estimate_confidence(&self, results: &[(ContentHash, f32)], k: usize) -> f32 {
        if results.len() < 2 {
            return 0.5; // Not enough data
        }
        
        // Look at gap between #1 and #k (or last available)
        let top_score = results[0].1;
        let kth_score = results.get(k.saturating_sub(1)).map(|r| r.1)
            .or_else(|| results.last().map(|r| r.1))
            .unwrap_or(0.0);
        
        if top_score <= 0.0 {
            return 0.0;
        }
        
        // Gap relative to top score magnitude
        let gap = top_score - kth_score;
        let relative_gap = gap / top_score;
        
        // Boost confidence based on gap
        // gap of 0.0 -> confidence 0.5
        // gap of 0.5 -> confidence 0.95
        let confidence = 0.5 + (relative_gap * 0.9);
        
        confidence.min(0.99) // Cap at 0.99 since we're estimating
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
    pub fn cache_stats(&self) -> (usize, u64) {
        let size = self.cache.len();
        let hits: u64 = self.cache.iter().map(|e| e.value().hit_count).sum();
        (size, hits)
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
        
        // Should have some tier assigned
        let tiers: std::collections::HashSet<_> = results.iter().map(|r| r.tier).collect();
        assert!(!tiers.is_empty());
    }
    
    #[test]
    fn test_semantic_cache() {
        let graph = SynthesisGraph::new();
        
        // Insert vectors
        for _ in 0..50 {
            graph.insert(random_vector(128)).unwrap();
        }
        
        let query = random_vector(128);
        
        // First search
        let results1 = graph.search(&query, 10).unwrap();
        assert!(!results1.is_empty());
        
        // Second search should hit cache
        let results2 = graph.search(&query, 10).unwrap();
        assert!(!results2.is_empty());
        
        // At least one result should be from hot tier
        let has_hot = results2.iter().any(|r| r.tier == SearchTier::Hot);
        assert!(has_hot, "Second query should hit cache");
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
    
    #[test]
    fn test_confidence_estimation() {
        let graph = SynthesisGraph::new();
        
        // Insert vectors
        for _ in 0..50 {
            graph.insert(random_vector(128)).unwrap();
        }
        
        let query = random_vector(128);
        let results = graph.search(&query, 10).unwrap();
        
        // All results should have confidence
        for r in &results {
            assert!(r.confidence >= 0.0 && r.confidence <= 1.0);
        }
    }
}
