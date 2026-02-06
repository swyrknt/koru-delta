/// Causal Graph: The web of becoming.
///
/// This module implements the causal graph data structure that tracks
/// how distinctions emerge from prior distinctions. Every synthesis
/// creates a node in this graph, with edges representing causality.
///
/// # The Causal Graph
///
/// The causal graph is a directed acyclic graph (DAG) where:
/// - Nodes are distinctions (DistinctionId)
/// - Edges represent causal relationships (A caused B)
///
/// ## Key Operations
///
/// - `add_node`: Add a new distinction to the graph
/// - `add_edge`: Record that one distinction caused another
/// - `ancestors`: Find all distinctions that led to this one
/// - `descendants`: Find all distinctions that flowed from this one
/// - `lca`: Find the least common ancestor (for merging)
/// - `frontier`: Find the current "leaves" of the graph
///
/// ## Biological Metaphor
///
/// Think of the causal graph as the "family tree" of ideas. Each
/// distinction has parents (what caused it) and children (what it caused).
/// The frontier is the "current generation" - the latest distinctions
/// that haven't yet caused anything new.
use dashmap::{DashMap, DashSet};
use std::collections::{HashSet, VecDeque};

/// A unique identifier for a distinction in the causal graph.
pub type DistinctionId = String;

/// The causal graph tracking how distinctions emerge from one another.
///
/// This is the foundation of the distinction-driven system. Every synthesis
/// adds nodes and edges to this graph, creating a complete history of
/// how the system has evolved.
#[derive(Debug, Default)]
pub struct CausalGraph {
    /// For each distinction, its causal parents (what caused it)
    /// A distinction can have multiple parents (merge points)
    parents: DashMap<DistinctionId, Vec<DistinctionId>>,

    /// For each distinction, its children (what it caused)
    /// A distinction can have multiple children (branching)
    children: DashMap<DistinctionId, Vec<DistinctionId>>,

    /// All distinctions in the graph
    nodes: DashSet<DistinctionId>,

    /// Current epoch (for garbage collection)
    epoch: std::sync::atomic::AtomicU64,
}

impl CausalGraph {
    /// Create a new empty causal graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a distinction to the graph.
    ///
    /// This creates a new node with no parents or children.
    /// Use `add_edge` to establish causal relationships.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this distinction
    ///
    /// # Example
    ///
    /// ```rust
    /// use koru_delta::causal_graph::CausalGraph;
    /// let graph = CausalGraph::new();
    /// graph.add_node("dist_1".to_string());
    /// ```
    pub fn add_node(&self, id: DistinctionId) {
        self.nodes.insert(id);
    }

    /// Add a causal edge: `parent` caused `child`.
    ///
    /// This establishes that one distinction causally precedes another.
    /// Nodes are auto-created if they don't exist (for persistence replay support).
    ///
    /// # Arguments
    ///
    /// * `parent` - The causing distinction
    /// * `child` - The caused distinction
    pub fn add_edge(&self, parent: DistinctionId, child: DistinctionId) {
        // Auto-create nodes if they don't exist (needed for persistence replay
        // where entries may not be in causal order)
        self.nodes.insert(parent.clone());
        self.nodes.insert(child.clone());

        // Add to parent's children list
        self.children
            .entry(parent.clone())
            .or_default()
            .push(child.clone());

        // Add to child's parents list
        self.parents.entry(child).or_default().push(parent);
    }

    /// Add a node with its parents in one operation.
    ///
    /// This is a convenience method for synthesis - when a new distinction
    /// emerges from prior distinctions.
    ///
    /// # Arguments
    ///
    /// * `id` - The new distinction's ID
    /// * `parents` - The distinctions that caused this one
    pub fn add_with_parents(&self, id: DistinctionId, parents: Vec<DistinctionId>) {
        self.add_node(id.clone());

        for parent in parents {
            debug_assert!(
                self.nodes.contains(&parent),
                "Parent {} must exist",
                parent
            );
            self.add_edge(parent, id.clone());
        }
    }

