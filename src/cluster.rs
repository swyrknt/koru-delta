/// Cluster management for distributed KoruDelta.
///
/// This module provides the high-level cluster management functionality:
///
/// - Node lifecycle (start, join, leave)
/// - Peer tracking and discovery
/// - Gossip protocol for cluster membership
/// - Cluster state management
///
/// # Design
///
/// A KoruDelta cluster is a peer-to-peer network where:
/// - Any node can accept writes
/// - Writes are propagated to all peers
/// - Eventually consistent with causal ordering
/// - Nodes can join/leave at any time
use crate::error::{DeltaError, DeltaResult};
use crate::network::{Connection, Listener, Message, NodeId, PeerInfo, PeerStatus, DEFAULT_PORT};
use crate::storage::CausalStorage;
use crate::types::{FullKey, VectorClock, VersionedValue};
use chrono::Utc;
use dashmap::DashMap;
use koru_lambda_core::DistinctionEngine;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;

/// Configuration for a cluster node.
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Address to bind for cluster communication.
    pub bind_addr: SocketAddr,
    /// Optional address of an existing peer to join.
    pub join_addr: Option<SocketAddr>,
    /// Interval for heartbeat pings (default: 5 seconds).
    pub heartbeat_interval: Duration,
    /// Interval for gossip announcements (default: 10 seconds).
    pub gossip_interval: Duration,
    /// Timeout for peer connections (default: 5 seconds).
    pub connection_timeout: Duration,
    /// Minimum number of peers required for quorum (default: 1).
    /// Set to (expected_cluster_size / 2) + 1 for majority quorum.
    pub quorum_size: usize,
    /// Whether to require quorum for writes (default: false).
    pub require_quorum_for_writes: bool,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)),
            join_addr: None,
            heartbeat_interval: Duration::from_secs(5),
            gossip_interval: Duration::from_secs(10),
            connection_timeout: Duration::from_secs(5),
            quorum_size: 1,                   // Default: single node is sufficient
            require_quorum_for_writes: false, // Default: allow writes without quorum
        }
    }
}

impl ClusterConfig {
    /// Create a new cluster config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the bind address.
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Set an address to join.
    pub fn join(mut self, addr: SocketAddr) -> Self {
        self.join_addr = Some(addr);
        self
    }
}

/// Internal cluster state.
struct ClusterState {
    /// Known peers in the cluster.
    peers: DashMap<NodeId, PeerInfo>,
    /// Partition state tracking.
    partition_state: RwLock<PartitionState>,
}

/// State of the cluster from a partition perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionState {
    /// Normal operation - have quorum.
    Healthy,
    /// Partition detected - lost quorum.
    Partitioned,
    /// Recovering from partition - reconciling with peers.
    Recovering,
}

impl ClusterState {
    fn new(_advertised_addr: SocketAddr) -> Self {
        Self {
            peers: DashMap::new(),
            partition_state: RwLock::new(PartitionState::Healthy),
        }
    }

    /// Check if we have quorum based on peer count.
    fn has_quorum(&self, quorum_size: usize) -> bool {
        // Count healthy peers + ourselves
        let healthy_peers = self
            .peers
            .iter()
            .filter(|p| matches!(p.status, PeerStatus::Healthy))
            .count();
        let total_nodes = healthy_peers + 1; // +1 for ourselves
        total_nodes >= quorum_size
    }

    /// Get current partition state.
    async fn partition_state(&self) -> PartitionState {
        *self.partition_state.read().await
    }

    /// Update partition state.
    async fn set_partition_state(&self, state: PartitionState) {
        let mut guard = self.partition_state.write().await;
        *guard = state;
    }

    /// Add or update a peer.
    fn upsert_peer(&self, peer: PeerInfo) {
        self.peers
            .entry(peer.node_id.clone())
            .and_modify(|existing| {
                existing.last_seen = peer.last_seen;
                existing.status = peer.status;
            })
            .or_insert(peer);
    }

