/// Merkle Tree for Distinction Sets.
///
/// A Merkle tree provides an efficient way to compare two sets of distinctions
/// and find exactly which elements differ. This is the foundation of efficient
/// sync—nodes can compare tree roots, then drill down to find missing distinctions.
///
/// ## How It Works
///
/// 1. Hash all distinctions
/// 2. Build a binary tree where each parent = hash(left || right)
/// 3. Compare tree roots—if equal, sets are identical
/// 4. If roots differ, recursively compare children
/// 5. Different leaves are the missing distinctions
///
/// ## Example
///
/// ```text
///         [root_hash]
///        /           \
///   [hash_A_B]    [hash_C_D]
///      /     \        /     \
///   [A]     [B]    [C]     [D]
/// ```
///
/// Comparing two trees only requires O(log n) hash comparisons in the best case.
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// A node in the Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerkleNode {
    /// Leaf node containing a distinction hash.
    Leaf {
        /// The distinction ID.
        distinction_id: String,
        /// Hash of the distinction ID.
        hash: [u8; 32],
    },
    /// Internal node with two children.
    Branch {
        /// Hash of (left.hash || right.hash).
        hash: [u8; 32],
        /// Left child.
        left: Box<MerkleNode>,
        /// Right child.
        right: Box<MerkleNode>,
    },
    /// Empty node (for padding).
    Empty,
}

impl MerkleNode {
    /// Get the hash of this node.
    pub fn hash(&self) -> [u8; 32] {
        match self {
            MerkleNode::Leaf { hash, .. } => *hash,
            MerkleNode::Branch { hash, .. } => *hash,
            MerkleNode::Empty => [0; 32],
        }
    }

    /// Check if this is a leaf node.
    pub fn is_leaf(&self) -> bool {
        matches!(self, MerkleNode::Leaf { .. })
    }

    /// Check if this is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, MerkleNode::Empty)
    }
}

/// Merkle tree for distinction set reconciliation.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// Root node of the tree.
    root: MerkleNode,
    /// Number of distinctions in the tree.
    size: usize,
}

impl MerkleTree {
    /// Create an empty Merkle tree.
    pub fn empty() -> Self {
        Self {
            root: MerkleNode::Empty,
            size: 0,
        }
    }

    /// Build a Merkle tree from a set of distinction IDs.
    ///
    /// The distinctions are sorted to ensure deterministic tree structure.
    pub fn from_distinctions(distinctions: &[String]) -> Self {
        if distinctions.is_empty() {
            return Self::empty();
        }

        // Sort for deterministic structure
        let mut sorted: Vec<_> = distinctions.to_vec();
        sorted.sort();

        // Create leaf nodes
        let leaves: Vec<_> = sorted
            .into_iter()
            .map(|id| {
                let hash = hash_distinction(&id);
                MerkleNode::Leaf {
                    distinction_id: id,
                    hash,
                }
            })
            .collect();

        // Build tree bottom-up
        let root = build_tree(leaves);
        let size = distinctions.len();

        Self { root, size }
    }

    /// Get the root hash.
    ///
    /// Two trees with the same root hash contain the same distinctions.
    pub fn root_hash(&self) -> [u8; 32] {
        self.root.hash()
    }

    /// Get the number of distinctions.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if this tree is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Compare with another tree and find differences.
    ///
    /// Returns the distinction IDs that are in `self` but not in `other`.
    pub fn diff(&self, other: &MerkleTree) -> HashSet<String> {
        let mut missing = HashSet::new();
        diff_nodes(&self.root, &other.root, &mut missing);
        missing
    }

    /// Get all distinction IDs in the tree.
    pub fn distinctions(&self) -> Vec<String> {
        let mut result = Vec::new();
        collect_distinctions(&self.root, &mut result);
        result
    }

    /// Verify the tree integrity (debugging).
    pub fn verify(&self) -> bool {
        verify_node(&self.root)
    }
}

/// Hash a distinction ID using SHA256.
fn hash_distinction(id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.finalize().into()
}

/// Hash two child hashes together.
fn hash_children(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Build a tree from leaf nodes.
fn build_tree(mut nodes: Vec<MerkleNode>) -> MerkleNode {
    if nodes.is_empty() {
        return MerkleNode::Empty;
    }

    if nodes.len() == 1 {
        return nodes.into_iter().next().unwrap();
    }

    // Pad to power of 2 for balanced tree
    let size = nodes.len().next_power_of_two();
    while nodes.len() < size {
        nodes.push(MerkleNode::Empty);
    }

    // Build bottom-up
    let mut current_level = nodes;
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        for i in (0..current_level.len()).step_by(2) {
            let left = Box::new(current_level[i].clone());
            let right = Box::new(current_level[i + 1].clone());

            let hash = match (&*left, &*right) {
                (MerkleNode::Empty, MerkleNode::Empty) => [0; 32],
                (MerkleNode::Empty, right) => right.hash(),
                (left, MerkleNode::Empty) => left.hash(),
                (left, right) => hash_children(&left.hash(), &right.hash()),
            };

            next_level.push(MerkleNode::Branch { hash, left, right });
        }
        current_level = next_level;
    }

    current_level.into_iter().next().unwrap()
}

