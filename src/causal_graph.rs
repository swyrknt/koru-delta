/// Lineage Agent: The web of becoming with LCA architecture.
///
/// This agent implements the causal graph data structure that tracks
/// how distinctions emerge from prior distinctions. Every synthesis
/// creates a node in this graph, with edges representing causality.
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Lineage Agent's local root is `RootType::Lineage` (👁️ LINEAGE).
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
use crate::actions::LineageAction;
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;
use dashmap::{DashMap, DashSet};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, RwLock};

/// A unique identifier for a distinction in the causal graph.
pub type DistinctionId = String;

/// The Lineage Agent tracking how distinctions emerge from one another with LCA architecture.
///
/// This is the foundation of the distinction-driven system. Every synthesis
/// adds nodes and edges to this graph, creating a complete history of
/// how the system has evolved.
///
/// All operations are synthesized through the unified field.
#[derive(Debug)]
pub struct LineageAgent {
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

    /// LCA: Local root distinction (Root: LINEAGE)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// Family tree - synthesis of all lineage
    /// Updated as new distinctions are added to the graph
    family_tree: RwLock<Distinction>,
}

impl LineageAgent {
    /// Create a new empty lineage agent.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Lineage (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    /// - `family_tree` = The synthesis of all lineage
    pub fn new(shared_engine: &SharedEngine) -> Self {
        let local_root = shared_engine.root(RootType::Lineage).clone();
        let field = FieldHandle::new(shared_engine);

        // Initial family tree is just the local root
        let family_tree = RwLock::new(local_root.clone());

        Self {
            parents: DashMap::new(),
            children: DashMap::new(),
            nodes: DashSet::new(),
            epoch: std::sync::atomic::AtomicU64::new(0),
            local_root,
            field,
            family_tree,
        }
    }

    /// Get the family tree - synthesis of all lineage.
    pub fn family_tree(&self) -> Distinction {
        self.family_tree.read().unwrap().clone()
    }