    /// Get all peers as a list.
    fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Update peer status.
    fn update_peer_status(&self, node_id: &NodeId, status: PeerStatus) {
        if let Some(mut peer) = self.peers.get_mut(node_id) {
            peer.status = status;
            peer.last_seen = Utc::now();
        }
    }

    /// Remove unreachable peers that haven't been seen in a while.
    fn prune_stale_peers(&self, max_age: Duration) {
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        self.peers.retain(|_, peer| peer.last_seen > cutoff);
    }
}

/// A node in the KoruDelta cluster.
///
/// ClusterNode manages the distributed aspects of KoruDelta:
/// - Network communication with peers
/// - Data synchronization
/// - Cluster membership
pub struct ClusterNode {
    /// This node's unique identifier.
    node_id: NodeId,
    /// Cluster configuration.
    config: ClusterConfig,
    /// Cluster state (peers, etc.).
    state: Arc<ClusterState>,
    /// The local storage engine.
    storage: Arc<CausalStorage>,
    /// The distinction engine.
    engine: Arc<DistinctionEngine>,
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
    /// Flag indicating if the node is running.
    running: Arc<RwLock<bool>>,
    /// Actual bound address (may differ from config if port 0 was used).
    actual_addr: Arc<RwLock<Option<SocketAddr>>>,
}

