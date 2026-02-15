//! Network Agent - Distributed awareness via LCA synthesis.
//!
//! The NetworkAgent provides the LCA architecture for network operations,
//! bridging the async ClusterNode with the synchronous distinction field.
//!
//! # LCA Architecture
//!
//! All network operations follow the synthesis pattern:
//! ```text
//! ΔNew = ΔLocal_Root ⊕ ΔNetwork_Action
//! ```
//!
//! - Local root: RootType::Network (canonical root for distributed awareness)
//! - Peers: Synthesis of all known peer distinctions
//!
//! # Bridge Pattern
//!
//! The NetworkAgent uses a channel-based bridge to connect with ClusterNode:
//! - ClusterNode emits NetworkEvents via channel (async side)
//! - NetworkAgent receives events and synthesizes them (sync LCA side)
//!
//! This maintains separation: ClusterNode handles runtime/async concerns,
//! NetworkAgent handles causal/distinction concerns.
//!
//! # Peer Distinctions
//!
//! Each peer becomes a distinction in the field:
//! - Peer join → Synthesize peer distinction with local root
//! - Peer update → Synthesize update action
//! - Peer leave → Synthesize tombstone distinction
//!
//! The `peers` field is the cumulative synthesis of all peer distinctions,
//! representing the network topology as a causal structure.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use koru_lambda_core::{Canonicalizable, Distinction};

use crate::actions::NetworkAction;
use crate::engine::{FieldHandle, SharedEngine};
use crate::network::{NodeId, PeerInfo, PeerStatus};
use crate::roots::RootType;

/// Network Agent - distributed awareness via LCA synthesis.
///
/// The NetworkAgent maintains the causal representation of the network:
/// - Which peers exist (as distinctions)
/// - Network topology (as synthesis relationships)
/// - Distributed state (as causal chains)
///
/// It receives events from ClusterNode via a channel and synthesizes
/// them into the unified field.
pub struct NetworkAgent {
    /// LCA: Local root distinction (Root: NETWORK)
    local_root: RwLock<Distinction>,

    /// LCA: Synthesis of all peer perspectives
    peers: RwLock<Distinction>,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// This node's ID
    node_id: NodeId,

    /// Map of peer IDs to their distinctions
    peer_distinctions: RwLock<HashMap<String, Distinction>>,

    /// Channel receiver for network events from ClusterNode
    event_rx: RwLock<std::sync::mpsc::Receiver<NetworkEvent>>,

    /// Statistics
    peers_joined: AtomicU64,
    peers_left: AtomicU64,
    syncs_completed: AtomicU64,
    messages_received: AtomicU64,
}

/// Events emitted by ClusterNode to be synthesized into the field.
///
/// These represent network occurrences that will become distinctions
/// within KoruDelta's causal history.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A peer joined the cluster
    PeerJoined {
        /// Peer information
        peer: PeerInfo,
    },

    /// A peer left or was removed
    PeerLeft {
        /// Peer node ID
        node_id: String,
    },

    /// Peer status changed (e.g., Healthy → Unreachable)
    PeerStatusChanged {
        /// Peer node ID
        node_id: String,
        /// New status
        status: PeerStatus,
    },

    /// Synchronization completed with a peer
    SyncCompleted {
        /// Peer node ID
        peer_id: String,
        /// Number of updates applied
        updates_count: usize,
    },

    /// Received a message from a peer
    MessageReceived {
        /// Sender node ID
        from: String,
        /// Message type
        message_type: String,
    },

    /// Gossip protocol exchanged state
    GossipExchanged {
        /// Peer node ID
        peer_id: String,
        /// Number of peers known to them
        their_peer_count: usize,
    },

    /// This node joined a cluster
    SelfJoined {
        /// Address of the peer we joined through
        via_peer: SocketAddr,
    },
}

/// Statistics for network operations.
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub peers_joined: u64,
    pub peers_left: u64,
    pub syncs_completed: u64,
    pub messages_received: u64,
    pub current_peers: u64,
}