    /// Get all ancestors of a distinction (causal history).
    pub fn ancestors(&self, id: impl AsRef<str>) -> Vec<DistinctionId> {
        let id = id.as_ref();
        if !self.nodes.contains(id) {
            return Vec::new();
        }

        let mut ancestors = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with immediate parents
        if let Some(parents) = self.parents.get(id) {
            for parent in parents.iter() {
                if visited.insert(parent.clone()) {
                    queue.push_back(parent.clone());
                }
            }
        }

        // BFS through ancestors
        while let Some(current) = queue.pop_front() {
            ancestors.push(current.clone());

            if let Some(grandparents) = self.parents.get(&current) {
                for gp in grandparents.iter() {
                    if visited.insert(gp.clone()) {
                        queue.push_back(gp.clone());
                    }
                }
            }
        }

        ancestors
    }

    /// Get all descendants of a distinction (causal future).
    pub fn descendants(&self, id: impl AsRef<str>) -> Vec<DistinctionId> {
        let id = id.as_ref();
        if !self.nodes.contains(id) {
            return Vec::new();
        }

        let mut descendants = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with immediate children
        if let Some(children) = self.children.get(id) {
            for child in children.iter() {
                if visited.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }

        // BFS through descendants
        while let Some(current) = queue.pop_front() {
            descendants.push(current.clone());

            if let Some(grandchildren) = self.children.get(&current) {
                for gc in grandchildren.iter() {
                    if visited.insert(gc.clone()) {
                        queue.push_back(gc.clone());
                    }
                }
            }
        }

        descendants
    }

    /// Find the least common ancestor (LCA) of two distinctions.
    pub fn lca(&self, a: impl AsRef<str>, b: impl AsRef<str>) -> Option<DistinctionId> {
        let a = a.as_ref();
        let b = b.as_ref();
        if !self.nodes.contains(a) || !self.nodes.contains(b) {
            return None;
        }

        // Special case: if one is ancestor of other
        let ancestors_a: HashSet<_> = self.ancestors(a).into_iter().collect();
        let ancestors_b: HashSet<_> = self.ancestors(b).into_iter().collect();

        let a_string = a.to_string();
        let b_string = b.to_string();

        if ancestors_b.contains(&a_string) {
            return Some(a_string);
        }
        if ancestors_a.contains(&b_string) {
            return Some(b_string);
        }

        // Find common ancestors
        let common: HashSet<_> = ancestors_a.intersection(&ancestors_b).cloned().collect();

        if common.is_empty() {
            return None;
        }

        // Find the "deepest" common ancestor (furthest from root)
        // This is the one with the most descendants in common set
        let common_for_closure = common.clone();
        common
            .into_iter()
            .max_by_key(|candidate| {
                let descendants = self.descendants(candidate);
                descendants.iter().filter(|d| common_for_closure.contains(*d)).count()
            })
    }

    /// Get the causal frontier (all leaf nodes).
    ///
    /// The frontier consists of distinctions that have no children -
    /// they represent the "current state" of the system.
    ///
    /// # Returns
    ///
    /// A vector of all frontier distinction IDs.
    pub fn frontier(&self) -> Vec<DistinctionId> {
        self.nodes
            .iter()
            .filter(|node| {
                // A node is in the frontier if it has no children
                self.children
                    .get(node.key())
                    .map(|c| c.is_empty())
                    .unwrap_or(true)
            })
            .map(|node| node.key().clone())
            .collect()
    }

    /// Get all roots (distinctions with no parents).
    ///
    /// Roots are the "genesis" distinctions - they emerged without
    /// causal predecessors in this graph.
    ///
    /// # Returns
    ///
    /// A vector of all root distinction IDs.
    pub fn roots(&self) -> Vec<DistinctionId> {
        self.nodes
            .iter()
            .filter(|node| {
                self.parents
                    .get(node.key())
                    .map(|p| p.is_empty())
                    .unwrap_or(true)
            })
            .map(|node| node.key().clone())
            .collect()
    }

    /// Check if a node exists in the graph.
    pub fn contains(&self, id: impl AsRef<str>) -> bool {
        self.nodes.contains(id.as_ref())
    }

    /// Get the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.parents.iter().map(|e| e.value().len()).sum()
    }

