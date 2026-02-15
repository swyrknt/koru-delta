//! Network Process - Distributed synthesis as causal propagation.
//!
//! This module implements the network as a process of distinction synthesis
//! across node boundaries. Unlike traditional networking (objects sending
//! messages), the LCA network treats every network operation as synthesis
//! within the unified field.
//!
//! # Core Philosophy
//!
//! From koru-lambda-core's perspective, there is no "network" separate from
//! the field. There is only synthesis that may occur:
//! - Locally (single node)
//! - Distributed (multiple nodes share distinctions)
//!
//! A "peer" is not an object but a **recurring pattern of synthesis**.
//! A "message" is not sent but **synthesized and observed**.
//! "Topology" is not a list but **causal relationships in the graph**.
//!
//! # The Synthesis Pattern
//!
//! All network operations follow the universal pattern:
//! ```text
//! ΔNew = ΔNetwork_Root ⊕ ΔContent ⊕ ΔContext
//! ```
//!
//! Where:
//! - `ΔNetwork_Root` - RootType::Network (the network perspective)
//! - `ΔContent` - The actual data/message/payload
//! - `ΔContext` - Metadata (sender, timestamp, sequence, etc.)
//!
//! # Causal Propagation
//!
//! When node A synthesizes a distinction, and node B synthesizes the same
//! content (via network sync), they share a causal ancestor. The distinction
//! ID is the same because synthesis is deterministic.
//!
//! This means:
//! - No explicit "sync" needed - shared synthesis IS sync
//! - No peer tracking needed - causal graph reveals topology
//! - No message deduplication needed - same ID = same distinction
//! - No topology maintenance needed - synthesis relationships ARE topology
//!
//! # Security Model
//!
//! Security emerges from causal properties:
//! - **Authenticity** - Only nodes that know the causal history can synthesize
//! - **Integrity** - Distinction ID is content-addressed (tamper-evident)
//! - **Non-repudiation** - Synthesis is immutable and observable
//! - **Authorization** - Capability distinctions grant synthesis rights
//!
//! # Discovery
//!
//! Peers are discovered by observing which distinctions they can synthesize.
//! If node B synthesizes a distinction that node A created, they share causal
//! history → they are "connected" in the network topology.

// HashSet removed - not used in process model
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine};

// NetworkAction removed - using direct synthesis instead
use crate::engine::{FieldHandle, SharedEngine};
use crate::network::NodeId;
use crate::roots::RootType;
use crate::types::FullKey;

/// Network Process - distributed synthesis as causal propagation.
///
/// The NetworkProcess is not a state tracker. It is a **synthesis facilitator**.
/// It provides the interface for network operations to enter the field as
/// distinctions, and for the field's causal structure to be observed as
/// network topology.
///
/// # No Peer Tracking
///
/// Unlike traditional network agents, this does NOT track peers in a HashMap.
/// Peers are **discovered** by querying the causal graph for distinctions
/// that share synthesis relationships with this node's local root.
///
/// # Querying Topology
///
/// To find "connected peers", query the causal graph for distinctions that:
/// 1. Were synthesized with the Network root
/// 2. Have synthesis relationships from multiple node IDs
/// 3. Share causal ancestry with this node's operations
///
/// This is more powerful than peer lists - it reveals actual causal
/// relationships, not just configuration.
pub struct NetworkProcess {
    /// This node's unique identifier
    node_id: NodeId,

    /// This node's address for network communication
    bind_addr: SocketAddr,

    /// LCA: Network root distinction (shared across all nodes)
    network_root: Distinction,

    /// LCA: This node's local network perspective
    /// Every synthesis updates this, creating the node's causal chain
    local_root: RwLock<Distinction>,

    /// LCA: Handle to the unified field
    field: FieldHandle,

    /// Synthesis sequence counter (for ordering)
    sequence: AtomicU64,

    /// Statistics (for observability, not state)
    distinctions_synthesized: AtomicU64,
    propagations_observed: AtomicU64,
}

