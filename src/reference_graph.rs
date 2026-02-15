/// Reference Graph: Tracking what points to what.
///
/// This module implements reference tracking for the distinction-driven
/// system. It tracks which distinctions reference which other distinctions,
/// enabling garbage collection and reachability analysis.
///
/// # The Reference Graph
///
/// The reference graph is a directed graph where:
/// - Nodes are distinctions (DistinctionId)
/// - Edges represent "references" (A points to B)
///
/// ## Key Operations
///
/// - `add_reference`: Record that one distinction references another
/// - `reference_count`: Get how many distinctions point to this one (for GC)
/// - `is_reachable`: Check if a distinction is reachable from roots
/// - `find_garbage`: Find distinctions that can be collected
///
/// ## Relationship to Causal Graph
///
/// The causal graph tracks "became from" (causality).
/// The reference graph tracks "points to" (reference).
///
/// A distinction can be:
/// - Causally reachable (via causal graph) - it contributed to current state
/// - Referentially reachable (via reference graph) - it's still being used
///
/// ## Garbage Collection
///
/// A distinction is "garbage" if:
/// 1. No current distinction references it (ref_count == 0)
/// 2. AND it's not in the current causal frontier
/// 3. AND it's not a root distinction
use dashmap::DashMap;
use std::collections::HashSet;

use crate::causal_graph::{CausalGraph, DistinctionId};

/// The reference graph tracking which distinctions reference which.
///
/// This is essential for:
/// - Garbage collection (what's still used?)
/// - Memory management (what can be evicted?)
/// - Optimization (what's heavily referenced? keep it hot)
#[derive(Debug, Default)]
pub struct ReferenceGraph {
    /// For each distinction, what it points TO (outgoing references)
    outgoing: DashMap<DistinctionId, Vec<DistinctionId>>,

    /// For each distinction, what points TO it (incoming references)
    /// This is used for reference counting GC
    incoming: DashMap<DistinctionId, Vec<DistinctionId>>,

    /// All distinctions in the graph
    nodes: DashMap<DistinctionId, ()>,
}

impl ReferenceGraph {
    /// Create a new empty reference graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a distinction to the graph.
    ///
    /// This creates a new node with no references.
    /// Use `add_reference` to establish reference relationships.
    pub fn add_node(&self, id: DistinctionId) {
        self.nodes.insert(id, ());
    }

    /// Add a reference edge: `from` references `to`.
    ///
    /// This records that one distinction points to another.
    /// Both nodes must already exist in the graph.
    ///
    /// # Arguments
    ///
    /// * `from` - The distinction that has the reference
    /// * `to` - The distinction being referenced
    pub fn add_reference(&self, from: DistinctionId, to: DistinctionId) {
        debug_assert!(self.nodes.contains_key(&from), "From node must exist");
        debug_assert!(self.nodes.contains_key(&to), "To node must exist");

        // Add to from's outgoing list
        self.outgoing
            .entry(from.clone())
            .or_default()
            .push(to.clone());

        // Add to to's incoming list
        self.incoming.entry(to).or_default().push(from);
    }

    /// Get all distinctions that this one references.
    pub fn references(&self, id: &DistinctionId) -> Vec<DistinctionId> {
        self.outgoing.get(id).map(|v| v.clone()).unwrap_or_default()
    }

    /// Get all distinctions that reference this one.
    pub fn referrers(&self, id: &DistinctionId) -> Vec<DistinctionId> {
        self.incoming.get(id).map(|v| v.clone()).unwrap_or_default()
    }

    /// Get the reference count (how many distinctions point to this).
    ///
    /// This is the key metric for garbage collection.
    /// A distinction with reference_count == 0 may be garbage.
    pub fn reference_count(&self, id: &DistinctionId) -> usize {
        self.incoming.get(id).map(|v| v.len()).unwrap_or(0)
    }

    /// Check if a distinction is reachable from any root.
    ///
    /// A distinction is reachable if there's a path from any root
    /// to it through the reference graph.
    ///
    /// # Arguments
    ///
    /// * `id` - The distinction to check
    /// * `roots` - The root distinctions (usually the causal frontier)
    pub fn is_reachable(&self, id: &DistinctionId, roots: &[DistinctionId]) -> bool {
        if roots.contains(id) {
            return true;
        }

        let mut visited = HashSet::new();
        let mut to_visit: Vec<_> = roots.to_vec();

        while let Some(current) = to_visit.pop() {
            if &current == id {
                return true;
            }

            if visited.insert(current.clone()) {
                // Check what this distinction references
                let refs = self.references(&current);
                for r in refs {
                    if !visited.contains(&r) {
                        to_visit.push(r);
                    }
                }
            }
        }

        false
    }