impl ClusterNode {
    /// Create a new cluster node.
    pub fn new(
        storage: Arc<CausalStorage>,
        engine: Arc<DistinctionEngine>,
        config: ClusterConfig,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            node_id: NodeId::new(),
            state: Arc::new(ClusterState::new(config.bind_addr)),
            storage,
            engine,
            config,
            shutdown_tx,
            running: Arc::new(RwLock::new(false)),
            actual_addr: Arc::new(RwLock::new(None)),
        }
    }

    /// Get this node's ID.
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the bind address (returns actual bound address if available).
    pub fn bind_addr(&self) -> SocketAddr {
        // Try to get actual address first, fall back to config.
        // This is a sync method, so we can't await. Use try_read instead.
        if let Ok(guard) = self.actual_addr.try_read() {
            if let Some(addr) = *guard {
                return addr;
            }
        }
        self.config.bind_addr
    }

    /// Get the actual bound address (async version).
    pub async fn actual_addr(&self) -> Option<SocketAddr> {
        *self.actual_addr.read().await
    }

    /// Get all known peers.
    pub fn peers(&self) -> Vec<PeerInfo> {
        self.state.get_peers()
    }

    /// Check if the node is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Check if the cluster has quorum (enough healthy peers).
    pub async fn has_quorum(&self) -> bool {
        self.state.has_quorum(self.config.quorum_size)
    }

    /// Check if writes should be allowed based on quorum requirements.
    ///
    /// Returns `true` if:
    /// - Quorum is not required for writes, OR
    /// - Quorum is required and we have it
    pub async fn is_write_allowed(&self) -> bool {
        if !self.config.require_quorum_for_writes {
            return true;
        }
        self.has_quorum().await
    }

    /// Get the current partition state.
    pub async fn partition_state(&self) -> PartitionState {
        self.state.partition_state().await
    }

    /// Get the current partition state as a string.
    pub async fn partition_state_str(&self) -> &'static str {
        match self.state.partition_state().await {
            PartitionState::Healthy => "healthy",
            PartitionState::Partitioned => "partitioned",
            PartitionState::Recovering => "recovering",
        }
    }

    /// Start the cluster node.
    ///
    /// This starts the network listener and background tasks for:
    /// - Accepting incoming connections
    /// - Heartbeat pings
    /// - Gossip announcements
    ///
    /// If a join address is configured, it will attempt to join the cluster.
    pub async fn start(&self) -> DeltaResult<()> {
        // Check if already running.
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(DeltaError::StorageError("Node already running".to_string()));
            }
            *running = true;
        }

        // Start the network listener.
        let listener = Listener::bind(self.config.bind_addr).await?;
        let actual_addr = listener.local_addr();

        // Store the actual bound address (important when binding to port 0).
        {
            let mut addr_guard = self.actual_addr.write().await;
            *addr_guard = Some(actual_addr);
        }

        // Join cluster if configured.
        if let Some(join_addr) = self.config.join_addr {
            self.join_cluster(join_addr).await?;
        }

        // Spawn the connection handler.
        let storage = Arc::clone(&self.storage);
        let state = Arc::clone(&self.state);
        let node_id = self.node_id.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        if let Ok(conn) = result {
                            let storage = Arc::clone(&storage);
                            let state = Arc::clone(&state);
                            let node_id = node_id.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(conn, storage, state, node_id).await {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Spawn heartbeat task.
        let state = Arc::clone(&self.state);
        let node_id = self.node_id.clone();
        let heartbeat_interval = self.config.heartbeat_interval;
        let quorum_size = self.config.quorum_size;
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut ticker = interval(heartbeat_interval);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        send_heartbeats(&state, &node_id, quorum_size).await;
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Spawn gossip task.
        let state = Arc::clone(&self.state);
        let node_id = self.node_id.clone();
        let gossip_interval = self.config.gossip_interval;
        let bind_addr = actual_addr;
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut ticker = interval(gossip_interval);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        send_gossip(&state, &node_id, bind_addr).await;
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Spawn anti-entropy task for continuous reconciliation.
        let state = Arc::clone(&self.state);
        let node_id = self.node_id.clone();
        let storage = Arc::clone(&self.storage);
        let anti_entropy_interval = Duration::from_secs(30); // Every 30 seconds
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut ticker = interval(anti_entropy_interval);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        run_anti_entropy(&state, &storage, &node_id).await;
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the cluster node.
    pub async fn stop(&self) -> DeltaResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        *running = false;

        // Send shutdown signal.
        let _ = self.shutdown_tx.send(());
        Ok(())
    }

    /// Join an existing cluster.
    async fn join_cluster(&self, peer_addr: SocketAddr) -> DeltaResult<()> {
        let mut conn = Connection::connect(peer_addr).await?;

        // Send join request.
        let response = conn
            .request(&Message::Join {
                node_id: self.node_id.clone(),
                address: self.config.bind_addr,
            })
            .await?;

        match response {
            Message::JoinAck { node_id, peers } => {
                // Add the peer we joined.
                self.state.upsert_peer(PeerInfo {
                    node_id: node_id.clone(),
                    address: peer_addr,
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                    status: PeerStatus::Healthy,
                });

                // Add all peers from the response.
                for peer in peers {
                    if peer.node_id != self.node_id {
                        self.state.upsert_peer(peer);
                    }
                }

                // Request full snapshot.
                self.sync_from_peer(&mut conn).await?;

                Ok(())
            }
            Message::Error { message } => Err(DeltaError::StorageError(format!(
                "Join failed: {}",
                message
            ))),
            _ => Err(DeltaError::StorageError(
                "Unexpected response to join request".to_string(),
            )),
        }
    }

    /// Sync data from a peer.
    async fn sync_from_peer(&self, conn: &mut Connection) -> DeltaResult<()> {
        let response = conn
            .request(&Message::SnapshotRequest {
                node_id: self.node_id.clone(),
            })
            .await?;

        match response {
            Message::SnapshotResponse {
                current_state,
                history_log,
                ..
            } => {
                // Merge the snapshot into local storage.
                self.merge_snapshot(current_state, history_log)?;
                Ok(())
            }
            Message::Error { message } => Err(DeltaError::StorageError(format!(
                "Sync failed: {}",
                message
            ))),
            _ => Err(DeltaError::StorageError(
                "Unexpected response to snapshot request".to_string(),
            )),
        }
    }

    /// Merge a snapshot into local storage.
    fn merge_snapshot(
        &self,
        current_state: Vec<(FullKey, VersionedValue)>,
        history_log: Vec<(FullKey, Vec<VersionedValue>)>,
    ) -> DeltaResult<()> {
        // Convert to HashMaps.
        let current: HashMap<FullKey, VersionedValue> = current_state.into_iter().collect();
        let history: HashMap<FullKey, Vec<VersionedValue>> = history_log.into_iter().collect();

        // Create a new storage from the snapshot and merge.
        // For simplicity, we replace local data (this is safe since we're joining fresh).
        let new_storage = CausalStorage::from_snapshot(Arc::clone(&self.engine), current, history);

        // Copy data from new_storage to self.storage.
        // This is a bit hacky but works for now.
        let (current_state, _history_log) = new_storage.create_snapshot();
        for (key, value) in current_state {
            self.storage
                .put(&key.namespace, &key.key, (*value.value).clone())?;
        }

        Ok(())
    }

    /// Broadcast a write to all peers with ACK tracking.
    pub async fn broadcast_write(&self, key: FullKey, value: VersionedValue) {
        let message = Message::WriteEvent {
            node_id: self.node_id.clone(),
            key: key.clone(),
            value: value.clone(),
        };
        let version_id = value.write_id.clone();

        for peer in self.state.get_peers() {
            let _node_id = self.node_id.clone();
            let message = message.clone();
            let version_id = version_id.clone();
            let key = key.clone();

            tokio::spawn(async move {
                let mut attempts = 0;
                let max_attempts = 3;

                while attempts < max_attempts {
                    attempts += 1;

                    match Connection::connect(peer.address).await {
                        Ok(mut conn) => {
                            // Send the write event
                            if let Err(e) = conn.send(&message).await {
                                tracing::debug!("Failed to send write to {}: {}", peer.node_id, e);
                                continue;
                            }

                            // Wait for ACK with timeout
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(5),
                                conn.receive(),
                            )
                            .await
                            {
                                Ok(Ok(Message::WriteAck {
                                    node_id: ack_node_id,
                                    key: ack_key,
                                    version_id: ack_version,
                                })) => {
                                    if ack_node_id == peer.node_id
                                        && ack_key == key
                                        && ack_version == version_id
                                    {
                                        tracing::trace!(
                                            "Received ACK from {} for {}",
                                            peer.node_id,
                                            version_id
                                        );
                                        return; // Success!
                                    }
                                }
                                Ok(Ok(_)) => {
                                    tracing::debug!("Unexpected response from {}", peer.node_id);
                                }
                                Ok(Err(e)) => {
                                    tracing::debug!(
                                        "Failed to receive ACK from {}: {}",
                                        peer.node_id,
                                        e
                                    );
                                }
                                Err(_) => {
                                    tracing::debug!(
                                        "Timeout waiting for ACK from {}",
                                        peer.node_id
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Failed to connect to {}: {}", peer.node_id, e);
                        }
                    }

                    // Exponential backoff before retry
                    if attempts < max_attempts {
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempts as u64))
                            .await;
                    }
                }

                tracing::warn!(
                    "Failed to broadcast write to {} after {} attempts",
                    peer.node_id,
                    max_attempts
                );
            });
        }
    }
}

