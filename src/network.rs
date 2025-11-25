/// Network layer for distributed KoruDelta.
///
/// This module provides the low-level networking primitives for node-to-node
/// communication in a KoruDelta cluster. It handles:
///
/// - TCP connection management
/// - Message serialization/deserialization
/// - Protocol message types
///
/// # Protocol Design
///
/// KoruDelta uses a simple request-response protocol over TCP. Each message
/// is prefixed with a 4-byte length header followed by JSON-encoded payload.
///
/// # Thread Safety
///
/// All network operations are designed to be async and can be used with
/// Tokio's multi-threaded runtime.
use crate::error::{DeltaError, DeltaResult};
use crate::types::{FullKey, VersionedValue};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

/// Default port for KoruDelta cluster communication.
pub const DEFAULT_PORT: u16 = 7878;

/// Maximum message size (16 MB).
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Unique identifier for a node in the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Generate a new random node ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a node ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Information about a peer node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Network address of the peer.
    pub address: SocketAddr,
    /// When this peer was first seen.
    pub first_seen: DateTime<Utc>,
    /// When this peer was last seen.
    pub last_seen: DateTime<Utc>,
    /// Current status of the peer.
    pub status: PeerStatus,
}

impl PeerInfo {
    /// Create new peer info.
    pub fn new(node_id: NodeId, address: SocketAddr) -> Self {
        let now = Utc::now();
        Self {
            node_id,
            address,
            first_seen: now,
            last_seen: now,
            status: PeerStatus::Unknown,
        }
    }

    /// Update the last seen timestamp.
    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }
}

/// Status of a peer node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Status unknown (not yet contacted).
    Unknown,
    /// Peer is healthy and responding.
    Healthy,
    /// Peer is currently synchronizing.
    Syncing,
    /// Peer is unreachable.
    Unreachable,
}

/// Protocol messages for cluster communication.
///
/// These messages form the basis of all cluster communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // ─────────────────────────────────────────────────────────────────────
    // Handshake & Discovery
    // ─────────────────────────────────────────────────────────────────────
    /// Initial handshake when joining a cluster.
    Join {
        node_id: NodeId,
        address: SocketAddr,
    },

    /// Acknowledgment of a join request.
    JoinAck {
        node_id: NodeId,
        peers: Vec<PeerInfo>,
    },

    /// Announce presence to peers (gossip).
    Announce {
        node_id: NodeId,
        address: SocketAddr,
        peers: Vec<PeerInfo>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Health & Status
    // ─────────────────────────────────────────────────────────────────────
    /// Heartbeat ping.
    Ping { node_id: NodeId },

    /// Heartbeat response.
    Pong { node_id: NodeId },

    // ─────────────────────────────────────────────────────────────────────
    // Data Synchronization
    // ─────────────────────────────────────────────────────────────────────
    /// Request a full snapshot of all data.
    SnapshotRequest { node_id: NodeId },

    /// Response with full snapshot data.
    SnapshotResponse {
        node_id: NodeId,
        current_state: Vec<(FullKey, VersionedValue)>,
        history_log: Vec<(FullKey, Vec<VersionedValue>)>,
    },

    /// Broadcast a new write to peers.
    WriteEvent {
        node_id: NodeId,
        key: FullKey,
        value: VersionedValue,
    },

    /// Acknowledge receipt of a write event.
    WriteAck {
        node_id: NodeId,
        key: FullKey,
        version_id: String,
    },

    /// Request sync for specific keys.
    SyncRequest {
        node_id: NodeId,
        /// Keys and their latest known version IDs.
        keys: HashMap<FullKey, Option<String>>,
    },

    /// Response with updated values for requested keys.
    SyncResponse {
        node_id: NodeId,
        /// Updated values since the requested versions.
        updates: Vec<(FullKey, Vec<VersionedValue>)>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Errors
    // ─────────────────────────────────────────────────────────────────────
    /// Error response.
    Error { message: String },
}

impl Message {
    /// Serialize message to bytes.
    pub fn to_bytes(&self) -> DeltaResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(DeltaError::SerializationError)
    }

    /// Deserialize message from bytes.
    pub fn from_bytes(bytes: &[u8]) -> DeltaResult<Self> {
        serde_json::from_slice(bytes).map_err(DeltaError::SerializationError)
    }
}

/// Network connection to a peer.
pub struct Connection {
    stream: TcpStream,
    peer_addr: SocketAddr,
}

