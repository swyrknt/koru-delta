/// Set Reconciliation for World Synchronization.
///
/// This module provides efficient set reconciliation for distributed nodes.
/// Instead of sending entire databases, nodes exchange compact representations
/// (Merkle trees, Bloom filters) to find exactly what's missing.
///
/// ## The Protocol
///
/// 1. **Compare Roots**: Exchange Merkle tree root hashes
/// 2. **Drill Down**: If roots differ, recursively compare children
/// 3. **Bloom Filter Fallback**: For large differences, use Bloom filters
/// 4. **Send Missing**: Only transmit distinctions the other node lacks
///
/// ## LCA Architecture
///
/// ReconciliationAgent implements `LocalCausalAgent`, making all sync operations
/// causal distinctions. The formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
///
/// ## Example
///
/// ```rust,ignore
/// use koru_delta::reconciliation::ReconciliationAgent;
///
/// let mut agent = ReconciliationAgent::new();
/// agent.add_local_distinction("dist_1".to_string());
/// agent.add_local_distinction("dist_2".to_string());
///
/// // Compare with remote (32-byte root hash from network)
/// let remote_root = [0u8; 32];
/// let missing = agent.compare_merkle_root(&remote_root);
/// ```
pub mod bloom;
pub mod merkle;
pub mod world;

pub use bloom::{BloomExchange, BloomFilter};
pub use merkle::{MerkleNode, MerkleTree};
pub use world::{SyncResult, WorldReconciliation};

use crate::actions::{ConflictResolution, ReconciliationAction};
use crate::causal_graph::LineageAgent;
use crate::engine::SharedEngine;
use crate::roots::KoruRoots;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::collections::HashSet;
use std::sync::Arc;

/// Strategy for set reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SyncStrategy {
    /// Use Merkle tree comparison (exact, O(log n) bandwidth).
    #[default]
    MerkleTree,
    /// Use Bloom filter (probabilistic, O(1) bandwidth).
    BloomFilter { expected_items: usize, fp_rate: f64 },
    /// Hybrid: Bloom filter first, then Merkle for differences.
    Hybrid { threshold: usize },
}

/// Reconciliation agent implementing LocalCausalAgent trait.
///
/// Follows the LCA formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
/// All reconciliation operations are causal distinctions synthesized from the reconciliation root.
#[derive(Debug, Clone)]
pub struct ReconciliationAgent {
    /// LCA: Local root distinction (Root: RECONCILIATION)
    local_root: Distinction,

    /// LCA: Handle to the unified field
    _field: SharedEngine,

    /// The underlying distinction engine for content addressing
    engine: Arc<DistinctionEngine>,

    /// Local distinction IDs.
    local_distinctions: HashSet<String>,
    /// Strategy for sync.
    #[allow(dead_code)]
    strategy: SyncStrategy,
    /// Cached Merkle tree (rebuilt on changes).
    cached_tree: Option<MerkleTree>,
    /// Whether cache is stale.
    cache_dirty: bool,
}

impl ReconciliationAgent {
    /// Create a new reconciliation agent with LCA pattern.
    ///
    /// The agent initializes from the reconciliation canonical root,
    /// establishing its causal anchor in the field.
    pub fn new() -> Self {
        let field = SharedEngine::new();
        Self::with_strategy_and_field(SyncStrategy::default(), &field)
    }

    /// Create with specific strategy.
    pub fn with_strategy(strategy: SyncStrategy) -> Self {
        let field = SharedEngine::new();
        Self::with_strategy_and_field(strategy, &field)
    }

    /// Create with strategy and shared field.
    pub fn with_strategy_and_field(strategy: SyncStrategy, field: &SharedEngine) -> Self {
        let engine = Arc::clone(field.inner());
        let roots = KoruRoots::initialize(&engine);
        let local_root = roots.reconciliation.clone();

        Self {
            local_root,
            _field: field.clone(),
            engine,
            local_distinctions: HashSet::new(),
            strategy,
            cached_tree: None,
            cache_dirty: true,
        }
    }

    /// Get the local root distinction.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Apply a reconciliation action, synthesizing new state.
    ///
    /// This is the primary interface for reconciliation operations following
    /// the LCA formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
    pub fn apply_action(&mut self, action: ReconciliationAction) -> Distinction {
        let engine = Arc::clone(&self.engine);
        let new_root = self.synthesize_action(action, &engine);
        self.local_root = new_root.clone();
        new_root
    }

    /// Add a local distinction.
    pub fn add_local_distinction(&mut self, id: String) {
        self.local_distinctions.insert(id);
        self.cache_dirty = true;
    }

    /// Add multiple local distinctions.
    pub fn add_local_distinctions(&mut self, ids: impl IntoIterator<Item = String>) {
        for id in ids {
            self.local_distinctions.insert(id);
        }
        self.cache_dirty = true;
    }