/// Handle an incoming connection.
async fn handle_connection(
    mut conn: Connection,
    storage: Arc<CausalStorage>,
    state: Arc<ClusterState>,
    node_id: NodeId,
) -> DeltaResult<()> {
    loop {
        let message = match conn.receive().await {
            Ok(msg) => msg,
            Err(_) => break, // Connection closed.
        };

        let response = handle_message(message, &storage, &state, &node_id)?;

        if let Some(resp) = response {
            conn.send(&resp).await?;
        }
    }

    Ok(())
}

/// Handle a single message.
fn handle_message(
    message: Message,
    storage: &Arc<CausalStorage>,
    state: &Arc<ClusterState>,
    node_id: &NodeId,
) -> DeltaResult<Option<Message>> {
    match message {
        Message::Join {
            node_id: peer_id,
            address,
        } => {
            // Add the new peer.
            state.upsert_peer(PeerInfo::new(peer_id, address));

            // Respond with our info and peer list.
            Ok(Some(Message::JoinAck {
                node_id: node_id.clone(),
                peers: state.get_peers(),
            }))
        }

        Message::Ping { node_id: peer_id } => {
            state.update_peer_status(&peer_id, PeerStatus::Healthy);
            Ok(Some(Message::Pong {
                node_id: node_id.clone(),
            }))
        }

        Message::Pong { node_id: peer_id } => {
            state.update_peer_status(&peer_id, PeerStatus::Healthy);
            Ok(None)
        }

        Message::Announce {
            node_id: announcing_peer_id,
            address,
            peers,
        } => {
            // Update/add the announcing peer.
            state.upsert_peer(PeerInfo {
                node_id: announcing_peer_id,
                address,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                status: PeerStatus::Healthy,
            });

            // Add any new peers from the announcement.
            for peer in peers {
                if peer.node_id != *node_id {
                    state.upsert_peer(peer);
                }
            }

            Ok(None)
        }

        Message::SnapshotRequest { .. } => {
            let (current_state, history_log) = storage.create_snapshot();
            let current_vec: Vec<_> = current_state.into_iter().collect();
            let history_vec: Vec<_> = history_log.into_iter().collect();

            Ok(Some(Message::SnapshotResponse {
                node_id: node_id.clone(),
                current_state: current_vec,
                history_log: history_vec,
            }))
        }

        Message::WriteEvent {
            node_id: _peer_id,
            key,
            value,
        } => {
            // Apply the write with causal ordering check.
            match storage.put_causal(
                &key.namespace,
                &key.key,
                (*value.value).clone(),
                value.vector_clock.clone(),
            )? {
                crate::types::CausalWriteResult::Applied(_)
                | crate::types::CausalWriteResult::Duplicate(_) => {
                    // Successfully applied or already had it
                    Ok(Some(Message::WriteAck {
                        node_id: node_id.clone(),
                        key,
                        version_id: value.write_id.clone(),
                    }))
                }
                crate::types::CausalWriteResult::Rejected(_) => {
                    // Causally earlier than current - still acknowledge to stop retries
                    Ok(Some(Message::WriteAck {
                        node_id: node_id.clone(),
                        key,
                        version_id: value.write_id.clone(),
                    }))
                }
                crate::types::CausalWriteResult::Conflict {
                    existing,
                    incoming_clock,
                } => {
                    // Concurrent write conflict - merge using last-write-wins
                    tracing::warn!(
                        "Concurrent write conflict for {:?}: existing={}, incoming={}. Merging...",
                        key,
                        existing.write_id,
                        value.write_id
                    );

                    // Attempt to merge the concurrent writes
                    match storage.merge_concurrent_writes(
                        &key.namespace,
                        &key.key,
                        &existing,
                        (*value.value).clone(),
                        incoming_clock,
                    ) {
                        Ok(merged) => Ok(Some(Message::WriteAck {
                            node_id: node_id.clone(),
                            key,
                            version_id: merged.write_id.clone(),
                        })),
                        Err(e) => {
                            tracing::error!("Failed to merge concurrent writes: {}", e);
                            // Still acknowledge to prevent infinite retries
                            Ok(Some(Message::WriteAck {
                                node_id: node_id.clone(),
                                key,
                                version_id: value.write_id.clone(),
                            }))
                        }
                    }
                }
            }
        }

        Message::SyncRequest {
            node_id: _,
            keys,
            tombstones: known_tombstones,
        } => {
            let mut updates = Vec::new();
            let mut tombstones_to_send = Vec::new();

            for (key, last_version) in keys {
                // Check if this key has a tombstone
                if let Some(tombstone) = storage.get_tombstone(&key.namespace, &key.key) {
                    // Check if the peer already knows about this tombstone
                    let peer_knows = known_tombstones
                        .get(&key)
                        .map(|peer_clock| {
                            // If our tombstone clock dominates peer's, they need update
                            tombstone.vector_clock.compare(peer_clock)
                                != Some(std::cmp::Ordering::Less)
                        })
                        .unwrap_or(true); // If peer doesn't have this tombstone, send it

                    if peer_knows {
                        tombstones_to_send.push(tombstone);
                    }
                    continue; // Skip checking for updates on deleted keys
                }

                if let Ok(history) = storage.history(&key.namespace, &key.key) {
                    // Find updates since the last known version.
                    let new_versions: Vec<_> = match last_version {
                        Some(version_id) => history
                            .into_iter()
                            .skip_while(|entry| entry.version_id != version_id)
                            .skip(1) // Skip the known version.
                            .map(|entry| {
                                // For sync: write_id = version_id from history, distinction_id same as write_id for now
                                VersionedValue::from_json(
                                    entry.value,
                                    entry.timestamp,
                                    entry.version_id.clone(), // write_id
                                    entry.version_id,         // distinction_id
                                    None,
                                    VectorClock::new(),
                                )
                            })
                            .collect(),
                        None => history
                            .into_iter()
                            .map(|entry| {
                                VersionedValue::from_json(
                                    entry.value,
                                    entry.timestamp,
                                    entry.version_id.clone(), // write_id
                                    entry.version_id,         // distinction_id
                                    None,
                                    VectorClock::new(),
                                )
                            })
                            .collect(),
                    };

                    if !new_versions.is_empty() {
                        updates.push((key, new_versions));
                    }
                }
            }

            // Send tombstones for keys the peer knows about but we have deleted
            for tombstone in storage.get_all_tombstones() {
                if !known_tombstones.contains_key(&tombstone.key) {
                    tombstones_to_send.push(tombstone);
                }
            }

            Ok(Some(Message::SyncResponse {
                node_id: node_id.clone(),
                updates,
                tombstones: tombstones_to_send,
            }))
        }

        _ => Ok(None),
    }
}