    /// Add a distinction to the graph.
    ///
    /// This creates a new node with no parents or children.
    /// Use `add_edge` to establish causal relationships.
    ///
    /// # LCA Pattern
    ///
    /// Birth is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔRecordBirth_Action`
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this distinction
    ///
    /// # Example
    ///
    /// ```rust
    /// use koru_delta::causal_graph::LineageAgent;
    /// use koru_delta::engine::SharedEngine;
    /// let engine = SharedEngine::new();
    /// let lineage = LineageAgent::new(&engine);
    /// lineage.add_node("dist_1".to_string());
    /// ```
    pub fn add_node(&self, id: DistinctionId) {
        // Synthesize record birth action
        let action = LineageAction::RecordBirth {
            child_id: id.clone(),
            parent_ids: vec![],
        };
        let _ = self.synthesize_action_internal(action);

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
        self.parents.entry(child.clone()).or_default().push(parent);
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
            debug_assert!(self.nodes.contains(&parent), "Parent {} must exist", parent);
            self.add_edge(parent, id.clone());
        }
    }

    /// Get all ancestors of a distinction (causal history).
    ///
    /// # LCA Pattern
    ///
    /// Tracing synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔTraceAncestors_Action`
    pub fn ancestors(&self, id: impl AsRef<str>) -> Vec<DistinctionId> {
        let id = id.as_ref();
        if !self.nodes.contains(id) {
            return Vec::new();
        }

        // Synthesize trace ancestors action
        let action = LineageAction::TraceAncestors {
            from_id: id.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

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
    ///
    /// # LCA Pattern
    ///
    /// Tracing synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔTraceDescendants_Action`
    pub fn descendants(&self, id: impl AsRef<str>) -> Vec<DistinctionId> {
        let id = id.as_ref();
        if !self.nodes.contains(id) {
            return Vec::new();
        }

        // Synthesize trace descendants action
        let action = LineageAction::TraceDescendants {
            from_id: id.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

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
    ///
    /// # LCA Pattern
    ///
    /// Finding LCA synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔFindCommonAncestor_Action`
    pub fn lca(&self, a: impl AsRef<str>, b: impl AsRef<str>) -> Option<DistinctionId> {
        let a = a.as_ref();
        let b = b.as_ref();
        if !self.nodes.contains(a) || !self.nodes.contains(b) {
            return None;
        }

        // Synthesize find common ancestor action
        let action = LineageAction::FindCommonAncestor {
            a_id: a.to_string(),
            b_id: b.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

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
        common.into_iter().max_by_key(|candidate| {
            let descendants = self.descendants(candidate);
            descendants
                .iter()
                .filter(|d| common_for_closure.contains(*d))
                .count()
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
        self.epoch.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the current epoch.
    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    /// Also updates the family_tree with each new lineage record.
    fn synthesize_action_internal(&self, action: LineageAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        
        // Update family_tree - synthesize this action into the lineage synthesis
        let current_family = self.family_tree.read().unwrap().clone();
        let new_family = engine.synthesize(&current_family, &action_distinction);
        *self.family_tree.write().unwrap() = new_family;
        
        new_root
    }
}

impl Default for LineageAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use LineageAgent::new(&shared_engine)
        panic!("LineageAgent requires a SharedEngine - use LineageAgent::new()")
    }
}

/// LCA Trait Implementation for LineageAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for LineageAgent {
    type ActionData = LineageAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: LineageAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

/// Backward-compatible type alias for existing code.
pub type CausalGraph = LineageAgent;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

    #[test]
    fn test_add_node() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        lineage.add_node("a".to_string());
        assert!(lineage.contains("a"));
        assert_eq!(lineage.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        lineage.add_node("parent".to_string());
        lineage.add_node("child".to_string());
        lineage.add_edge("parent".to_string(), "child".to_string());

        let ancestors = lineage.ancestors("child");
        assert_eq!(ancestors, vec!["parent".to_string()]);
    }

    #[test]
    fn test_ancestors_chain() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        // a -> b -> c
        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_node("c".to_string());
        lineage.add_edge("a".to_string(), "b".to_string());
        lineage.add_edge("b".to_string(), "c".to_string());

        let ancestors_c = lineage.ancestors("c");
        assert_eq!(ancestors_c.len(), 2);
        assert!(ancestors_c.contains(&"a".to_string()));
        assert!(ancestors_c.contains(&"b".to_string()));
    }

    #[test]
    fn test_descendants() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        // a -> b -> c
        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_node("c".to_string());
        lineage.add_edge("a".to_string(), "b".to_string());
        lineage.add_edge("b".to_string(), "c".to_string());

        let descendants_a = lineage.descendants("a");
        assert_eq!(descendants_a.len(), 2);
        assert!(descendants_a.contains(&"b".to_string()));
        assert!(descendants_a.contains(&"c".to_string()));
    }

    #[test]
    fn test_lca_direct_parent() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        // a -> b
        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_edge("a".to_string(), "b".to_string());

        // LCA of a and b should be a (since a is ancestor of b)
        let lca = lineage.lca("a", "b");
        assert_eq!(lca, Some("a".to_string()));
    }

    #[test]
    fn test_lca_common_ancestor() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        //   a
        //  / \
        // b   c
        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_node("c".to_string());
        lineage.add_edge("a".to_string(), "b".to_string());
        lineage.add_edge("a".to_string(), "c".to_string());

        let lca = lineage.lca("b", "c");
        assert_eq!(lca, Some("a".to_string()));
    }

    #[test]
    fn test_frontier() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        // a -> b -> c
        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_node("c".to_string());
        lineage.add_edge("a".to_string(), "b".to_string());
        lineage.add_edge("b".to_string(), "c".to_string());

        let frontier = lineage.frontier();
        assert_eq!(frontier, vec!["c".to_string()]);
    }

    #[test]
    fn test_roots() {
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);
        // a -> b
        // c (orphan)
        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_node("c".to_string());
        lineage.add_edge("a".to_string(), "b".to_string());

        let roots = lineage.roots();
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
        let engine = create_test_engine();
        let lineage = LineageAgent::new(&engine);

        lineage.add_node("a".to_string());
        lineage.add_node("b".to_string());
        lineage.add_node("c".to_string());
        lineage.add_node("d".to_string());

        lineage.add_edge("a".to_string(), "b".to_string());
        lineage.add_edge("a".to_string(), "c".to_string());

        // d has two parents: b and c (merge)
        lineage.add_with_parents("d".to_string(), vec!["b".to_string(), "c".to_string()]);

        // d's ancestors should include a, b, c
        let ancestors_d = lineage.ancestors("d");
        assert_eq!(ancestors_d.len(), 3);

        // LCA of b and c should be a
        let lca = lineage.lca("b", "c");
        assert_eq!(lca, Some("a".to_string()));

        // Frontier should be d
        let frontier = lineage.frontier();
        assert_eq!(frontier, vec!["d".to_string()]);
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = LineageAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = LineageAction::RecordBirth {
            child_id: "test123".to_string(),
            parent_ids: vec!["parent1".to_string()],
        };
        let engine_arc = Arc::clone(agent.field.engine_arc());
        let new_root = agent.synthesize_action(action, &engine_arc);
        assert!(!new_root.id().is_empty());
        assert_ne!(new_root.id(), root_id);

        // Test update_local_root
        agent.update_local_root(new_root.clone());
        assert_eq!(agent.get_current_root().id(), new_root.id());
    }

    #[test]
    fn test_backward_compatible_alias() {
        // Ensure backward compatibility works
        let engine = create_test_engine();
        let _causal_graph: CausalGraph = LineageAgent::new(&engine);
    }
}