    /// Remove a local distinction.
    pub fn remove_local_distinction(&mut self, id: &str) -> bool {
        let removed = self.local_distinctions.remove(id);
        if removed {
            self.cache_dirty = true;
        }
        removed
    }

    /// Get the number of local distinctions.
    pub fn len(&self) -> usize {
        self.local_distinctions.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.local_distinctions.is_empty()
    }

    /// Check if we have a distinction.
    pub fn has_distinction(&self, id: &str) -> bool {
        self.local_distinctions.contains(id)
    }

    /// Get local distinction IDs.
    pub fn distinctions(&self) -> Vec<String> {
        self.local_distinctions.iter().cloned().collect()
    }

    /// Get the Merkle tree root hash.
    ///
    /// This is a compact representation (32 bytes) of the entire set.
    pub fn merkle_root(&mut self) -> [u8; 32] {
        self.ensure_cache();
        self.cached_tree
            .as_ref()
            .map(|t| t.root_hash())
            .unwrap_or([0; 32])
    }

    /// Get the full Merkle tree.
    pub fn merkle_tree(&mut self) -> Option<MerkleTree> {
        self.ensure_cache();
        self.cached_tree.clone()
    }

    /// Create a Bloom filter of local distinctions.
    pub fn bloom_filter(&self, expected_items: usize, fp_rate: f64) -> BloomFilter {
        let mut filter = BloomFilter::new(expected_items, fp_rate);
        for id in &self.local_distinctions {
            filter.insert(id);
        }
        filter
    }

    /// Compare with a remote Merkle root.
    ///
    /// Returns distinctions we have that might be missing from remote.
    pub fn compare_merkle_root(&mut self, remote_root: &[u8; 32]) -> Option<Vec<String>> {
        self.ensure_cache();

        let local_root = self.cached_tree.as_ref()?.root_hash();
        if local_root == *remote_root {
            // Roots match—sets are identical
            return Some(vec![]);
        }

        // We need the remote's tree to do proper diff
        // Return None to indicate we need more data
        None
    }

    /// Find missing distinctions given a remote tree.
    pub fn find_missing_with_tree(&mut self, remote_tree: &MerkleTree) -> Vec<String> {
        self.ensure_cache();

        if let Some(ref local_tree) = self.cached_tree {
            let missing: Vec<_> = local_tree.diff(remote_tree).into_iter().collect();
            missing
        } else {
            vec![]
        }
    }

    /// Find missing distinctions using Bloom filter.
    ///
    /// Returns distinctions we have that are definitely not in the remote filter.
    pub fn find_missing_with_bloom(&self, remote_filter: &BloomFilter) -> Vec<String> {
        self.local_distinctions
            .iter()
            .filter(|id| remote_filter.definitely_not_contain(id))
            .cloned()
            .collect()
    }

    /// Reconcile with a causal graph.
    ///
    /// Returns distinctions in our graph that are missing from the remote graph.
    pub fn reconcile_with_graph(&self, remote_graph: &LineageAgent) -> Vec<String> {
        // Get all distinctions from remote graph
        let remote_distinctions: HashSet<_> = remote_graph.all_nodes().into_iter().collect();

        // Find what we have that they don't
        self.local_distinctions
            .difference(&remote_distinctions)
            .cloned()
            .collect()
    }

    /// Clear all distinctions.
    pub fn clear(&mut self) {
        self.local_distinctions.clear();
        self.cached_tree = None;
        self.cache_dirty = true;
    }

    /// Ensure the Merkle tree cache is up to date.
    fn ensure_cache(&mut self) {
        if self.cache_dirty {
            let distinctions: Vec<_> = self.local_distinctions.iter().cloned().collect();
            self.cached_tree = Some(MerkleTree::from_distinctions(&distinctions));
            self.cache_dirty = false;
        }
    }

    /// Start sync with synthesis.
    pub fn start_sync_synthesized(&mut self, peer_id: String) -> Distinction {
        let action = ReconciliationAction::StartSync { peer_id };
        self.apply_action(action)
    }

    /// Exchange roots with synthesis.
    pub fn exchange_roots_synthesized(&mut self, peer_frontier: [u8; 32]) -> Distinction {
        let action = ReconciliationAction::ExchangeRoots { peer_frontier };
        self.apply_action(action)
    }

    /// Request differences with synthesis.
    pub fn request_differences_synthesized(&mut self, divergence_point: String) -> Distinction {
        let action = ReconciliationAction::RequestDifferences { divergence_point };
        self.apply_action(action)
    }

    /// Apply delta with synthesis.
    pub fn apply_delta_synthesized(&mut self, changes: Vec<String>) -> Distinction {
        let action = ReconciliationAction::ApplyDelta { changes };
        self.apply_action(action)
    }

    /// Resolve conflict with synthesis.
    pub fn resolve_conflict_synthesized(
        &mut self,
        conflict_id: String,
        resolution: ConflictResolution,
    ) -> Distinction {
        let action = ReconciliationAction::ResolveConflict {
            conflict_id,
            resolution,
        };
        self.apply_action(action)
    }

