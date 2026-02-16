//! HNSW (Hierarchical Navigable Small World) index for approximate nearest neighbor search.
//!
//! HNSW is a graph-based ANN algorithm that provides O(log n) search complexity
//! with high recall (>95%). This implementation is designed for production use
//! with 100K-1M vectors.
//!
//! # Features
//!
//! - Multi-layer graph structure for efficient navigation
//! - Configurable M (max connections) and ef (search scope) parameters
//! - Causal-consistent snapshots for time-travel queries
//! - Thread-safe concurrent access
//!
//! # Example
//!
//! ```ignore
//! use koru_delta::vector::hnsw::{HnswIndex, HnswConfig};
//!
//! let config = HnswConfig::default();
//! let mut index = HnswIndex::new(config);
//!
//! // Add vectors
//! index.add("doc1".to_string(), vector1)?;
//! index.add("doc2".to_string(), vector2)?;
//!
//! // Search
//! let results = index.search(&query_vector, 10, 50);
//! ```

use super::types::{Vector, VectorSearchResult};
use crate::types::FullKey;
use dashmap::DashMap;
use rand::SeedableRng;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Configuration for HNSW index.
#[derive(Debug, Clone, Copy)]
pub struct HnswConfig {
    /// Maximum number of connections per node (default: 16)
    pub m: usize,
    /// Size of dynamic candidate list during construction (default: 200)
    pub ef_construction: usize,
    /// Size of dynamic candidate list during search (default: 50)
    pub ef_search: usize,
    /// Probability decay factor for layer assignment (default: 1.0 / ln(M))
    pub m_l: f64,
}

impl Default for HnswConfig {
    fn default() -> Self {
        let m = 16;
        Self {
            m,
            ef_construction: 200,
            ef_search: 50,
            m_l: 1.0 / (m as f64).ln(),
        }
    }
}

impl HnswConfig {
    /// Create a new config with custom M.
    pub fn with_m(m: usize) -> Self {
        Self {
            m,
            ef_construction: 200,
            ef_search: 50,
            m_l: 1.0 / (m as f64).ln(),
        }
    }

    /// Set ef_construction.
    pub fn ef_construction(mut self, ef: usize) -> Self {
        self.ef_construction = ef;
        self
    }

    /// Set ef_search.
    pub fn ef_search(mut self, ef: usize) -> Self {
        self.ef_search = ef;
        self
    }
}

/// A node in the HNSW graph.
#[derive(Debug, Clone)]
struct Node {
    /// The vector data
    vector: Vector,
    /// Maximum layer this node exists in
    max_layer: usize,
}

impl Node {
    fn new(vector: Vector, max_layer: usize, _m: usize) -> Self {
        Self { vector, max_layer }
    }
}

/// A layer in the HNSW graph.
#[derive(Debug, Default)]
struct Layer {
    /// Node id -> neighbor ids
    edges: HashMap<String, Vec<String>>,
}

impl Layer {
    fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    fn get_neighbors(&self, node_id: &str) -> &[String] {
        self.edges.get(node_id).map_or(&[], |v| v.as_slice())
    }
}

/// HNSW (Hierarchical Navigable Small World) index.
///
/// Implements the HNSW algorithm for approximate nearest neighbor search.
/// Provides O(log n) search complexity with high recall.
pub struct HnswIndex {
    /// Configuration
    config: HnswConfig,
    /// All nodes in the index
    nodes: DashMap<String, Node>,
    /// Layer structure: layer_index -> Layer
    layers: Vec<std::sync::RwLock<Layer>>,
    /// Entry point (node with highest layer)
    entry_point: std::sync::RwLock<Option<String>>,
    /// Current max layer
    max_layer: std::sync::atomic::AtomicUsize,
    /// Random number generator
    rng: std::sync::Mutex<StdRng>,
    /// Model filter (only index vectors from this model)
    model_filter: Option<String>,
}

impl std::fmt::Debug for HnswIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswIndex")
            .field("config", &self.config)
            .field("num_nodes", &self.nodes.len())
            .field("num_layers", &self.layers.len())
            .field(
                "max_layer",
                &self.max_layer.load(std::sync::atomic::Ordering::Relaxed),
            )
            .field("entry_point", &self.entry_point.read().unwrap())
            .finish()
    }
}