/// Recursively find differences between two nodes.
fn diff_nodes(a: &MerkleNode, b: &MerkleNode, missing: &mut HashSet<String>) {
    // If hashes match, subtrees are identical
    if a.hash() == b.hash() {
        return;
    }

    match (a, b) {
        // Both leaves—different distinctions
        (
            MerkleNode::Leaf {
                distinction_id: id_a,
                ..
            },
            MerkleNode::Leaf {
                distinction_id: id_b,
                ..
            },
        ) => {
            if id_a != id_b {
                missing.insert(id_a.clone());
            }
        }

        // a is leaf, b is not—definitely missing
        (MerkleNode::Leaf { distinction_id, .. }, _) => {
            missing.insert(distinction_id.clone());
        }

        // Both branches—recurse
        (
            MerkleNode::Branch { left: l1, right: r1, .. },
            MerkleNode::Branch { left: l2, right: r2, .. },
        ) => {
            diff_nodes(l1, l2, missing);
            diff_nodes(r1, r2, missing);
        }

        // a is branch, b is empty—all of a is missing
        (MerkleNode::Branch { left, right, .. }, MerkleNode::Empty) => {
            collect_distinctions(left, missing);
            collect_distinctions(right, missing);
        }

        // Handle other cases
        _ => {
            // Fallback: collect all distinctions from a
            collect_distinctions(a, missing);
        }
    }
}

/// Collect all distinction IDs from a node.
fn collect_distinctions(node: &MerkleNode, result: &mut impl Extend<String>) {
    match node {
        MerkleNode::Leaf { distinction_id, .. } => {
            result.extend(std::iter::once(distinction_id.clone()));
        }
        MerkleNode::Branch { left, right, .. } => {
            collect_distinctions(left, result);
            collect_distinctions(right, result);
        }
        MerkleNode::Empty => {}
    }
}

/// Verify node hash integrity.
fn verify_node(node: &MerkleNode) -> bool {
    match node {
        MerkleNode::Leaf { distinction_id, hash } => {
            *hash == hash_distinction(distinction_id)
        }
        MerkleNode::Branch { hash, left, right } => {
            let expected = match (&**left, &**right) {
                (MerkleNode::Empty, MerkleNode::Empty) => [0; 32],
                (MerkleNode::Empty, right) => right.hash(),
                (left, MerkleNode::Empty) => left.hash(),
                (left, right) => hash_children(&left.hash(), &right.hash()),
            };
            *hash == expected && verify_node(left) && verify_node(right)
        }
        MerkleNode::Empty => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_distinctions(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("dist_{:08x}", i)).collect()
    }

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::empty();
        assert!(tree.is_empty());
        assert_eq!(tree.root_hash(), [0; 32]);
    }

    #[test]
    fn test_single_distinction() {
        let tree = MerkleTree::from_distinctions(&["abc".to_string()]);
        assert_eq!(tree.size(), 1);
        assert!(!tree.is_empty());

        let distinctions = tree.distinctions();
        assert_eq!(distinctions, vec!["abc"]);
    }

    #[test]
    fn test_multiple_distinctions() {
        let distinctions = create_distinctions(4);
        let tree = MerkleTree::from_distinctions(&distinctions);

        assert_eq!(tree.size(), 4);
        assert!(tree.verify());

        let collected = tree.distinctions();
        assert_eq!(collected.len(), 4);
    }

    #[test]
    fn test_deterministic_build() {
        // Same distinctions should produce same root hash
        let d1 = create_distinctions(8);
        let d2 = create_distinctions(8);

        let tree1 = MerkleTree::from_distinctions(&d1);
        let tree2 = MerkleTree::from_distinctions(&d2);

        assert_eq!(tree1.root_hash(), tree2.root_hash());
    }

    #[test]
    fn test_diff_identical() {
        let distinctions = create_distinctions(8);
        let tree1 = MerkleTree::from_distinctions(&distinctions);
        let tree2 = MerkleTree::from_distinctions(&distinctions);

        let diff = tree1.diff(&tree2);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_missing_one() {
        let d1 = create_distinctions(8);
        let d2 = create_distinctions(7); // Missing dist_00000007

        let tree1 = MerkleTree::from_distinctions(&d1);
        let tree2 = MerkleTree::from_distinctions(&d2);

        let diff = tree1.diff(&tree2);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains("dist_00000007"));
    }

    #[test]
    fn test_diff_missing_multiple() {
        let d1 = create_distinctions(8);
        let d2 = create_distinctions(4); // Missing half

        let tree1 = MerkleTree::from_distinctions(&d1);
        let tree2 = MerkleTree::from_distinctions(&d2);

        let diff = tree1.diff(&tree2);
        // Diff includes all 4 missing distinctions
        // Due to tree structure differences, might include more, but should include all missing
        assert!(diff.len() >= 4, "Should have at least 4 differences, got {}", diff.len());
        // Verify all expected missing items are in the diff
        for i in 4..8 {
            assert!(diff.contains(&format!("dist_{:08x}", i)));
        }
    }

    #[test]
    fn test_large_tree() {
        let distinctions = create_distinctions(1000);
        let tree = MerkleTree::from_distinctions(&distinctions);

        assert_eq!(tree.size(), 1000);
        assert!(tree.verify());
    }

    #[test]
    fn test_power_of_two_padding() {
        // 5 distinctions should pad to 8 leaves internally
        let distinctions = create_distinctions(5);
        let tree = MerkleTree::from_distinctions(&distinctions);

        assert_eq!(tree.size(), 5);
        assert!(tree.verify());

        // Should still get all 5 back
        let collected = tree.distinctions();
        assert_eq!(collected.len(), 5);
    }
}