    /// Complete sync with synthesis.
    pub fn complete_sync_synthesized(&mut self, peer_id: String) -> Distinction {
        let action = ReconciliationAction::CompleteSync { peer_id };
        self.apply_action(action)
    }

    /// Get sync status with synthesis.
    pub fn get_sync_status_synthesized(&mut self) -> Distinction {
        let action = ReconciliationAction::GetSyncStatus;
        self.apply_action(action)
    }
}

// LCA Trait Implementation
impl LocalCausalAgent for ReconciliationAgent {
    type ActionData = ReconciliationAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: ReconciliationAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

impl Default for ReconciliationAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// Find missing distinctions between two sets.
///
/// Simple utility function for basic set difference.
pub fn find_missing(local: &[String], remote: &[String]) -> Vec<String> {
    let remote_set: HashSet<_> = remote.iter().cloned().collect();
    local
        .iter()
        .filter(|id| !remote_set.contains(*id))
        .cloned()
        .collect()
}

/// Estimate sync efficiency.
///
/// Returns the ratio of distinctions that need to be sent vs total.
pub fn sync_efficiency(missing_count: usize, total_count: usize) -> f64 {
    if total_count == 0 {
        1.0
    } else {
        1.0 - (missing_count as f64 / total_count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_basic() {
        let mut manager = ReconciliationAgent::new();
        assert!(manager.is_empty());

        manager.add_local_distinction("dist_1".to_string());
        assert_eq!(manager.len(), 1);
        assert!(manager.has_distinction("dist_1"));

        manager.add_local_distinction("dist_2".to_string());
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_merkle_root() {
        let mut manager = ReconciliationAgent::new();

        let root1 = manager.merkle_root();
        assert_eq!(root1, [0; 32]); // Empty

        manager.add_local_distinction("test".to_string());
        let root2 = manager.merkle_root();
        assert_ne!(root2, root1);
    }

    #[test]
    fn test_deterministic_root() {
        let mut manager1 = ReconciliationAgent::new();
        let mut manager2 = ReconciliationAgent::new();

        for i in 0..10 {
            manager1.add_local_distinction(format!("dist_{}", i));
            manager2.add_local_distinction(format!("dist_{}", i));
        }

        assert_eq!(manager1.merkle_root(), manager2.merkle_root());
    }

    #[test]
    fn test_bloom_filter() {
        let mut manager = ReconciliationAgent::new();
        for i in 0..100 {
            manager.add_local_distinction(format!("dist_{}", i));
        }

        let filter = manager.bloom_filter(100, 0.01);

        // All inserted items should be found
        for i in 0..100 {
            assert!(filter.might_contain(&format!("dist_{}", i)));
        }
    }

    #[test]
    fn test_find_missing() {
        let local = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let remote = vec!["a".to_string(), "b".to_string()];

        let missing = find_missing(&local, &remote);
        assert_eq!(missing, vec!["c"]);
    }

    #[test]
    fn test_sync_efficiency() {
        assert_eq!(sync_efficiency(0, 100), 1.0); // Perfect
        assert_eq!(sync_efficiency(50, 100), 0.5); // 50% efficient
        assert_eq!(sync_efficiency(100, 100), 0.0); // Nothing in common
    }

    // LCA Tests
    mod lca_tests {
        use super::*;
        use koru_lambda_core::LocalCausalAgent;

        fn setup_agent() -> ReconciliationAgent {
            ReconciliationAgent::new()
        }

        #[test]
        fn test_reconciliation_agent_implements_lca_trait() {
            let agent = setup_agent();

            // Verify trait is implemented
            let _root = agent.get_current_root();
        }

        #[test]
        fn test_reconciliation_agent_has_unique_local_root() {
            let agent1 = ReconciliationAgent::new();
            let agent2 = ReconciliationAgent::new();

            // Each agent should have the same reconciliation root from canonical roots
            assert_eq!(
                agent1.local_root().id(),
                agent2.local_root().id(),
                "Reconciliation agents share the same canonical root"
            );
        }

        #[test]
        fn test_start_sync_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.start_sync_synthesized("peer-1".to_string());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_exchange_roots_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.exchange_roots_synthesized([1u8; 32]);

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after exchange roots synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_apply_delta_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root =
                agent.apply_delta_synthesized(vec!["dist1".to_string(), "dist2".to_string()]);

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after apply delta synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_resolve_conflict_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.resolve_conflict_synthesized(
                "conflict-1".to_string(),
                ConflictResolution::PreferLocal,
            );

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after resolve conflict synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_complete_sync_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.complete_sync_synthesized("peer-1".to_string());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after complete sync synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_get_sync_status_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.get_sync_status_synthesized();

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after get sync status synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_apply_action_changes_root() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let action = ReconciliationAction::GetSyncStatus;
            let new_root = agent.apply_action(action);

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after apply_action"
            );
            assert_eq!(new_root.id(), root_after);
        }
    }
}