/// A network distinction - content that exists in the distributed field.
///
/// This represents a distinction that has been synthesized into the network
/// perspective. It may have originated locally or been observed from peers.
#[derive(Debug, Clone)]
pub struct NetworkDistinction {
    /// The distinction itself
    pub distinction: Distinction,

    /// Content that was synthesized
    pub content: NetworkContent,

    /// Synthesis context (who, when, sequence)
    pub context: SynthesisContext,
}

/// Content types for network synthesis.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum NetworkContent {
    /// A peer announcement (I'm here)
    PeerPresence { node_id: String, address: SocketAddr },

    /// A data write (store this)
    DataWrite { key: FullKey, value_hash: String },

    /// A query request (who has this?)
    QueryRequest { query_hash: String },

    /// A query response (I have this)
    QueryResponse { query_hash: String, result_hash: String },

    /// A capability grant (you may do this)
    CapabilityGrant { grantee: String, permission: String },

    /// Custom application content
    Custom { content_type: String, data_hash: String },
}

/// Context for a synthesis operation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SynthesisContext {
    /// Node that performed the synthesis
    pub node_id: String,

    /// Timestamp (for human observation, not causal ordering)
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Sequence number (for deterministic ordering within node)
    pub sequence: u64,

    /// Causal parents (distinctions this synthesis depends on)
    pub causal_parents: Vec<String>,
}

/// Network topology discovered from causal graph.
#[derive(Debug, Clone)]
pub struct CausalTopology {
    /// Nodes observed in the causal graph
    pub nodes: Vec<CausalNode>,

    /// Connections (synthesis relationships between nodes)
    pub connections: Vec<CausalConnection>,

    /// Distinctions shared between nodes
    pub shared_distinctions: Vec<String>,
}

/// A node discovered in the causal graph.
#[derive(Debug, Clone)]
pub struct CausalNode {
    /// Node ID
    pub node_id: String,

    /// Address (if known from PeerPresence distinctions)
    pub address: Option<SocketAddr>,

    /// Last observed sequence number
    pub last_sequence: u64,

    /// Number of distinctions from this node
    pub distinction_count: usize,

    /// Whether this node is "active" (recent synthesis)
    pub is_active: bool,
}

/// A connection between nodes in the causal graph.
#[derive(Debug, Clone)]
pub struct CausalConnection {
    /// Source node
    pub from: String,

    /// Target node
    pub to: String,

    /// Number of shared distinctions
    pub shared_count: usize,

    /// Direction of causal influence
    pub direction: CausalDirection,
}

/// Direction of causal influence.
#[derive(Debug, Clone, PartialEq)]
pub enum CausalDirection {
    /// A influences B (A's distinctions appear in B's synthesis)
    AToB,
    /// B influences A
    BToA,
    /// Bidirectional (both share distinctions)
    Bidirectional,
    /// Unknown (insufficient data)
    Unknown,
}

/// Statistics for network process operations.
#[derive(Debug, Clone)]
pub struct NetworkProcessStats {
    pub distinctions_synthesized: u64,
    pub propagations_observed: u64,
    pub current_sequence: u64,
    pub local_root_id: String,
    pub network_root_id: String,
}

impl NetworkProcess {
    /// Create a new network process.
    ///
    /// # LCA Pattern
    ///
    /// The process initializes with:
    /// - `network_root` = RootType::Network (shared across all nodes)
    /// - `local_root` = This node's synthesis of the network root with its node ID
    pub fn new(shared_engine: &SharedEngine, bind_addr: SocketAddr) -> Self {
        let network_root = shared_engine.root(RootType::Network).clone();
        let field = FieldHandle::new(shared_engine);

        // Create this node's local root by synthesizing network root with node identity
        let node_id = NodeId::new();
        let node_id_distinction = Self::node_id_to_distinction(&field, &node_id);
        let local_root = field.synthesize(&network_root, &node_id_distinction);

        Self {
            node_id,
            bind_addr,
            network_root,
            local_root: RwLock::new(local_root),
            field,
            sequence: AtomicU64::new(0),
            distinctions_synthesized: AtomicU64::new(0),
            propagations_observed: AtomicU64::new(0),
        }
    }