/// Send heartbeat pings to all peers.
async fn send_heartbeats(state: &Arc<ClusterState>, node_id: &NodeId, quorum_size: usize) {
    let peers = state.get_peers();

    for peer in peers {
        let node_id = node_id.clone();
        let state = Arc::clone(state);
        tokio::spawn(async move {
            match Connection::connect(peer.address).await {
                Ok(mut conn) => {
                    let msg = Message::Ping {
                        node_id: node_id.clone(),
                    };
                    if conn.request(&msg).await.is_ok() {
                        state.update_peer_status(&peer.node_id, PeerStatus::Healthy);
                    } else {
                        state.update_peer_status(&peer.node_id, PeerStatus::Unreachable);
                    }
                }
                Err(_) => {
                    state.update_peer_status(&peer.node_id, PeerStatus::Unreachable);
                }
            }
        });
    }

    // Prune stale peers.
    state.prune_stale_peers(Duration::from_secs(60));

    // Check partition state after updating peer statuses
    let current_state = state.partition_state().await;
    let has_quorum = state.has_quorum(quorum_size);

    match (current_state, has_quorum) {
        (PartitionState::Healthy, false) => {
            // Lost quorum - entering partitioned state
            tracing::warn!(
                "Lost quorum! Entering partitioned state (healthy peers < {})",
                quorum_size
            );
            state.set_partition_state(PartitionState::Partitioned).await;
        }
        (PartitionState::Partitioned, true) => {
            // Regained quorum - entering recovery
            tracing::info!("Regained quorum! Entering recovery state");
            state.set_partition_state(PartitionState::Recovering).await;
        }
        _ => {
            // State unchanged
        }
    }
}