impl Connection {
    /// Create a new connection from a TCP stream.
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        Self { stream, peer_addr }
    }

    /// Connect to a peer.
    pub async fn connect(addr: SocketAddr) -> DeltaResult<Self> {
        let stream = TcpStream::connect(addr).await.map_err(|e| {
            DeltaError::StorageError(format!("Failed to connect to {}: {}", addr, e))
        })?;
        Ok(Self::new(stream, addr))
    }

    /// Get the peer address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Send a message to the peer.
    pub async fn send(&mut self, message: &Message) -> DeltaResult<()> {
        let bytes = message.to_bytes()?;

        if bytes.len() > MAX_MESSAGE_SIZE {
            return Err(DeltaError::StorageError(format!(
                "Message too large: {} bytes (max: {})",
                bytes.len(),
                MAX_MESSAGE_SIZE
            )));
        }

        // Write length header (4 bytes, big-endian).
        let len = bytes.len() as u32;
        self.stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| {
                DeltaError::StorageError(format!("Failed to write message length: {}", e))
            })?;

        // Write message body.
        self.stream.write_all(&bytes).await.map_err(|e| {
            DeltaError::StorageError(format!("Failed to write message body: {}", e))
        })?;

        self.stream
            .flush()
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to flush stream: {}", e)))?;

        Ok(())
    }

    /// Receive a message from the peer.
    pub async fn receive(&mut self) -> DeltaResult<Message> {
        // Read length header (4 bytes, big-endian).
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await.map_err(|e| {
            DeltaError::StorageError(format!("Failed to read message length: {}", e))
        })?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > MAX_MESSAGE_SIZE {
            return Err(DeltaError::StorageError(format!(
                "Message too large: {} bytes (max: {})",
                len, MAX_MESSAGE_SIZE
            )));
        }

        // Read message body.
        let mut bytes = vec![0u8; len];
        self.stream
            .read_exact(&mut bytes)
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to read message body: {}", e)))?;

        Message::from_bytes(&bytes)
    }

    /// Send a message and wait for a response.
    pub async fn request(&mut self, message: &Message) -> DeltaResult<Message> {
        self.send(message).await?;
        self.receive().await
    }
}

/// TCP listener for incoming cluster connections.
pub struct Listener {
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl Listener {
    /// Bind to an address and start listening.
    pub async fn bind(addr: SocketAddr) -> DeltaResult<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| DeltaError::StorageError(format!("Failed to bind to {}: {}", addr, e)))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| DeltaError::StorageError(format!("Failed to get local address: {}", e)))?;

        Ok(Self {
            listener,
            local_addr,
        })
    }

    /// Get the local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accept an incoming connection.
    pub async fn accept(&self) -> DeltaResult<Connection> {
        let (stream, peer_addr) =
            self.listener.accept().await.map_err(|e| {
                DeltaError::StorageError(format!("Failed to accept connection: {}", e))
            })?;

        Ok(Connection::new(stream, peer_addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_node_id_generation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new();
        let display = format!("{}", id);
        assert_eq!(display.len(), 8);
    }

    #[test]
    fn test_peer_info_creation() {
        let node_id = NodeId::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7878);
        let peer = PeerInfo::new(node_id.clone(), addr);

        assert_eq!(peer.node_id, node_id);
        assert_eq!(peer.address, addr);
        assert_eq!(peer.status, PeerStatus::Unknown);
    }

    #[test]
    fn test_message_serialization() {
        let node_id = NodeId::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7878);

        let message = Message::Join {
            node_id: node_id.clone(),
            address: addr,
        };

        let bytes = message.to_bytes().unwrap();
        let decoded = Message::from_bytes(&bytes).unwrap();

        match decoded {
            Message::Join {
                node_id: decoded_id,
                address: decoded_addr,
            } => {
                assert_eq!(decoded_id, node_id);
                assert_eq!(decoded_addr, addr);
            }
            _ => panic!("Expected Join message"),
        }
    }

    #[test]
    fn test_ping_pong_messages() {
        let node_id = NodeId::new();

        let ping = Message::Ping {
            node_id: node_id.clone(),
        };
        let pong = Message::Pong {
            node_id: node_id.clone(),
        };

        // Verify serialization round-trip.
        let ping_bytes = ping.to_bytes().unwrap();
        let pong_bytes = pong.to_bytes().unwrap();

        let decoded_ping = Message::from_bytes(&ping_bytes).unwrap();
        let decoded_pong = Message::from_bytes(&pong_bytes).unwrap();

        match decoded_ping {
            Message::Ping { node_id: id } => assert_eq!(id, node_id),
            _ => panic!("Expected Ping message"),
        }

        match decoded_pong {
            Message::Pong { node_id: id } => assert_eq!(id, node_id),
            _ => panic!("Expected Pong message"),
        }
    }

    #[tokio::test]
    async fn test_listener_and_connection() {
        // Bind to a random port.
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let listener = Listener::bind(addr).await.unwrap();
        let listen_addr = listener.local_addr();

        // Spawn a task to accept a connection.
        let accept_handle = tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            let msg = conn.receive().await.unwrap();
            conn.send(&Message::Pong {
                node_id: NodeId::new(),
            })
            .await
            .unwrap();
            msg
        });

        // Connect and send a message.
        let mut client = Connection::connect(listen_addr).await.unwrap();
        let node_id = NodeId::new();
        client
            .send(&Message::Ping {
                node_id: node_id.clone(),
            })
            .await
            .unwrap();

        // Wait for response.
        let response = client.receive().await.unwrap();
        match response {
            Message::Pong { .. } => {}
            _ => panic!("Expected Pong response"),
        }

        // Verify server received the ping.
        let received = accept_handle.await.unwrap();
        match received {
            Message::Ping { node_id: id } => assert_eq!(id, node_id),
            _ => panic!("Expected Ping message"),
        }
    }
}