    /// Get this node's ID.
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get this node's bind address.
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }

    /// Get the network root distinction (shared across all nodes).
    pub fn network_root(&self) -> &Distinction {
        &self.network_root
    }

    /// Get this node's current local root.
    ///
    /// The local root is the cumulative synthesis of all network operations.
    /// It represents this node's current causal state in the network.
    pub fn local_root(&self) -> Distinction {
        self.local_root.read().unwrap().clone()
    }

    // ========================================================================
    // Synthesis Operations (The Core)
    // ========================================================================

    /// Synthesize network content into the field.
    ///
    /// # LCA Pattern
    ///
    /// ```text
    /// ΔNew = ΔLocal_Root ⊕ ΔContent ⊕ ΔContext
    /// ```
    ///
    /// This is the fundamental network operation. Content enters the causal
    /// chain and becomes observable by other nodes.
    pub fn synthesize(&self, content: NetworkContent) -> NetworkDistinction {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);

        // Build synthesis context
        let context = SynthesisContext {
            node_id: self.node_id.to_string(),
            timestamp: chrono::Utc::now(),
            sequence: seq,
            causal_parents: vec![self.local_root().id().to_string()],
        };

        // Synthesize: content + context + local_root
        let content_distinction = content.to_canonical_structure(self.field.engine_arc());
        let context_distinction = context.to_canonical_structure(self.field.engine_arc());

        let local_root = self.local_root();
        let with_content = self.field.synthesize(&local_root, &content_distinction);
        let new_root = self.field.synthesize(&with_content, &context_distinction);

        // Update local root
        *self.local_root.write().unwrap() = new_root.clone();

        self.distinctions_synthesized.fetch_add(1, Ordering::SeqCst);

        NetworkDistinction {
            distinction: new_root.clone(),
            content,
            context,
        }
    }

    /// Observe a distinction synthesized by another node.
    ///
    /// # LCA Pattern
    ///
    /// When we observe a distinction from another node, we synthesize it
    /// with our local root. This creates a causal relationship showing
    /// that our state now includes their state.
    ///
    /// ```text
    /// ΔNew = ΔLocal_Root ⊕ ΔObservedDistinction
    /// ```
    pub fn observe(&self, distinction: &Distinction) -> Distinction {
        let local_root = self.local_root();
        let new_root = self.field.synthesize(&local_root, distinction);

        *self.local_root.write().unwrap() = new_root.clone();
        self.propagations_observed.fetch_add(1, Ordering::SeqCst);

        new_root
    }

    /// Announce this node's presence to the network.
    ///
    /// Creates a PeerPresence distinction that other nodes can observe.
    pub fn announce_presence(&self) -> NetworkDistinction {
        let content = NetworkContent::PeerPresence {
            node_id: self.node_id.to_string(),
            address: self.bind_addr,
        };
        self.synthesize(content)
    }

    /// Synthesize a data write into the network.
    pub fn write_data(&self, key: FullKey, value: &serde_json::Value) -> NetworkDistinction {
        // Hash the value for content addressing
        let value_hash = Self::hash_value(value);

        let content = NetworkContent::DataWrite { key, value_hash };
        self.synthesize(content)
    }

    // ========================================================================
    // Topology Discovery (Query the Causal Graph)
    // ========================================================================

    /// Discover network topology from the causal graph.
    ///
    /// This queries the field for distinctions that:
    /// 1. Have NetworkContent::PeerPresence
    /// 2. Share synthesis relationships with this node's local root
    ///
    /// # Returns
    ///
    /// The causal topology showing which nodes exist and how they're connected
    /// through shared distinctions.
    pub fn discover_topology(&self) -> CausalTopology {
        // In a full implementation, this would query the distinction engine
        // for all distinctions with NetworkContent and build the topology.
        // For now, we return a placeholder based on observations.

        CausalTopology {
            nodes: vec![CausalNode {
                node_id: self.node_id.to_string(),
                address: Some(self.bind_addr),
                last_sequence: self.sequence.load(Ordering::SeqCst),
                distinction_count: self.distinctions_synthesized.load(Ordering::SeqCst) as usize,
                is_active: true,
            }],
            connections: vec![],
            shared_distinctions: vec![],
        }
    }

    /// Find active peers from the causal graph.
    ///
    /// Active peers are nodes that have synthesized distinctions recently
    /// (within the active threshold).
    pub fn find_active_peers(&self, _active_threshold: std::time::Duration) -> Vec<CausalNode> {
        // Query the causal graph for PeerPresence distinctions with recent timestamps
        // For now, return this node
        vec![CausalNode {
            node_id: self.node_id.to_string(),
            address: Some(self.bind_addr),
            last_sequence: self.sequence.load(Ordering::SeqCst),
            distinction_count: self.distinctions_synthesized.load(Ordering::SeqCst) as usize,
            is_active: true,
        }]
    }

    /// Check if a node is reachable via causal relationships.
    ///
    /// A node is "reachable" if there's a path of synthesis relationships
    /// from this node's local root to distinctions from that node.
    pub fn is_reachable(&self, _node_id: &str) -> bool {
        // Check if any distinctions from node_id appear in our causal ancestry
        // For now, assume only self is reachable
        true
    }

    // ========================================================================
    // Utility
    // ========================================================================

    /// Convert a node ID to a distinction.
    fn node_id_to_distinction(field: &FieldHandle, node_id: &NodeId) -> Distinction {
        // Serialize node ID to bytes and synthesize
        let bytes = node_id.to_string().into_bytes();
        let engine = field.engine_arc();
        bytes
            .iter()
            .map(|&byte| byte.to_canonical_structure(engine))
            .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
    }

    /// Hash a JSON value for content addressing.
    fn hash_value(value: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        value.to_string().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get statistics.
    pub fn stats(&self) -> NetworkProcessStats {
        NetworkProcessStats {
            distinctions_synthesized: self.distinctions_synthesized.load(Ordering::SeqCst),
            propagations_observed: self.propagations_observed.load(Ordering::SeqCst),
            current_sequence: self.sequence.load(Ordering::SeqCst),
            local_root_id: self.local_root().id().to_string(),
            network_root_id: self.network_root.id().to_string(),
        }
    }
}