    /// Get all nodes in the graph.
    ///
    /// Returns a vector of all distinction IDs.
    pub fn all_nodes(&self) -> Vec<DistinctionId> {
        self.nodes.iter().map(|n| n.key().clone()).collect()
    }

    /// Increment the epoch (for garbage collection).
    pub fn increment_epoch(&self) -> u64 {
        self.epoch
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the current epoch.
    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let graph = CausalGraph::new();
        graph.add_node("a".to_string());
        assert!(graph.contains("a"));
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let graph = CausalGraph::new();
        graph.add_node("parent".to_string());
        graph.add_node("child".to_string());
        graph.add_edge("parent".to_string(), "child".to_string());

        let ancestors = graph.ancestors("child");
        assert_eq!(ancestors, vec!["parent".to_string()]);
    }

    #[test]
    fn test_ancestors_chain() {
        let graph = CausalGraph::new();
        // a -> b -> c
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_edge("a".to_string(), "b".to_string());
        graph.add_edge("b".to_string(), "c".to_string());

        let ancestors_c = graph.ancestors("c");
        assert_eq!(ancestors_c.len(), 2);
        assert!(ancestors_c.contains(&"a".to_string()));
        assert!(ancestors_c.contains(&"b".to_string()));
    }

    #[test]
    fn test_descendants() {
        let graph = CausalGraph::new();
        // a -> b -> c
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_edge("a".to_string(), "b".to_string());
        graph.add_edge("b".to_string(), "c".to_string());

        let descendants_a = graph.descendants("a");
        assert_eq!(descendants_a.len(), 2);
        assert!(descendants_a.contains(&"b".to_string()));
        assert!(descendants_a.contains(&"c".to_string()));
    }

    #[test]
    fn test_lca_direct_parent() {
        let graph = CausalGraph::new();
        // a -> b
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_edge("a".to_string(), "b".to_string());

        // LCA of a and b should be a (since a is ancestor of b)
        let lca = graph.lca("a", "b");
        assert_eq!(lca, Some("a".to_string()));
    }

    #[test]
    fn test_lca_common_ancestor() {
        let graph = CausalGraph::new();
        //   a
        //  / \
        // b   c
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_edge("a".to_string(), "b".to_string());
        graph.add_edge("a".to_string(), "c".to_string());

        let lca = graph.lca("b", "c");
        assert_eq!(lca, Some("a".to_string()));
    }

    #[test]
    fn test_frontier() {
        let graph = CausalGraph::new();
        // a -> b -> c
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_edge("a".to_string(), "b".to_string());
        graph.add_edge("b".to_string(), "c".to_string());

        let frontier = graph.frontier();
        assert_eq!(frontier, vec!["c".to_string()]);
    }

    #[test]
    fn test_roots() {
        let graph = CausalGraph::new();
        // a -> b
        // c (orphan)
        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_edge("a".to_string(), "b".to_string());

        let roots = graph.roots();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&"a".to_string()));
        assert!(roots.contains(&"c".to_string()));
    }

    #[test]
    fn test_merge_scenario() {
        // Simulate a merge scenario:
        //   a
        //  / \
        // b   c
        //  \ /
        //   d (merge of b and c)
        let graph = CausalGraph::new();

        graph.add_node("a".to_string());
        graph.add_node("b".to_string());
        graph.add_node("c".to_string());
        graph.add_node("d".to_string());

        graph.add_edge("a".to_string(), "b".to_string());
        graph.add_edge("a".to_string(), "c".to_string());

        // d has two parents: b and c (merge)
        graph.add_with_parents("d".to_string(), vec!["b".to_string(), "c".to_string()]);

        // d's ancestors should include a, b, c
        let ancestors_d = graph.ancestors("d");
        assert_eq!(ancestors_d.len(), 3);

        // LCA of b and c should be a
        let lca = graph.lca("b", "c");
        assert_eq!(lca, Some("a".to_string()));

        // Frontier should be d
        let frontier = graph.frontier();
        assert_eq!(frontier, vec!["d".to_string()]);
    }
}