    /// Find all garbage distinctions.
    ///
    /// A distinction is garbage if:
    /// 1. It has no incoming references
    /// 2. It's not in the causal frontier (not current state)
    /// 3. It's not a root of the causal graph
    ///
    /// # Arguments
    ///
    /// * `causal_graph` - The causal graph for context
    ///
    /// # Returns
    ///
    /// A vector of distinction IDs that can be garbage collected.
    pub fn find_garbage(&self, causal_graph: &CausalGraph) -> Vec<DistinctionId> {
        let frontier: HashSet<_> = causal_graph.frontier().into_iter().collect();
        let roots: HashSet<_> = causal_graph.roots().into_iter().collect();

        self.nodes
            .iter()
            .filter_map(|entry| {
                let id = entry.key();

                // Skip if in frontier (current state)
                if frontier.contains(id) {
                    return None;
                }

                // Skip if root (genesis distinction)
                if roots.contains(id) {
                    return None;
                }

                // Check reference count
                if self.reference_count(id) == 0 {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find all distinctions that are "heavily referenced".
    ///
    /// These are candidates for keeping in hot memory.
    pub fn find_hot_candidates(&self, threshold: usize) -> Vec<DistinctionId> {
        let mut candidates: Vec<_> = self
            .nodes
            .iter()
            .filter_map(|entry| {
                let id = entry.key();
                let count = self.reference_count(id);
                if count >= threshold {
                    Some((id.clone(), count))
                } else {
                    None
                }
            })
            .collect();

        // Sort by reference count descending
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.into_iter().map(|(id, _)| id).collect()
    }

    /// Remove a distinction and all its references.
    ///
    /// This is called during garbage collection.
    pub fn remove(&self, id: &DistinctionId) {
        // Remove outgoing references
        if let Some((_, refs)) = self.outgoing.remove(id) {
            for r in refs {
                if let Some(mut incoming) = self.incoming.get_mut(&r) {
                    incoming.retain(|x| x != id);
                }
            }
        }

        // Remove incoming references
        if let Some((_, refs)) = self.incoming.remove(id) {
            for r in refs {
                if let Some(mut outgoing) = self.outgoing.get_mut(&r) {
                    outgoing.retain(|x| x != id);
                }
            }
        }

        // Remove node
        self.nodes.remove(id);
    }

    /// Get the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of reference edges.
    pub fn edge_count(&self) -> usize {
        self.outgoing.iter().map(|e| e.value().len()).sum()
    }

    /// Check if a node exists.
    pub fn contains(&self, id: &DistinctionId) -> bool {
        self.nodes.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let graph = ReferenceGraph::new();
        graph.add_node("a".to_string());
        assert!(graph.contains(&"a".to_string()));
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_reference() {
        let graph = ReferenceGraph::new();
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_reference("a".to_string(), "b".to_string());

        assert_eq!(graph.references(&"a".to_string()), vec!["b".to_string()]);
        assert_eq!(graph.referrers(&"b".to_string()), vec!["a".to_string()]);
        assert_eq!(graph.reference_count(&"b".to_string()), 1);
    }

    #[test]
    fn test_reference_count_multiple() {
        let graph = ReferenceGraph::new();
        // a -> c, b -> c (c has 2 references)
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_reference("a".to_string(), "c".to_string());
        graph.add_reference("b".to_string(), "c".to_string());

        assert_eq!(graph.reference_count(&"c".to_string()), 2);
        assert_eq!(graph.reference_count(&"a".to_string()), 0);
    }

    #[test]
    fn test_is_reachable() {
        let graph = ReferenceGraph::new();
        // a -> b -> c
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_reference("a".to_string(), "b".to_string());
        graph.add_reference("b".to_string(), "c".to_string());

        assert!(graph.is_reachable(&"a".to_string(), &["a".to_string()]));
        assert!(graph.is_reachable(&"b".to_string(), &["a".to_string()]));
        assert!(graph.is_reachable(&"c".to_string(), &["a".to_string()]));
        assert!(!graph.is_reachable(&"a".to_string(), &["b".to_string()]));
    }

    #[test]
    fn test_find_garbage() {
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new(&crate::engine::SharedEngine::new());

        // Setup causal graph:
        // root -> orphan -> current
        // - root: is a root (no parents), NOT in frontier (has child)
        // - orphan: NOT a root (has parent), NOT in frontier (has child)
        // - current: NOT a root (has parent), IS in frontier (no children)
        causal_graph.add_node("root".to_string());
        causal_graph.add_node("orphan".to_string());
        causal_graph.add_node("current".to_string());
        causal_graph.add_edge("root".to_string(), "orphan".to_string());
        causal_graph.add_edge("orphan".to_string(), "current".to_string());

        // Setup reference graph:
        // root -> current (orphan has NO references!)
        ref_graph.add_node("root".to_string());
        ref_graph.add_node("orphan".to_string());
        ref_graph.add_node("current".to_string());
        ref_graph.add_reference("root".to_string(), "current".to_string());

        let garbage = ref_graph.find_garbage(&causal_graph);
        // orphan has no references AND is not in frontier AND is not a root
        assert!(garbage.contains(&"orphan".to_string()));
        // current is in frontier (not garbage)
        assert!(!garbage.contains(&"current".to_string()));
        // root is a root (not garbage)
        assert!(!garbage.contains(&"root".to_string()));
    }

    #[test]
    fn test_find_hot_candidates() {
        let graph = ReferenceGraph::new();
        // a referenced by b, c, d (3 refs)
        // b referenced by c (1 ref)
        // c referenced by none (0 refs)

        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_node("d".to_string());

        graph.add_reference("b".to_string(), "a".to_string());
        graph.add_reference("c".to_string(), "a".to_string());
        graph.add_reference("d".to_string(), "a".to_string());
        graph.add_reference("c".to_string(), "b".to_string());

        let hot = graph.find_hot_candidates(2);
        assert_eq!(hot, vec!["a".to_string()]);
    }

    #[test]
    fn test_remove() {
        let graph = ReferenceGraph::new();
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_reference("a".to_string(), "b".to_string());

        assert_eq!(graph.reference_count(&"b".to_string()), 1);

        graph.remove(&"a".to_string());

        assert!(!graph.contains(&"a".to_string()));
        assert_eq!(graph.reference_count(&"b".to_string()), 0);
    }
}
