/// World Reconciliation: Distributed Truth via Distinction Exchange.
///
/// This module implements the high-level protocol for synchronizing
/// distributed nodes. "Worlds" are distinct causal graphs that need
/// to be reconciled into a shared truth.
///
/// ## The Protocol
///
/// ```text
/// Node A                                    Node B
///   │                                         │
///   │  1. SEND: Merkle root                   │
///   │────────────────────────────────────────>│
///   │                                         │
///   │         2. SEND: Diff (what B has)     │
///   │<────────────────────────────────────────│
///   │                                         │
///   │  3. SEND: Missing distinctions          │
///   │────────────────────────────────────────>│
///   │                                         │
///   │         4. MERGE: Combine graphs       │
///   │<────────────────────────────────────────│
/// ```
///
/// ## Conflict Resolution
///
/// When nodes have divergent histories, conflicts become causal branches:
///
/// ```text
/// Before:          After reconciliation:
///   A──►B──►C         A──►B──►C
///       │                 │
///       └──►D (conflict)  ├──►D (branch 1)
///                       └──►E (branch 2)
/// ```
use crate::causal_graph::CausalGraph;
use crate::reconciliation::{MerkleTree, SyncStrategy};
use std::collections::HashSet;

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Distinctions we sent to remote.
    pub sent: Vec<String>,
    /// Distinctions we received from remote.
    pub received: Vec<String>,
    /// Conflicts detected (divergent branches).
    pub conflicts: Vec<Conflict>,
    /// Sync efficiency (0.0-1.0, higher is better).
    pub efficiency: f64,
}

impl SyncResult {
    /// Create empty sync result.
    pub fn empty() -> Self {
        Self {
            sent: vec![],
            received: vec![],
            conflicts: vec![],
            efficiency: 1.0,
        }
    }

    /// Check if sync was perfect (no data transfer needed).
    pub fn is_perfect(&self) -> bool {
        self.sent.is_empty() && self.received.is_empty() && self.conflicts.is_empty()
    }

    /// Get total distinctions transferred.
    pub fn total_transferred(&self) -> usize {
        self.sent.len() + self.received.len()
    }
}

/// A conflict between divergent branches.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The key/namespace where conflict occurred.
    pub key: String,
    /// Our version (branch 1).
    pub our_version: String,
    /// Their version (branch 2).
    pub their_version: String,
    /// Common ancestor (if known).
    pub common_ancestor: Option<String>,
}

/// High-level world reconciliation manager.
pub struct WorldReconciliation {
    /// Our causal graph.
    local_graph: CausalGraph,
    /// Sync strategy.
    #[allow(dead_code)]
    strategy: SyncStrategy,
    /// Statistics.
    stats: ReconciliationStats,
}

/// Statistics for reconciliation.
#[derive(Debug, Clone, Default)]
pub struct ReconciliationStats {
    /// Number of sync operations performed.
    pub syncs_performed: u64,
    /// Total distinctions sent.
    pub total_sent: u64,
    /// Total distinctions received.
    pub total_received: u64,
    /// Total conflicts detected.
    pub total_conflicts: u64,
    /// Perfect syncs (no transfer needed).
    pub perfect_syncs: u64,
}

impl WorldReconciliation {
    /// Create a new world reconciliation manager.
    pub fn new(local_graph: CausalGraph) -> Self {
        Self {
            local_graph,
            strategy: SyncStrategy::default(),
            stats: ReconciliationStats::default(),
        }
    }

    /// Create with specific strategy.
    pub fn with_strategy(local_graph: CausalGraph, strategy: SyncStrategy) -> Self {
        Self {
            local_graph,
            strategy,
            stats: ReconciliationStats::default(),
        }
    }

    /// Get the frontier (current leaf nodes) to share with remote.
    ///
    /// This is a compact representation of our current state.
    pub fn exchange_roots(&self) -> Vec<String> {
        self.local_graph.frontier()
    }

    /// Compare frontiers with remote to find divergence point.
    pub fn find_divergence(&self, remote_frontier: &[String]) -> Option<String> {
        let local_frontier: HashSet<_> = self.local_graph.frontier().into_iter().collect();
        let remote_frontier: HashSet<_> = remote_frontier.iter().cloned().collect();

        // Find common ancestors
        for local_leaf in &local_frontier {
            let ancestors = self.local_graph.ancestors(local_leaf);
            for remote_leaf in &remote_frontier {
                if ancestors.contains(remote_leaf) {
                    // Found common ancestor
                    return Some(remote_leaf.clone());
                }
            }
        }

        // Check if any remote leaf is in our graph
        for remote_leaf in &remote_frontier {
            if self.local_graph.contains(remote_leaf) {
                return Some(remote_leaf.clone());
            }
        }

        None
    }