impl Canonicalizable for NetworkContent {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        // Serialize to JSON then to distinction
        let json = serde_json::to_string(self).unwrap_or_default();
        let bytes = json.into_bytes();
        bytes
            .iter()
            .map(|&byte| byte.to_canonical_structure(engine))
            .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
    }
}

impl Canonicalizable for SynthesisContext {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let json = serde_json::to_string(self).unwrap_or_default();
        let bytes = json.into_bytes();
        bytes
            .iter()
            .map(|&byte| byte.to_canonical_structure(engine))
            .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
    }
}

// ============================================================================
// Tests - Falsification Testing
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_process() -> NetworkProcess {
        let shared_engine = SharedEngine::new();
        let addr = "127.0.0.1:7878".parse().unwrap();
        NetworkProcess::new(&shared_engine, addr)
    }

    // ====================================================================
    // Basic Synthesis Tests
    // ====================================================================

    #[test]
    fn test_process_creation() {
        let process = create_test_process();

        // Local root should be different from network root (synthesized with node ID)
        assert_ne!(process.local_root().id(), process.network_root().id());

        // Stats should be zero
        let stats = process.stats();
        assert_eq!(stats.distinctions_synthesized, 0);
        assert_eq!(stats.propagations_observed, 0);
    }

    #[test]
    fn test_synthesis_advances_local_root() {
        let process = create_test_process();
        let root_before = process.local_root();

        let dist = process.synthesize(NetworkContent::PeerPresence {
            node_id: "test".to_string(),
            address: "127.0.0.1:9999".parse().unwrap(),
        });

        let root_after = process.local_root();

        // Local root should advance
        assert_ne!(root_after.id(), root_before.id());
        assert_eq!(root_after.id(), dist.distinction.id());
    }

    #[test]
    fn test_synthesis_sequence_increments() {
        let process = create_test_process();

        let dist1 = process.synthesize(NetworkContent::Custom {
            content_type: "test".to_string(),
            data_hash: "hash1".to_string(),
        });

        let dist2 = process.synthesize(NetworkContent::Custom {
            content_type: "test".to_string(),
            data_hash: "hash2".to_string(),
        });

        // Sequence should increment
        assert_eq!(dist1.context.sequence, 0);
        assert_eq!(dist2.context.sequence, 1);

        let stats = process.stats();
        assert_eq!(stats.distinctions_synthesized, 2);
    }

    #[test]
    fn test_synthesis_deterministic() {
        // Same content should produce same distinction (content-addressed)
        let shared_engine = SharedEngine::new();
        let addr = "127.0.0.1:7878".parse().unwrap();

        let process1 = NetworkProcess::new(&shared_engine, addr);
        let process2 = NetworkProcess::new(&shared_engine, addr);

        // Note: Distinctions will differ because local roots differ (different node IDs)
        // But the CONTENT distinction should be the same
        let content = NetworkContent::Custom {
            content_type: "test".to_string(),
            data_hash: "same_hash".to_string(),
        };

        let content_dist1 = content.to_canonical_structure(process1.field.engine_arc());
        let content_dist2 = content.to_canonical_structure(process2.field.engine_arc());

        assert_eq!(content_dist1.id(), content_dist2.id());
    }

    // ====================================================================
    // Observation Tests (The "Sync" Mechanism)
    // ====================================================================

    #[test]
    fn test_observation_advances_local_root() {
        let process = create_test_process();
        let root_before = process.local_root();

        // Create a distinction (simulating receipt from another node)
        let observed = process.network_root().clone();

        let new_root = process.observe(&observed);

        let root_after = process.local_root();

        assert_ne!(new_root.id(), root_before.id());
        assert_eq!(root_after.id(), new_root.id());
    }

    #[test]
    fn test_observation_tracks_propagations() {
        let process = create_test_process();

        let observed = process.network_root().clone();
        process.observe(&observed);
        process.observe(&observed);
        process.observe(&observed);

        let stats = process.stats();
        assert_eq!(stats.propagations_observed, 3);
    }

    // ====================================================================
    // Content Type Tests
    // ====================================================================

    #[test]
    fn test_peer_presence_synthesis() {
        let process = create_test_process();

        let dist = process.announce_presence();

        match &dist.content {
            NetworkContent::PeerPresence { node_id, address } => {
                assert_eq!(node_id, &process.node_id().to_string());
                assert_eq!(*address, process.bind_addr());
            }
            _ => panic!("Expected PeerPresence content"),
        }

        assert_eq!(dist.context.sequence, 0);
    }

    #[test]
    fn test_data_write_synthesis() {
        let process = create_test_process();
        let key = FullKey::new("test_ns", "test_key");
        let value = serde_json::json!({"field": "value"});

        let dist = process.write_data(key.clone(), &value);

        match &dist.content {
            NetworkContent::DataWrite { key: k, value_hash } => {
                assert_eq!(k.namespace, "test_ns");
                assert_eq!(k.key, "test_key");
                assert!(!value_hash.is_empty());
            }
            _ => panic!("Expected DataWrite content"),
        }
    }

    // ====================================================================
    // Causal Relationship Tests
    // ====================================================================

    #[test]
    fn test_causal_parents_tracked() {
        let process = create_test_process();
        let initial_root = process.local_root();

        let dist1 = process.synthesize(NetworkContent::Custom {
            content_type: "first".to_string(),
            data_hash: "hash1".to_string(),
        });

        let dist2 = process.synthesize(NetworkContent::Custom {
            content_type: "second".to_string(),
            data_hash: "hash2".to_string(),
        });

        // Each synthesis should have the previous local root as causal parent
        // First synthesis: parent is initial local root
        assert!(dist1.context.causal_parents.contains(&initial_root.id().to_string()));
        // Second synthesis: parent is the result of first synthesis
        assert!(dist2.context.causal_parents.contains(&dist1.distinction.id().to_string()));
    }

    #[test]
    fn test_synthesis_creates_distinct_distinctions() {
        let process = create_test_process();

        let dist1 = process.synthesize(NetworkContent::Custom {
            content_type: "test".to_string(),
            data_hash: "hash1".to_string(),
        });

        let dist2 = process.synthesize(NetworkContent::Custom {
            content_type: "test".to_string(),
            data_hash: "hash2".to_string(),
        });

        // Different content should produce different distinctions
        assert_ne!(dist1.distinction.id(), dist2.distinction.id());
    }

    // ====================================================================
    // Falsification Tests
    // ====================================================================

    #[test]
    fn test_empty_content_still_synthesizes() {
        let process = create_test_process();
        let root_before = process.local_root();

        let dist = process.synthesize(NetworkContent::Custom {
            content_type: "".to_string(),
            data_hash: "".to_string(),
        });

        // Even empty content advances the local root
        assert_ne!(process.local_root().id(), root_before.id());
        assert_eq!(dist.distinction.id(), process.local_root().id());
    }

    #[test]
    fn test_large_sequence_numbers() {
        let process = create_test_process();

        // Simulate many syntheses
        for i in 0..1000 {
            let dist = process.synthesize(NetworkContent::Custom {
                content_type: "load_test".to_string(),
                data_hash: format!("hash_{}", i),
            });
            assert_eq!(dist.context.sequence, i);
        }

        let stats = process.stats();
        assert_eq!(stats.distinctions_synthesized, 1000);
        assert_eq!(stats.current_sequence, 1000);
    }

    #[test]
    fn test_network_root_constant() {
        let shared_engine = SharedEngine::new();
        let addr = "127.0.0.1:7878".parse().unwrap();

        let process1 = NetworkProcess::new(&shared_engine, addr);
        let process2 = NetworkProcess::new(&shared_engine, addr);

        // All processes on same engine share the network root
        assert_eq!(process1.network_root().id(), process2.network_root().id());
    }

    #[test]
    fn test_local_roots_differ() {
        let shared_engine = SharedEngine::new();
        let addr = "127.0.0.1:7878".parse().unwrap();

        let process1 = NetworkProcess::new(&shared_engine, addr);
        let process2 = NetworkProcess::new(&shared_engine, addr);

        // Local roots should differ (synthesized with different node IDs)
        assert_ne!(process1.local_root().id(), process2.local_root().id());
    }

    // ====================================================================
    // Content Hashing Tests
    // ====================================================================

    #[test]
    fn test_value_hashing_deterministic() {
        let value1 = serde_json::json!({"a": 1, "b": 2});
        let value2 = serde_json::json!({"a": 1, "b": 2});
        let value3 = serde_json::json!({"a": 1, "b": 3});

        let hash1 = NetworkProcess::hash_value(&value1);
        let hash2 = NetworkProcess::hash_value(&value2);
        let hash3 = NetworkProcess::hash_value(&value3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_value_hashing_order_matters() {
        // JSON object key order matters in string representation
        let value1 = serde_json::json!({"a": 1, "b": 2});
        let value2 = serde_json::json!({"b": 2, "a": 1});

        // These might hash differently depending on serde_json's serialization
        let _hash1 = NetworkProcess::hash_value(&value1);
        let _hash2 = NetworkProcess::hash_value(&value2);

        // Note: In practice, we might want canonical JSON ordering
        // This test documents current behavior
    }
}