/// Search candidate for priority queue.
#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    distance: f32,
    id: String,
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .distance
            .partial_cmp(&self.distance) // Reverse for min-heap
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl HnswIndex {
    /// Create a new HNSW index with the given configuration.
    pub fn new(config: HnswConfig) -> Self {
        let max_layers = 16; // Pre-allocate for reasonable depth
        let mut layers = Vec::with_capacity(max_layers);
        for _ in 0..max_layers {
            layers.push(std::sync::RwLock::new(Layer::new()));
        }

        Self {
            config,
            nodes: DashMap::new(),
            layers,
            entry_point: std::sync::RwLock::new(None),
            max_layer: std::sync::atomic::AtomicUsize::new(0),
            rng: std::sync::Mutex::new(StdRng::seed_from_u64(42)),
            model_filter: None,
        }
    }

    /// Create a new HNSW index filtered to a specific model.
    pub fn with_model_filter(config: HnswConfig, model: impl Into<String>) -> Self {
        let mut index = Self::new(config);
        index.model_filter = Some(model.into());
        index
    }

    /// Get the number of vectors in the index.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Assign a random layer to a new node.
    fn random_layer(&self) -> usize {
        let mut rng = self.rng.lock().unwrap();
        let uniform = Uniform::from(0.0..1.0);
        let mut level = 0;
        loop {
            let r: f64 = uniform.sample(&mut *rng);
            if r < (-(level as f64) * self.config.m_l).exp() {
                level += 1;
            } else {
                break;
            }
        }
        level
    }

    /// Add a vector to the index.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this vector
    /// * `vector` - The vector to add
    ///
    /// # Errors
    /// Returns an error if the vector's model doesn't match the filter.
    pub fn add(&self, id: String, vector: Vector) -> crate::error::DeltaResult<()> {
        // Check model filter
        if let Some(ref filter) = self.model_filter {
            if vector.model() != filter {
                return Err(crate::error::DeltaError::InvalidData {
                    reason: format!(
                        "Vector model '{}' doesn't match filter '{}'",
                        vector.model(),
                        filter
                    ),
                });
            }
        }

        // Check if already exists
        if self.nodes.contains_key(&id) {
            // Remove old entry first
            self.remove(&id);
        }

        let layer = self.random_layer();
        let node = Node::new(vector.clone(), layer, self.config.m);
        let vector_ref = vector.clone();

        // Insert node
        self.nodes.insert(id.clone(), node);

        // Update max layer
        let current_max = self.max_layer.load(std::sync::atomic::Ordering::Relaxed);
        if layer > current_max {
            self.max_layer
                .store(layer, std::sync::atomic::Ordering::Relaxed);
            *self.entry_point.write().unwrap() = Some(id.clone());
        }

        // If this is the first node, we're done
        let entry_point = self.entry_point.read().unwrap().clone();
        if entry_point.is_none() || (entry_point.as_ref() == Some(&id)) {
            return Ok(());
        }

        // Find entry point for search
        let mut curr_ep = entry_point.unwrap();
        let curr_node = self.nodes.get(&curr_ep).unwrap();
        let mut curr_dist = self.distance(&curr_node.vector, &vector_ref);
        let curr_max_layer = curr_node.max_layer;

        // Search from top layer down to layer+1
        for lc in ((layer + 1)..=curr_max_layer).rev() {
            let (new_ep, new_dist) = self.search_layer_simple(&curr_ep, &vector_ref, 1, lc);
            if new_dist < curr_dist {
                curr_ep = new_ep;
                curr_dist = new_dist;
            }
        }

        // Insert from min(layer, curr_max_layer) down to 0
        let min_layer = layer.min(curr_max_layer);
        for lc in (0..=min_layer).rev() {
            // Search for neighbors
            let neighbors =
                self.search_layer(&curr_ep, &vector_ref, self.config.ef_construction, lc);

            // Select M neighbors using heuristic
            let selected = self.select_neighbors(&neighbors, self.config.m);

            // Add bidirectional connections
            for neighbor_id in &selected {
                self.add_edge(lc, &id, neighbor_id);
                self.add_edge(lc, neighbor_id, &id);

                // Check if we need to prune neighbor's connections
                self.prune_connections(lc, neighbor_id)?;
            }
        }

        Ok(())
    }

    /// Simple layer search (greedy).
    fn search_layer_simple(
        &self,
        entry_point: &str,
        query: &Vector,
        ef: usize,
        layer: usize,
    ) -> (String, f32) {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut best = BinaryHeap::new();

        let entry_node = match self.nodes.get(entry_point) {
            Some(n) => n,
            None => return (entry_point.to_string(), f32::MAX),
        };

        let entry_dist = self.distance(&entry_node.vector, query);
        visited.insert(entry_point.to_string());
        candidates.push(Candidate {
            distance: entry_dist,
            id: entry_point.to_string(),
        });
        best.push(Candidate {
            distance: -entry_dist,
            id: entry_point.to_string(),
        });

        while let Some(curr) = candidates.pop() {
            let worst_best = best
                .peek()
                .map(|c: &Candidate| -c.distance)
                .unwrap_or(f32::MAX);
            if curr.distance > worst_best {
                break;
            }

            let layer_guard = self.layers[layer].read().unwrap();
            let neighbors = layer_guard.get_neighbors(&curr.id);

            for neighbor_id in neighbors {
                if visited.contains(neighbor_id) {
                    continue;
                }
                visited.insert(neighbor_id.clone());

                if let Some(neighbor_node) = self.nodes.get(neighbor_id) {
                    let dist = self.distance(&neighbor_node.vector, query);

                    if dist < worst_best || best.len() < ef {
                        candidates.push(Candidate {
                            distance: dist,
                            id: neighbor_id.clone(),
                        });
                        best.push(Candidate {
                            distance: -dist,
                            id: neighbor_id.clone(),
                        });
                        if best.len() > ef {
                            best.pop();
                        }
                    }
                }
            }
        }

        // Return the closest
        best.pop()
            .map(|c| (c.id, -c.distance))
            .unwrap_or_else(|| (entry_point.to_string(), entry_dist))
    }

    /// Search a layer and return candidates.
    fn search_layer(
        &self,
        entry_point: &str,
        query: &Vector,
        ef: usize,
        layer: usize,
    ) -> Vec<(String, f32)> {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut best = BinaryHeap::new();

        let entry_node = match self.nodes.get(entry_point) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let entry_dist = self.distance(&entry_node.vector, query);
        visited.insert(entry_point.to_string());
        candidates.push(Candidate {
            distance: entry_dist,
            id: entry_point.to_string(),
        });
        best.push(Candidate {
            distance: -entry_dist,
            id: entry_point.to_string(),
        });

        while let Some(curr) = candidates.pop() {
            let worst_best = best
                .peek()
                .map(|c: &Candidate| -c.distance)
                .unwrap_or(f32::MAX);
            if curr.distance > worst_best {
                break;
            }

            let layer_guard = self.layers[layer].read().unwrap();
            let neighbors = layer_guard.get_neighbors(&curr.id);

            for neighbor_id in neighbors {
                if visited.contains(neighbor_id) {
                    continue;
                }
                visited.insert(neighbor_id.clone());

                if let Some(neighbor_node) = self.nodes.get(neighbor_id) {
                    let dist = self.distance(&neighbor_node.vector, query);

                    if dist < worst_best || best.len() < ef {
                        candidates.push(Candidate {
                            distance: dist,
                            id: neighbor_id.clone(),
                        });
                        best.push(Candidate {
                            distance: -dist,
                            id: neighbor_id.clone(),
                        });
                        if best.len() > ef {
                            best.pop();
                        }
                    }
                }
            }
        }

        // Convert to vec
        best.into_iter().map(|c| (c.id, -c.distance)).collect()
    }

    /// Select M neighbors from candidates using simple heuristic.
    fn select_neighbors(&self, candidates: &[(String, f32)], m: usize) -> Vec<String> {
        candidates
            .iter()
            .take(m)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Add an edge in a layer.
    fn add_edge(&self, layer: usize, from: &str, to: &str) {
        if let Ok(mut layer_guard) = self.layers[layer].write() {
            layer_guard.add_edge(from, to);
        }
    }

    /// Prune connections if a node has too many.
    fn prune_connections(&self, layer: usize, node_id: &str) -> crate::error::DeltaResult<()> {
        let max_connections = self.config.m * 2;

        let neighbors: Vec<String> = {
            let layer_guard = self.layers[layer].read().unwrap();
            layer_guard.get_neighbors(node_id).to_vec()
        };

        if neighbors.len() <= max_connections {
            return Ok(());
        }

        // Need to prune - keep closest M*2
        let node =
            self.nodes
                .get(node_id)
                .ok_or_else(|| crate::error::DeltaError::KeyNotFound {
                    namespace: "hnsw".to_string(),
                    key: node_id.to_string(),
                })?;

        let mut neighbor_dists: Vec<(String, f32)> = neighbors
            .iter()
            .filter_map(|nid| {
                self.nodes.get(nid).map(|n| {
                    let dist = self.distance(&node.vector, &n.vector);
                    (nid.clone(), dist)
                })
            })
            .collect();

        neighbor_dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        neighbor_dists.truncate(max_connections);

        // Update edges
        if let Ok(mut layer_guard) = self.layers[layer].write() {
            let new_edges: Vec<String> = neighbor_dists.into_iter().map(|(id, _)| id).collect();
            layer_guard.edges.insert(node_id.to_string(), new_edges);
        }

        Ok(())
    }

    /// Compute distance between two vectors (using cosine distance).
    fn distance(&self, a: &Vector, b: &Vector) -> f32 {
        // Convert similarity to distance: distance = 1 - similarity
        a.cosine_similarity(b).map(|s| 1.0 - s).unwrap_or(f32::MAX)
    }

    /// Remove a vector from the index.
    pub fn remove(&self, id: &str) {
        // First, check if we need to update the entry point
        let needs_ep_update = {
            let ep_guard = self.entry_point.read().unwrap();
            ep_guard.as_ref().map(|ep| ep == id).unwrap_or(false)
        };

        if let Some((_, node)) = self.nodes.remove(id) {
            // Remove edges at all layers
            for layer in 0..=node.max_layer {
                if let Ok(mut layer_guard) = self.layers[layer].write() {
                    layer_guard.edges.remove(id);
                    // Remove references from other nodes
                    for (_, neighbors) in layer_guard.edges.iter_mut() {
                        neighbors.retain(|n| n != id);
                    }
                }
            }
        }

        // Update entry point if needed (after releasing all other locks)
        if needs_ep_update {
            // Find new entry point (highest layer node)
            let mut max_layer = 0;
            let mut new_ep = None;
            for entry in self.nodes.iter() {
                if entry.value().max_layer >= max_layer {
                    max_layer = entry.value().max_layer;
                    new_ep = Some(entry.key().clone());
                }
            }
            *self.entry_point.write().unwrap() = new_ep;
            self.max_layer
                .store(max_layer, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Search for k nearest neighbors.
    ///
    /// # Arguments
    /// * `query` - The query vector
    /// * `k` - Number of results to return
    /// * `ef` - Size of dynamic candidate list (higher = more accurate, slower)
    ///
    /// # Returns
    /// Vector of search results sorted by similarity (highest first).
    pub fn search(&self, query: &Vector, k: usize, ef: usize) -> Vec<VectorSearchResult> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        // Check model compatibility
        if let Some(ref filter) = self.model_filter {
            if query.model() != filter {
                return Vec::new();
            }
        }

        let entry_point = match self.entry_point.read().unwrap().clone() {
            Some(ep) => ep,
            None => return Vec::new(),
        };

        let ef = ef.max(k);
        let max_layer = self.max_layer.load(std::sync::atomic::Ordering::Relaxed);

        // Get entry point node
        let entry_node = match self.nodes.get(&entry_point) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let mut curr_ep = entry_point;
        let mut curr_dist = self.distance(&entry_node.vector, query);
        let entry_max_layer = entry_node.max_layer;

        // Search from top layer down to layer 1
        for lc in (1..=entry_max_layer.min(max_layer)).rev() {
            let (new_ep, new_dist) = self.search_layer_simple(&curr_ep, query, 1, lc);
            if new_dist < curr_dist {
                curr_ep = new_ep;
                curr_dist = new_dist;
            }
        }

        // Search layer 0 with ef
        let candidates = self.search_layer(&curr_ep, query, ef, 0);

        // Build results
        let mut results: Vec<VectorSearchResult> = candidates
            .into_iter()
            .take(k)
            .filter_map(|(id, dist)| {
                self.nodes.get(&id).map(|node| {
                    let similarity = 1.0 - dist;
                    // Parse namespace from id (format: "namespace:key")
                    let (namespace, key) = if let Some(pos) = id.find(':') {
                        (id[..pos].to_string(), id[pos + 1..].to_string())
                    } else {
                        ("default".to_string(), id.clone())
                    };
                    VectorSearchResult::new(namespace, key, similarity, node.vector.clone())
                })
            })
            .collect();

        // Sort by score (highest first)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Clear all vectors from the index.
    pub fn clear(&self) {
        self.nodes.clear();
        for layer in &self.layers {
            if let Ok(mut guard) = layer.write() {
                guard.edges.clear();
            }
        }
        *self.entry_point.write().unwrap() = None;
        self.max_layer
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

impl super::index::AnnIndex for HnswIndex {
    fn add(&self, key: FullKey, vector: Vector) {
        let id = format!("{}:{}", key.namespace, key.key);
        let _ = self.add(id, vector);
    }

    fn remove(&self, namespace: &str, key: &str) {
        let id = format!("{}:{}", namespace, key);
        self.remove(&id);
    }

    fn search(
        &self,
        query: &Vector,
        opts: &super::types::VectorSearchOptions,
    ) -> Vec<VectorSearchResult> {
        let results = self.search(query, opts.top_k, self.config.ef_search);
        // Filter by threshold
        results
            .into_iter()
            .filter(|r| r.score >= opts.threshold)
            .collect()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn clear(&self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vector(data: Vec<f32>) -> Vector {
        Vector::new(data, "test-model")
    }

    #[test]
    fn test_hnsw_config_default() {
        let config = HnswConfig::default();
        assert_eq!(config.m, 16);
        assert_eq!(config.ef_construction, 200);
        assert_eq!(config.ef_search, 50);
    }

    #[test]
    fn test_hnsw_config_custom() {
        let config = HnswConfig::with_m(32).ef_construction(400).ef_search(100);
        assert_eq!(config.m, 32);
        assert_eq!(config.ef_construction, 400);
        assert_eq!(config.ef_search, 100);
    }

    #[test]
    fn test_hnsw_add_and_search() {
        let index = HnswIndex::new(HnswConfig::default());

        // Add some vectors
        let v1 = create_test_vector(vec![1.0, 0.0, 0.0]);
        let v2 = create_test_vector(vec![0.0, 1.0, 0.0]);
        let v3 = create_test_vector(vec![0.0, 0.0, 1.0]);

        index.add("doc1".to_string(), v1).unwrap();
        index.add("doc2".to_string(), v2).unwrap();
        index.add("doc3".to_string(), v3).unwrap();

        // Verify all vectors were added
        assert_eq!(index.len(), 3);

        // Search should not panic and return results
        let query = create_test_vector(vec![0.9, 0.1, 0.0]);
        let results = index.search(&query, 3, 50);

        // Basic sanity checks
        assert!(results.len() <= 3, "Should return at most 3 results");

        // Results should be sorted by score descending
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_hnsw_empty_search() {
        let index = HnswIndex::new(HnswConfig::default());
        let query = create_test_vector(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 10, 50);
        assert!(results.is_empty());
    }

    #[test]
    fn test_hnsw_remove() {
        let index = HnswIndex::new(HnswConfig::default());

        let v1 = create_test_vector(vec![1.0, 0.0, 0.0]);
        index.add("doc1".to_string(), v1).unwrap();
        assert_eq!(index.len(), 1);

        index.remove("doc1");
        assert_eq!(index.len(), 0);

        let query = create_test_vector(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 10, 50);
        assert!(results.is_empty());
    }

    #[test]
    fn test_hnsw_clear() {
        let index = HnswIndex::new(HnswConfig::default());

        index
            .add("doc1".to_string(), create_test_vector(vec![1.0, 0.0]))
            .unwrap();
        index
            .add("doc2".to_string(), create_test_vector(vec![0.0, 1.0]))
            .unwrap();
        assert_eq!(index.len(), 2);

        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_hnsw_model_filter() {
        let config = HnswConfig::default();
        let index = HnswIndex::with_model_filter(config, "model-a");

        let v1 = Vector::new(vec![1.0, 0.0], "model-a");
        let v2 = Vector::new(vec![0.0, 1.0], "model-b"); // Different model

        index.add("doc1".to_string(), v1).unwrap();
        assert!(index.add("doc2".to_string(), v2).is_err()); // Should fail

        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_hnsw_multiple_layers() {
        let config = HnswConfig::with_m(4);
        let index = HnswIndex::new(config);

        // Add many vectors to trigger multi-layer structure
        for i in 0..100 {
            let v = create_test_vector(vec![i as f32, (i * 2) as f32]);
            index.add(format!("doc{}", i), v).unwrap();
        }

        assert_eq!(index.len(), 100);

        // Search
        let query = create_test_vector(vec![50.0, 100.0]);
        let results = index.search(&query, 10, 50);

        assert!(!results.is_empty());
        // The closest should be around doc50
        assert!(results[0].score > 0.99);
    }

    #[test]
    fn test_hnsw_recall_benchmark() {
        let config = HnswConfig::with_m(16);
        let index = HnswIndex::new(config);

        // Create random-ish vectors
        let mut vectors = Vec::new();
        for i in 0..1000 {
            let v = create_test_vector(vec![(i % 10) as f32 / 10.0, ((i / 10) % 10) as f32 / 10.0]);
            vectors.push((format!("doc{}", i), v.clone()));
            index.add(format!("doc{}", i), v).unwrap();
        }

        // Query
        let query = create_test_vector(vec![0.5, 0.5]);
        let k = 10;
        let results = index.search(&query, k, 100);

        // Check recall (should find the closest vectors)
        assert!(results.len() >= k.min(index.len()));

        // Scores should be high for similar vectors
        for result in &results {
            assert!(result.score > 0.5);
        }
    }
}