    /// Find distinctions missing from remote.
    pub fn find_missing(&self, remote_frontier: &[String]) -> Vec<String> {
        let divergence = self.find_divergence(remote_frontier);
        
        // Get all distinctions from divergence point
        let local_all: HashSet<_> = match &divergence {
            Some(point) => self
                .local_graph
                .descendants(point)
                .into_iter()
                .collect(),
            None => self.local_graph.all_nodes().into_iter().collect(),
        };

        let remote_set: HashSet<_> = remote_frontier.iter().cloned().collect();

        // Find what we have that they don't
        local_all
            .difference(&remote_set)
            .cloned()
            .collect()
    }

    /// Prepare sync data to send to remote.
    pub fn prepare_sync(&self, remote_frontier: &[String]) -> SyncData {
        let missing = self.find_missing(remote_frontier);
        let merkle_root = self.compute_merkle_root();

        SyncData {
            merkle_root,
            frontier: self.local_graph.frontier(),
            missing_count: missing.len(),
            distinctions_to_send: missing,
        }
    }

    /// Apply sync data from remote.
    pub fn apply_sync(&mut self, data: &SyncData) -> Result<SyncResult, ReconciliationError> {
        let mut result = SyncResult::empty();

        // Add received distinctions
        for id in &data.distinctions_to_send {
            self.local_graph.add_node(id.clone());
            result.received.push(id.clone());
        }

        // Check for conflicts (divergent branches)
        let conflicts = self.detect_conflicts(&data.frontier);
        result.conflicts = conflicts;

        // Update stats
        self.stats.syncs_performed += 1;
        self.stats.total_received += result.received.len() as u64;
        self.stats.total_conflicts += result.conflicts.len() as u64;

        if result.is_perfect() {
            self.stats.perfect_syncs += 1;
        }

        // Calculate efficiency
        let total_distinctions = self.local_graph.node_count();
        result.efficiency = if total_distinctions > 0 {
            1.0 - (result.total_transferred() as f64 / total_distinctions as f64)
        } else {
            1.0
        };

        Ok(result)
    }

    /// Full reconcile: prepare and apply in one operation.
    pub fn reconcile(&mut self, remote_data: &SyncData) -> Result<SyncResult, ReconciliationError> {
        // First apply what they sent us
        let mut result = self.apply_sync(remote_data)?;

        // Prepare what we need to send them
        let our_data = self.prepare_sync(&remote_data.frontier);
        result.sent = our_data.distinctions_to_send;

        self.stats.total_sent += result.sent.len() as u64;

        Ok(result)
    }

    /// Merge two causal graphs.
    ///
    /// Combines remote graph into local, handling conflicts as branches.
    pub fn merge_graphs(&mut self, remote_graph: &CausalGraph) -> MergeResult {
        let local_nodes: HashSet<_> = self.local_graph.all_nodes().into_iter().collect();
        let remote_nodes: HashSet<_> = remote_graph.all_nodes().into_iter().collect();

        // Find common nodes
        let common: HashSet<_> = local_nodes.intersection(&remote_nodes).cloned().collect();

        // Find unique to remote
        let remote_unique: Vec<_> = remote_nodes
            .difference(&local_nodes)
            .cloned()
            .collect();

        // Add remote unique nodes
        for id in &remote_unique {
            self.local_graph.add_node(id.clone());
        }

        // Add edges from remote graph
        for id in &remote_nodes {
            let parents = remote_graph.ancestors(id);
            for parent in parents {
                if self.local_graph.contains(&parent) && self.local_graph.contains(id) {
                    self.local_graph.add_edge(parent, id.clone());
                }
            }
        }

        // Detect conflicts (divergent paths from common ancestor)
        let conflicts = self.detect_conflicts(&remote_graph.frontier());

        MergeResult {
            added: remote_unique.len(),
            common: common.len(),
            conflicts,
        }
    }

    /// Detect conflicts (divergent branches).
    fn detect_conflicts(&self, remote_frontier: &[String]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for remote_id in remote_frontier {
            // If we have this node, no conflict
            if self.local_graph.contains(remote_id) {
                continue;
            }

            // Check if this is a divergent branch
            let remote_ancestors: HashSet<_> = if let Some(divergence) = 
                self.find_divergence(std::slice::from_ref(remote_id)) {
                // Find path from divergence to remote
                self.local_graph
                    .ancestors(remote_id)
                    .into_iter()
                    .filter(|a| *a != divergence)
                    .collect()
            } else {
                continue;
            };

            // If we have divergent paths, it's a conflict
            if !remote_ancestors.is_empty() {
                conflicts.push(Conflict {
                    key: format!("node_{}", remote_id),
                    our_version: "local".to_string(),
                    their_version: remote_id.clone(),
                    common_ancestor: self.find_divergence(std::slice::from_ref(remote_id)),
                });
            }
        }

        conflicts
    }

    /// Compute Merkle root of current graph.
    fn compute_merkle_root(&self) -> [u8; 32] {
        let nodes = self.local_graph.all_nodes();
        let tree = MerkleTree::from_distinctions(&nodes);
        tree.root_hash()
    }