/// Send gossip announcements to all peers.
async fn send_gossip(state: &Arc<ClusterState>, node_id: &NodeId, bind_addr: SocketAddr) {
    let peers = state.get_peers();
    let message = Message::Announce {
        node_id: node_id.clone(),
        address: bind_addr,
        peers: peers.clone(),
    };

    for peer in peers {
        let message = message.clone();
        tokio::spawn(async move {
            if let Ok(mut conn) = Connection::connect(peer.address).await {
                let _ = conn.send(&message).await;
            }
        });
    }
}

/// Run anti-entropy reconciliation with all peers.
async fn run_anti_entropy(
    state: &Arc<ClusterState>,
    storage: &Arc<CausalStorage>,
    node_id: &NodeId,
) {
    let peers = state.get_peers();

    // Only run anti-entropy if we have healthy peers
    let healthy_peers: Vec<_> = peers
        .into_iter()
        .filter(|p| matches!(p.status, PeerStatus::Healthy))
        .collect();

    if healthy_peers.is_empty() {
        return;
    }

    tracing::trace!("Running anti-entropy with {} peers", healthy_peers.len());

    for peer in healthy_peers {
        let storage = Arc::clone(storage);
        let node_id = node_id.clone();

        tokio::spawn(async move {
            // Get our current key set with version info
            let mut keys_to_check = HashMap::new();

            // Get all namespaces and keys
            // TODO: Optimize this to only check recently changed keys
            let namespaces = storage.list_namespaces();
            for ns in namespaces {
                let keys = storage.list_keys(&ns);
                for key in keys {
                    let full_key = FullKey::new(&ns, &key);
                    // Get latest version ID for this key
                    let version = storage
                        .get(&ns, &key)
                        .ok()
                        .map(|v| v.write_id().to_string());
                    keys_to_check.insert(full_key, version);
                }
            }

            // Get our known tombstones for tombstone propagation
            let our_tombstones: HashMap<FullKey, VectorClock> = storage
                .get_all_tombstones()
                .into_iter()
                .map(|t| (t.key.clone(), t.vector_clock))
                .collect();

            // Send sync request to peer
            match Connection::connect(peer.address).await {
                Ok(mut conn) => {
                    let request = Message::SyncRequest {
                        node_id: node_id.clone(),
                        keys: keys_to_check,
                        tombstones: our_tombstones,
                    };

                    match conn.request(&request).await {
                        Ok(Message::SyncResponse {
                            updates,
                            tombstones,
                            ..
                        }) => {
                            // Apply updates from peer
                            for (key, versions) in updates {
                                // Skip if we have a tombstone for this key
                                if storage.has_tombstone(&key.namespace, &key.key) {
                                    tracing::trace!("Skipping update for deleted key {:?}", key);
                                    continue;
                                }

                                for version in versions {
                                    // TODO: Use vector clock merge instead of blind put
                                    if let Err(e) = storage.put(
                                        &key.namespace,
                                        &key.key,
                                        (*version.value).clone(),
                                    ) {
                                        tracing::debug!(
                                            "Failed to apply anti-entropy update: {}",
                                            e
                                        );
                                    }
                                }
                            }

                            // Apply tombstones from peer
                            for tombstone in tombstones {
                                // Check if we already have this key
                                if let Ok(existing) =
                                    storage.get(&tombstone.key.namespace, &tombstone.key.key)
                                {
                                    // Check if the peer's tombstone causally supersedes our value
                                    match tombstone.vector_clock.compare(existing.vector_clock()) {
                                        Some(std::cmp::Ordering::Greater) => {
                                            // Peer has newer tombstone, delete our value
                                            if let Err(e) = storage.delete_causal(
                                                &tombstone.key.namespace,
                                                &tombstone.key.key,
                                                tombstone.vector_clock.clone(),
                                                &tombstone.deleted_by,
                                            ) {
                                                tracing::debug!("Failed to apply tombstone: {}", e);
                                            } else {
                                                tracing::info!(
                                                    "Applied tombstone for {:?} from peer",
                                                    tombstone.key
                                                );
                                            }
                                        }
                                        _ => {
                                            // Our value is newer or concurrent, keep it
                                            tracing::trace!(
                                                "Skipping tombstone for {:?} - local value is newer",
                                                tombstone.key
                                            );
                                        }
                                    }
                                } else if !storage
                                    .has_tombstone(&tombstone.key.namespace, &tombstone.key.key)
                                {
                                    // We don't have this key and don't have a tombstone - record the tombstone
                                    storage.insert_tombstone(tombstone);
                                }
                            }

                            tracing::trace!("Anti-entropy completed with {}", peer.node_id);
                        }
                        Ok(_) => {
                            tracing::debug!(
                                "Unexpected response from {} during anti-entropy",
                                peer.node_id
                            );
                        }
                        Err(e) => {
                            tracing::debug!("Anti-entropy failed with {}: {}", peer.node_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to connect to {} for anti-entropy: {}",
                        peer.node_id,
                        e
                    );
                }
            }
        });
    }
}

/// Cluster status information.
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    /// This node's ID.
    pub node_id: NodeId,
    /// This node's address.
    pub address: SocketAddr,
    /// Number of known peers.
    pub peer_count: usize,
    /// Number of healthy peers.
    pub healthy_peers: usize,
    /// Whether this node is running.
    pub is_running: bool,
}

impl ClusterNode {
    /// Get cluster status.
    pub async fn status(&self) -> ClusterStatus {
        let peers = self.state.get_peers();
        let healthy = peers
            .iter()
            .filter(|p| p.status == PeerStatus::Healthy)
            .count();

        ClusterStatus {
            node_id: self.node_id.clone(),
            address: self.config.bind_addr,
            peer_count: peers.len(),
            healthy_peers: healthy,
            is_running: *self.running.read().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_storage() -> (Arc<CausalStorage>, Arc<DistinctionEngine>) {
        let engine = Arc::new(DistinctionEngine::new());
        let storage = Arc::new(CausalStorage::new(Arc::clone(&engine)));
        (storage, engine)
    }

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert_eq!(config.bind_addr.port(), DEFAULT_PORT);
        assert!(config.join_addr.is_none());
    }

    #[test]
    fn test_cluster_config_builder() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8888);
        let join = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), 8888);

        let config = ClusterConfig::new().bind_addr(addr).join(join);

        assert_eq!(config.bind_addr, addr);
        assert_eq!(config.join_addr, Some(join));
    }

    #[test]
    fn test_cluster_state() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7878);
        let state = ClusterState::new(addr);

        // Initially no peers.
        assert!(state.get_peers().is_empty());

        // Add a peer.
        let peer_id = NodeId::new();
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 7878);
        state.upsert_peer(PeerInfo::new(peer_id.clone(), peer_addr));

        assert_eq!(state.get_peers().len(), 1);

        // Update peer status.
        state.update_peer_status(&peer_id, PeerStatus::Healthy);
        let peers = state.get_peers();
        assert_eq!(peers[0].status, PeerStatus::Healthy);
    }

    #[tokio::test]
    async fn test_cluster_node_creation() {
        let (storage, engine) = create_test_storage();
        let config = ClusterConfig::default();
        let node = ClusterNode::new(storage, engine, config);

        assert!(!node.is_running().await);
        assert!(node.peers().is_empty());
    }

    #[tokio::test]
    async fn test_cluster_node_start_stop() {
        let (storage, engine) = create_test_storage();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let config = ClusterConfig::new().bind_addr(addr);
        let node = ClusterNode::new(storage, engine, config);

        // Start the node.
        node.start().await.unwrap();
        assert!(node.is_running().await);

        // Stop the node.
        node.stop().await.unwrap();
        assert!(!node.is_running().await);
    }

    #[tokio::test]
    async fn test_cluster_status() {
        let (storage, engine) = create_test_storage();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let config = ClusterConfig::new().bind_addr(addr);
        let node = ClusterNode::new(storage, engine, config);

        let status = node.status().await;
        assert_eq!(status.peer_count, 0);
        assert_eq!(status.healthy_peers, 0);
        assert!(!status.is_running);
    }

    #[tokio::test]
    async fn test_two_node_cluster() {
        // Create first node.
        let (storage1, engine1) = create_test_storage();
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let config1 = ClusterConfig::new().bind_addr(addr1);
        let node1 = ClusterNode::new(storage1.clone(), engine1, config1);

        // Start first node and get its actual address.
        node1.start().await.unwrap();

        // Give it a moment to start listening.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Create second node that joins the first.
        let (storage2, engine2) = create_test_storage();
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let config2 = ClusterConfig::new()
            .bind_addr(addr2)
            .join(node1.bind_addr());
        let node2 = ClusterNode::new(storage2.clone(), engine2, config2);

        // Store some data on node1 before node2 joins.
        storage1
            .put("test", "key1", serde_json::json!({"value": 1}))
            .unwrap();

        // Start second node (will join cluster).
        node2.start().await.unwrap();

        // Give it time to sync.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Node2 should have the peer.
        assert!(!node2.peers().is_empty());

        // Clean up.
        node1.stop().await.unwrap();
        node2.stop().await.unwrap();
    }
}