impl NetworkAgent {
    /// Create a new network agent.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Network (from shared field roots)
    /// - `peers` = Initial synthesis (just the local root)
    /// - `field` = Handle to the unified distinction engine
    ///
    /// # Arguments
    /// * `shared_engine` - The shared distinction engine
    /// * `node_id` - This node's unique identifier
    /// * `event_rx` - Channel receiver for network events from ClusterNode
    pub fn new(
        shared_engine: &SharedEngine,
        node_id: NodeId,
        event_rx: std::sync::mpsc::Receiver<NetworkEvent>,
    ) -> Self {
        let local_root = shared_engine.root(RootType::Network).clone();
        let peers = local_root.clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            local_root: RwLock::new(local_root),
            peers: RwLock::new(peers),
            field,
            node_id,
            peer_distinctions: RwLock::new(HashMap::new()),
            event_rx: RwLock::new(event_rx),
            peers_joined: AtomicU64::new(0),
            peers_left: AtomicU64::new(0),
            syncs_completed: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
        }
    }

    /// Get this node's ID.
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the local root distinction.
    pub fn local_root(&self) -> Distinction {
        self.local_root.read().unwrap().clone()
    }

    /// Get the peers distinction (synthesis of all peers).
    pub fn peers_distinction(&self) -> Distinction {
        self.peers.read().unwrap().clone()
    }

    /// Get a peer's distinction by ID.
    pub fn get_peer_distinction(&self, peer_id: &str) -> Option<Distinction> {
        self.peer_distinctions.read().unwrap().get(peer_id).cloned()
    }

    // ========================================================================
    // Event Processing
    // ========================================================================

    /// Process pending network events.
    ///
    /// This should be called regularly (e.g., in a loop or timer) to
    /// process events from ClusterNode and synthesize them into the field.
    ///
    /// # LCA Pattern
    ///
    /// Each event becomes: `ΔNew = ΔLocal_Root ⊕ ΔNetwork_Action`
    pub fn process_events(&self) -> usize {
        let mut count = 0;
        
        while let Ok(event) = self.event_rx.read().unwrap().try_recv() {
            self.handle_event(event);
            count += 1;
        }
        
        count
    }

    /// Handle a single network event.
    fn handle_event(&self, event: NetworkEvent) {
        match event {
            NetworkEvent::PeerJoined { peer } => {
                self.handle_peer_joined(peer);
            }
            NetworkEvent::PeerLeft { node_id } => {
                self.handle_peer_left(&node_id);
            }
            NetworkEvent::PeerStatusChanged { node_id, status } => {
                self.handle_peer_status_changed(&node_id, status);
            }
            NetworkEvent::SyncCompleted { peer_id, updates_count } => {
                self.handle_sync_completed(&peer_id, updates_count);
            }
            NetworkEvent::MessageReceived { from, message_type } => {
                self.handle_message_received(&from, &message_type);
            }
            NetworkEvent::GossipExchanged { peer_id, their_peer_count } => {
                self.handle_gossip_exchanged(&peer_id, their_peer_count);
            }
            NetworkEvent::SelfJoined { via_peer } => {
                self.handle_self_joined(via_peer);
            }
        }
    }

    /// Handle peer joined event.
    fn handle_peer_joined(&self, peer: PeerInfo) {
        // Synthesize join action
        let action = NetworkAction::Join {
            peer_address: peer.address.to_string(),
        };
        let peer_distinction = self.synthesize_action_internal(action);

        // Store peer distinction
        self.peer_distinctions.write().unwrap().insert(
            peer.node_id.to_string(),
            peer_distinction.clone(),
        );

        // Synthesize into peers distinction
        self.synthesize_peer(peer_distinction);

        self.peers_joined.fetch_add(1, Ordering::SeqCst);
    }

    /// Handle peer left event.
    fn handle_peer_left(&self, node_id: &str) {
        // Remove from peer distinctions
        if self.peer_distinctions.write().unwrap().remove(node_id).is_some() {
            // Note: In a full implementation, we'd synthesize a tombstone
            // For now, we just remove from the active set
            self.peers_left.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Handle peer status changed event.
    fn handle_peer_status_changed(&self, node_id: &str, status: PeerStatus) {
        // Synthesize status change as a reconcile action (represents state change)
        let action = NetworkAction::Reconcile {
            difference_ids: vec![format!("{}:{:?}", node_id, status)],
        };
        let _ = self.synthesize_action_internal(action);
    }

    /// Handle sync completed event.
    fn handle_sync_completed(&self, peer_id: &str, updates_count: usize) {
        let action = NetworkAction::Synchronize {
            peer_id: peer_id.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        if updates_count > 0 {
            self.syncs_completed.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Handle message received event.
    fn handle_message_received(&self, from: &str, message_type: &str) {
        let action = NetworkAction::Broadcast {
            message_json: serde_json::json!({
                "from": from,
                "type": message_type,
            }),
        };
        let _ = self.synthesize_action_internal(action);

        self.messages_received.fetch_add(1, Ordering::SeqCst);
    }

    /// Handle gossip exchanged event.
    fn handle_gossip_exchanged(&self, peer_id: &str, their_peer_count: usize) {
        let action = NetworkAction::Gossip {
            state_json: serde_json::json!({
                "peer_id": peer_id,
                "their_peer_count": their_peer_count,
            }),
        };
        let _ = self.synthesize_action_internal(action);
    }

    /// Handle self joined event.
    fn handle_self_joined(&self, via_peer: SocketAddr) {
        let action = NetworkAction::Join {
            peer_address: via_peer.to_string(),
        };
        let _ = self.synthesize_action_internal(action);
    }

    // ========================================================================
    // LCA Synthesis
    // ========================================================================

    /// Synthesize a peer distinction into the peers aggregate.
    ///
    /// # LCA Pattern
    ///
    /// `peers_new = peers_old ⊕ peer_distinction`
    fn synthesize_peer(&self, peer_distinction: Distinction) -> Distinction {
        let engine = self.field.engine_arc();
        let peers = self.peers.read().unwrap().clone();
        let new_peers = engine.synthesize(&peers, &peer_distinction);
        *self.peers.write().unwrap() = new_peers.clone();
        new_peers
    }

    /// Internal synthesis helper for network actions.
    ///
    /// # LCA Pattern
    ///
    /// `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: NetworkAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        let local_root = self.local_root.read().unwrap().clone();
        let new_root = engine.synthesize(&local_root, &action_distinction);
        *self.local_root.write().unwrap() = new_root.clone();
        new_root
    }

    /// Synthesize a network action (public API for direct synthesis).
    ///
    /// # LCA Pattern
    ///
    /// `ΔNew = ΔLocal_Root ⊕ ΔAction`
    pub fn synthesize_action(&self, action: NetworkAction) -> Distinction {
        self.synthesize_action_internal(action)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get network statistics.
    pub fn stats(&self) -> NetworkStats {
        NetworkStats {
            peers_joined: self.peers_joined.load(Ordering::SeqCst),
            peers_left: self.peers_left.load(Ordering::SeqCst),
            syncs_completed: self.syncs_completed.load(Ordering::SeqCst),
            messages_received: self.messages_received.load(Ordering::SeqCst),
            current_peers: self.peer_distinctions.read().unwrap().len() as u64,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn create_test_agent() -> (NetworkAgent, std::sync::mpsc::Sender<NetworkEvent>) {
        let shared_engine = SharedEngine::new();
        let node_id = NodeId::new();
        let (tx, rx) = channel();
        let agent = NetworkAgent::new(&shared_engine, node_id, rx);
        (agent, tx)
    }

    #[test]
    fn test_network_agent_creation() {
        let (agent, _tx) = create_test_agent();

        assert!(!agent.local_root().id().is_empty());
        assert!(!agent.peers_distinction().id().is_empty());
        assert_eq!(agent.stats().current_peers, 0);
    }

    #[test]
    fn test_peer_joined_event() {
        let (agent, tx) = create_test_agent();
        let root_before = agent.local_root();

        let peer = PeerInfo {
            node_id: NodeId::new(),
            address: "127.0.0.1:8080".parse().unwrap(),
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            status: PeerStatus::Healthy,
        };

        tx.send(NetworkEvent::PeerJoined { peer: peer.clone() }).unwrap();
        agent.process_events();

        // Verify synthesis happened
        let root_after = agent.local_root();
        assert_ne!(root_after.id(), root_before.id());

        // Verify peer is tracked
        assert_eq!(agent.stats().peers_joined, 1);
        assert!(agent.get_peer_distinction(&peer.node_id.to_string()).is_some());
    }

    #[test]
    fn test_peer_left_event() {
        let (agent, tx) = create_test_agent();

        // First join a peer
        let peer = PeerInfo {
            node_id: NodeId::new(),
            address: "127.0.0.1:8080".parse().unwrap(),
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            status: PeerStatus::Healthy,
        };

        tx.send(NetworkEvent::PeerJoined { peer: peer.clone() }).unwrap();
        agent.process_events();
        assert_eq!(agent.stats().current_peers, 1);

        // Then remove them
        tx.send(NetworkEvent::PeerLeft {
            node_id: peer.node_id.to_string(),
        })
        .unwrap();
        agent.process_events();

        assert_eq!(agent.stats().peers_left, 1);
        assert_eq!(agent.stats().current_peers, 0);
        assert!(agent.get_peer_distinction(&peer.node_id.to_string()).is_none());
    }

    #[test]
    fn test_sync_completed_event() {
        let (agent, tx) = create_test_agent();
        let root_before = agent.local_root();

        tx.send(NetworkEvent::SyncCompleted {
            peer_id: "peer_123".to_string(),
            updates_count: 5,
        })
        .unwrap();
        agent.process_events();

        let root_after = agent.local_root();
        assert_ne!(root_after.id(), root_before.id());
        assert_eq!(agent.stats().syncs_completed, 1);
    }

    #[test]
    fn test_message_received_event() {
        let (agent, tx) = create_test_agent();

        tx.send(NetworkEvent::MessageReceived {
            from: "peer_123".to_string(),
            message_type: "WriteEvent".to_string(),
        })
        .unwrap();
        agent.process_events();

        assert_eq!(agent.stats().messages_received, 1);
    }

    #[test]
    fn test_gossip_exchanged_event() {
        let (agent, tx) = create_test_agent();

        tx.send(NetworkEvent::GossipExchanged {
            peer_id: "peer_123".to_string(),
            their_peer_count: 3,
        })
        .unwrap();
        agent.process_events();

        // Gossip synthesizes but stats are tracked separately
        let root = agent.local_root();
        assert!(!root.id().is_empty());
    }

    #[test]
    fn test_self_joined_event() {
        let (agent, tx) = create_test_agent();
        let root_before = agent.local_root();

        tx.send(NetworkEvent::SelfJoined {
            via_peer: "192.168.1.1:7878".parse().unwrap(),
        })
        .unwrap();
        agent.process_events();

        let root_after = agent.local_root();
        assert_ne!(root_after.id(), root_before.id());
    }

    #[test]
    fn test_process_multiple_events() {
        let (agent, tx) = create_test_agent();

        // Send multiple events
        for i in 0..5 {
            let peer = PeerInfo {
                node_id: NodeId::new(),
                address: format!("127.0.0.1:{}", 8080 + i).parse().unwrap(),
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                status: PeerStatus::Healthy,
            };
            tx.send(NetworkEvent::PeerJoined { peer }).unwrap();
        }

        let count = agent.process_events();
        assert_eq!(count, 5);
        assert_eq!(agent.stats().current_peers, 5);
    }

    #[test]
    fn test_synthesize_action() {
        let (agent, _tx) = create_test_agent();
        let root_before = agent.local_root();

        let action = NetworkAction::Join {
            peer_address: "127.0.0.1:9999".to_string(),
        };
        let new_root = agent.synthesize_action(action);

        assert_ne!(new_root.id(), root_before.id());
        assert_eq!(agent.local_root().id(), new_root.id());
    }
}