    /// Get statistics.
    pub fn stats(&self) -> &ReconciliationStats {
        &self.stats
    }

    /// Get the local causal graph.
    pub fn graph(&self) -> &CausalGraph {
        &self.local_graph
    }

    /// Get mutable access to local graph.
    pub fn graph_mut(&mut self) -> &mut CausalGraph {
        &mut self.local_graph
    }
}

/// Data sent during sync.
#[derive(Debug, Clone)]
pub struct SyncData {
    /// Merkle root for quick comparison.
    pub merkle_root: [u8; 32],
    /// Frontier (current leaves).
    pub frontier: Vec<String>,
    /// Count of missing distinctions.
    pub missing_count: usize,
    /// The actual distinctions to send.
    pub distinctions_to_send: Vec<String>,
}

/// Result of merging two graphs.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Number of distinctions added.
    pub added: usize,
    /// Number of distinctions in common.
    pub common: usize,
    /// Conflicts detected.
    pub conflicts: Vec<Conflict>,
}

/// Errors during reconciliation.
#[derive(Debug, Clone)]
pub enum ReconciliationError {
    /// Invalid data received.
    InvalidData(String),
    /// Graph merge failed.
    MergeFailed(String),
    /// Cycle detected.
    CycleDetected(String),
}

impl std::fmt::Display for ReconciliationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconciliationError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            ReconciliationError::MergeFailed(msg) => write!(f, "Merge failed: {}", msg),
            ReconciliationError::CycleDetected(node) => write!(f, "Cycle detected at: {}", node),
        }
    }
}

impl std::error::Error for ReconciliationError {}

/// Simple in-memory world for testing.
#[derive(Debug)]
pub struct World {
    /// World identifier.
    pub id: String,
    /// Causal graph.
    pub graph: CausalGraph,
}

impl World {
    /// Create a new world.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            graph: CausalGraph::new(),
        }
    }

    /// Add a distinction.
    pub fn add(&mut self, id: impl Into<String>) {
        self.graph.add_node(id.into());
    }

    /// Add a distinction with parent.
    pub fn add_with_parent(&mut self, id: impl Into<String>, parent: impl Into<String>) {
        let id = id.into();
        let parent = parent.into();
        self.graph.add_node(id.clone());
        self.graph.add_edge(parent, id);
    }

    /// Get frontier.
    pub fn frontier(&self) -> Vec<String> {
        self.graph.frontier()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_creation() {
        let mut world = World::new("test");
        world.add("a");
        world.add_with_parent("b", "a");
        world.add_with_parent("c", "b");

        assert_eq!(world.frontier(), vec!["c"]);
        assert_eq!(world.graph.node_count(), 3);
    }

    #[test]
    fn test_exchange_roots() {
        let mut world = World::new("local");
        world.add("a");
        world.add_with_parent("b", "a");

        let reconciler = WorldReconciliation::new(world.graph);
        let roots = reconciler.exchange_roots();

        assert_eq!(roots, vec!["b"]);
    }

    #[test]
    fn test_find_divergence() {
        // Local: a -> b -> c
        let mut local = World::new("local");
        local.add("a");
        local.add_with_parent("b", "a");
        local.add_with_parent("c", "b");

        let reconciler = WorldReconciliation::new(local.graph);

        // Remote has: a -> b (diverged at b)
        let divergence = reconciler.find_divergence(&["b".to_string()]);
        assert_eq!(divergence, Some("b".to_string()));
    }

    #[test]
    fn test_find_missing() {
        // Local: a -> b -> c
        let mut local = World::new("local");
        local.add("a");
        local.add_with_parent("b", "a");
        local.add_with_parent("c", "b");

        let reconciler = WorldReconciliation::new(local.graph);

        // Remote has: a -> b
        let missing = reconciler.find_missing(&["b".to_string()]);
        assert!(missing.contains(&"c".to_string()));
    }

    #[test]
    fn test_merge_graphs() {
        // Local: a -> b
        let mut local = World::new("local");
        local.add("a");
        local.add_with_parent("b", "a");

        // Remote: a -> c
        let mut remote = World::new("remote");
        remote.add("a");
        remote.add_with_parent("c", "a");

        let mut reconciler = WorldReconciliation::new(local.graph);
        let result = reconciler.merge_graphs(&remote.graph);

        assert_eq!(result.added, 1); // c added
        assert_eq!(result.common, 1); // a in common

        // Local should now have a, b, c
        assert_eq!(reconciler.graph().node_count(), 3);
    }

    #[test]
    fn test_sync_result() {
        let result = SyncResult::empty();
        assert!(result.is_perfect());
        assert_eq!(result.total_transferred(), 0);

        let result = SyncResult {
            sent: vec!["a".to_string()],
            received: vec!["b".to_string()],
            conflicts: vec![],
            efficiency: 0.5,
        };
        assert!(!result.is_perfect());
        assert_eq!(result.total_transferred(), 2);
    }
}
