/// Storage layer with emergent causal tracking.
///
/// This module implements the core storage engine for KoruDelta. Unlike traditional
/// databases that overwrite values, this storage layer captures the emergence of
/// distinctions and their causal relationships:
///
/// - Every write creates a new distinction (content-addressed)
/// - Causal graph tracks how distinctions emerge from one another
/// - Reference graph tracks what points to what (for GC and hot memory)
/// - Time-travel queries traverse the causal graph
///
/// The storage layer is thread-safe and uses DashMap for lock-free concurrent access.
use crate::causal_graph::CausalGraph;
use crate::error::{DeltaError, DeltaResult};
use crate::mapper::DocumentMapper;
use crate::reference_graph::ReferenceGraph;
use crate::types::{FullKey, HistoryEntry, VersionedValue};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use koru_lambda_core::DistinctionEngine;
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Storage engine capturing emergent distinction behavior.
///
/// The storage layer maintains:
///
/// 1. **Current State**: Maps each key to its latest versioned value
/// 2. **Version Store**: All versions for history/time-travel (content-addressed)
/// 3. **Causal Graph**: Tracks how distinctions cause one another (emergent)
/// 4. **Reference Graph**: Tracks what points to what (emergent)
/// 5. **Value Store**: Deduplicates values by content-addressed version ID
///
/// ## Evolution from Previous Architecture
///
/// Previously: `history_log: DashMap<FullKey, Vec<VersionedValue>>`
/// Now: version_store (content-addressed) + causal_graph (causal structure)
///
/// Benefits:
/// - Deduplication via content addressing
/// - Proper causal traversal for time travel
/// - Reachability analysis for GC
#[derive(Debug)]
pub struct CausalStorage {
    /// The underlying distinction engine for content addressing
    /// Respected, unchanged - computes distinctions via 5 axioms
    engine: Arc<DistinctionEngine>,

    /// Causal graph: tracks how distinctions emerge from one another
    /// Captured from emergent behavior of put() operations
    causal_graph: CausalGraph,

    /// Reference graph: tracks what distinctions reference what
    /// Captured from emergent relationships in values
    reference_graph: ReferenceGraph,

    /// Current (latest) value for each key
    /// Maps FullKey → VersionedValue
    current_state: DashMap<FullKey, VersionedValue>,

    /// All versions (for history and time travel)
    /// Maps version_id → VersionedValue
    /// Content-addressed: same content = same ID
    version_store: DashMap<String, VersionedValue>,

    /// Deduplicated value storage
    /// Maps version_id → Arc<JsonValue>
    /// Same values share the same Arc allocation
    value_store: DashMap<String, Arc<JsonValue>>,
}

impl CausalStorage {
    /// Create a new causal storage instance.
    ///
    /// The storage captures emergent behavior from koru-lambda-core operations
    /// through causal and reference graphs.
    pub fn new(engine: Arc<DistinctionEngine>) -> Self {
        Self {
            engine,
            causal_graph: CausalGraph::new(),
            reference_graph: ReferenceGraph::new(),
            current_state: DashMap::new(),
            version_store: DashMap::new(),
            value_store: DashMap::new(),
        }
    }

    /// Store a value, capturing the emergent distinction and its relationships.
    ///
    /// This operation:
    /// 1. Computes the distinction (content hash) via koru-lambda-core
    /// 2. Captures causal emergence (links to previous version)
    /// 3. Captures references (what this value points to)
    /// 4. Updates current state
    ///
    /// # Thread Safety
    ///
    /// Thread-safe via DashMap. Causal chain maintained correctly under concurrency.
    pub fn put(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: JsonValue,
    ) -> DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);
        let timestamp = Utc::now();

        // Get previous version if it exists (causal parent)
        let previous_version = self
            .current_state
            .get(&full_key)
            .map(|v| v.write_id.clone());

        // Compute distinction via koru-lambda-core (unchanged, respected)
        let distinction = DocumentMapper::json_to_distinction(&value, &self.engine)?;
        let distinction_id = DocumentMapper::store_distinction_id(&distinction);

        // Generate unique write ID for this specific write event
        // Uses nanosecond precision to avoid collisions in rapid writes
        let write_id = format!("{}_{}", distinction_id, timestamp.timestamp_nanos_opt().unwrap_or(0));

        // Capture in causal graph (NEW: emergent tracking)
        self.causal_graph.add_node(write_id.clone());
        if let Some(ref parent_id) = previous_version {
            self.causal_graph.add_edge(parent_id.clone(), write_id.clone());
        }

        // Capture in reference graph (NEW: emergent tracking)
        self.reference_graph.add_node(write_id.clone());
        // TODO: Extract and track references from value

        // Get or create shared value from the value store (deduplication)
        // Uses distinction_id (content hash) for sharing
        let shared_value = self
            .value_store
            .entry(distinction_id.clone())
            .or_insert_with(|| Arc::new(value))
            .clone();

        // Create new versioned value with unique write_id
        let versioned = VersionedValue::new(
            shared_value, 
            timestamp, 
            write_id.clone(), // unique per write
            distinction_id,   // content hash for deduplication
            previous_version
        );

        // Store in version store (for history and time travel)
        // Uses unique write_id as key to preserve all writes
        self.version_store
            .insert(write_id.clone(), versioned.clone());

        // Update current state
        self.current_state
            .insert(full_key.clone(), versioned.clone());

        Ok(versioned)
    }

    /// Insert a versioned value directly (for persistence replay).
    ///
    /// This method preserves the original write_id and distinction_id from the WAL,
    /// maintaining the history chain correctly during replay.
    pub fn insert_direct(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        versioned: VersionedValue,
    ) -> DeltaResult<()> {
        let full_key = FullKey::new(namespace, key);
        let write_id = versioned.write_id.clone();
        let distinction_id = versioned.distinction_id.clone();
        
        // Add to causal graph (preserving original write_id)
        self.causal_graph.add_node(write_id.clone());
        if let Some(ref parent_id) = versioned.previous_version {
            self.causal_graph.add_edge(parent_id.clone(), write_id.clone());
        }

        // Add to reference graph
        self.reference_graph.add_node(write_id.clone());

        // Store value in value store (content-addressed)
        self.value_store
            .entry(distinction_id)
            .or_insert_with(|| versioned.value.clone());

        // Store in version store (keyed by write_id)
        self.version_store
            .insert(write_id.clone(), versioned.clone());

        // Update current state (this overwrites any existing entry for the key)
        self.current_state
            .insert(full_key.clone(), versioned);

        Ok(())
    }

    /// Get the current (latest) value for a key.
    pub fn get(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);

        self.current_state
            .get(&full_key)
            .map(|v| v.clone())
            .ok_or_else(|| DeltaError::KeyNotFound {
                namespace: full_key.namespace.clone(),
                key: full_key.key.clone(),
            })
    }

    /// Get the value at a specific point in time (time travel).
    ///
    /// Traverses the causal graph from current state backward,
    /// finding the most recent version at or before the given timestamp.
    pub fn get_at(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);

        // Get current version's ID
        let current = self
            .current_state
            .get(&full_key)
            .ok_or_else(|| DeltaError::KeyNotFound {
                namespace: full_key.namespace.clone(),
                key: full_key.key.clone(),
            })?;

        let current_id = current.write_id.clone();

        // Traverse ancestors via causal graph
        // Find the version with the latest timestamp that is <= query timestamp
        let mut best_version: Option<VersionedValue> = None;
        let mut to_visit = vec![current_id];
        let mut visited = std::collections::HashSet::new();

        while let Some(version_id) = to_visit.pop() {
            if !visited.insert(version_id.clone()) {
                continue;
            }

            // Get the versioned value from version store
            if let Some(versioned) = self.version_store.get(&version_id) {
                if versioned.timestamp <= timestamp {
                    // This version is at or before the query timestamp
                    // Keep it if it's the best (latest) so far
                    match &best_version {
                        None => best_version = Some(versioned.clone()),
                        Some(best) => {
                            if versioned.timestamp > best.timestamp {
                                best_version = Some(versioned.clone());
                            }
                        }
                    }
                }
            }

            // Add parents to visit
            let parents = self.causal_graph.ancestors(&version_id);
            for parent in parents {
                if !visited.contains(&parent) {
                    to_visit.push(parent);
                }
            }
        }

        best_version.ok_or_else(|| DeltaError::NoValueAtTimestamp {
            namespace: full_key.namespace,
            key: full_key.key,
            timestamp: timestamp.timestamp(),
        })
    }

    /// Get the complete history for a key via causal graph traversal.
    ///
    /// Returns all versions in causal order (oldest to newest).
    pub fn history(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let full_key = FullKey::new(namespace, key);

        // Get current version
        let current = self
            .current_state
            .get(&full_key)
            .ok_or_else(|| DeltaError::KeyNotFound {
                namespace: full_key.namespace.clone(),
                key: full_key.key.clone(),
            })?;

        // Collect all versions via causal graph traversal
        let mut versions: Vec<VersionedValue> = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut to_visit = vec![current.write_id.clone()];

        while let Some(version_id) = to_visit.pop() {
            if !visited.insert(version_id.clone()) {
                continue;
            }

            // Get version from version store
            if let Some(versioned) = self.version_store.get(&version_id) {
                versions.push(versioned.clone());
            }

            // Add parents
            let parents = self.causal_graph.ancestors(&version_id);
            for parent in parents {
                if !visited.contains(&parent) {
                    to_visit.push(parent);
                }
            }
        }

        // Sort by timestamp (oldest first)
        versions.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Convert to HistoryEntry
        Ok(versions.iter().map(HistoryEntry::from).collect())
    }

    /// Check if a key exists in the storage.
    pub fn contains_key(&self, namespace: impl Into<String>, key: impl Into<String>) -> bool {
        let full_key = FullKey::new(namespace, key);
        self.current_state.contains_key(&full_key)
    }

    /// Get the number of unique keys currently stored.
    pub fn key_count(&self) -> usize {
        self.current_state.len()
    }

    /// Get the total number of versions across all keys.
    ///
    /// Counts via causal graph (more accurate than previous history_log count).
    pub fn total_version_count(&self) -> usize {
        self.causal_graph.node_count()
    }

    /// Get all namespaces currently in use.
    pub fn list_namespaces(&self) -> Vec<String> {
        let mut namespaces: Vec<String> = self
            .current_state
            .iter()
            .map(|entry| entry.key().namespace.clone())
            .collect();

        namespaces.sort();
        namespaces.dedup();
        namespaces
    }

    /// Get all keys in a specific namespace.
    pub fn list_keys(&self, namespace: &str) -> Vec<String> {
        let mut keys: Vec<String> = self
            .current_state
            .iter()
            .filter(|entry| entry.key().namespace == namespace)
            .map(|entry| entry.key().key.clone())
            .collect();

        keys.sort();
        keys
    }

    /// Scan all key-value pairs in a namespace.
    pub fn scan_collection(&self, namespace: &str) -> Vec<(String, VersionedValue)> {
        self.current_state
            .iter()
            .filter(|entry| entry.key().namespace == namespace)
            .map(|entry| (entry.key().key.clone(), entry.value().clone()))
            .collect()
    }

    /// Scan all key-value pairs across all namespaces.
    pub fn scan_all(&self) -> Vec<(FullKey, VersionedValue)> {
        self.current_state
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Access the causal graph (for advanced operations).
    pub fn causal_graph(&self) -> &CausalGraph {
        &self.causal_graph
    }

    /// Access the reference graph (for GC and hot memory).
    pub fn reference_graph(&self) -> &ReferenceGraph {
        &self.reference_graph
    }

    /// Create a consistent snapshot of the database state.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn create_snapshot(
        &self,
    ) -> (
        std::collections::HashMap<FullKey, VersionedValue>,
        std::collections::HashMap<FullKey, Vec<VersionedValue>>,
    ) {
        use std::collections::HashMap;

        // Snapshot current state
        let current_state: HashMap<_, _> = self
            .current_state
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        // Build history log from causal graph - traverse and collect all versions
        let mut history_log: HashMap<FullKey, Vec<VersionedValue>> = HashMap::new();
        
        for entry in self.current_state.iter() {
            let key = entry.key().clone();
            let current = entry.value().clone();
            
            // Traverse causal graph to collect all versions
            let mut history = Vec::new();
            let mut visited = std::collections::HashSet::new();
            let mut to_visit = vec![current.write_id.clone()];
            
            while let Some(write_id) = to_visit.pop() {
                if !visited.insert(write_id.clone()) {
                    continue;
                }
                
                // Get the version from version store
                if let Some(versioned) = self.version_store.get(&write_id) {
                    history.push(versioned.clone());
                }
                
                // Add parents to visit
                let parents = self.causal_graph.ancestors(&write_id);
                for parent in parents {
                    if !visited.contains(&parent) {
                        to_visit.push(parent);
                    }
                }
            }
            
            // Sort by timestamp (oldest first) for consistent ordering
            history.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            history_log.insert(key, history);
        }

        (current_state, history_log)
    }

    /// Restore storage from a snapshot.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_snapshot(
        engine: Arc<DistinctionEngine>,
        current_state: std::collections::HashMap<FullKey, VersionedValue>,
        history_log: std::collections::HashMap<FullKey, Vec<VersionedValue>>,
    ) -> Self {
        let storage = Self::new(engine);

        // Restore current state and version store
        for (key, versioned) in current_state {
            // Add to value store for deduplication (uses distinction_id)
            storage.value_store
                .entry(versioned.distinction_id().to_string())
                .or_insert_with(|| versioned.value.clone());
            
            // Add to version store (uses write_id)
            storage.version_store
                .entry(versioned.write_id().to_string())
                .or_insert_with(|| versioned.clone());
            
            storage.current_state.insert(key, versioned);
        }

        // Rebuild causal graph and version store from history
        for (_key, history) in history_log {
            let mut prev_id: Option<String> = None;
            for versioned in history {
                let id = versioned.write_id().to_string();
                
                // Add to value store
                storage.value_store
                    .entry(id.clone())
                    .or_insert_with(|| versioned.value.clone());
                
                // Add to version store
                storage.version_store
                    .entry(id.clone())
                    .or_insert_with(|| versioned.clone());
                
                // Add to causal graph
                storage.causal_graph.add_node(id.clone());
                
                if let Some(ref parent) = prev_id {
                    storage.causal_graph.add_edge(parent.clone(), id.clone());
                }
                
                prev_id = Some(id);
            }
        }

        storage
    }

    /// Get the distinction engine.
    pub fn distinction_engine(&self) -> &Arc<DistinctionEngine> {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    fn create_storage() -> CausalStorage {
        let engine = Arc::new(DistinctionEngine::new());
        CausalStorage::new(engine)
    }

    #[test]
    fn test_put_and_get() {
        let storage = create_storage();
        let value = json!({"name": "Alice", "age": 30});

        storage.put("users", "alice", value.clone()).unwrap();
        let retrieved = storage.get("users", "alice").unwrap();

        assert_eq!(retrieved.value(), &value);
    }

    #[test]
    fn test_get_nonexistent_key() {
        let storage = create_storage();

        let result = storage.get("users", "nonexistent");
        assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));
    }

    #[test]
    fn test_versioning() {
        let storage = create_storage();

        let v1 = storage.put("users", "alice", json!({"age": 30})).unwrap();
        thread::sleep(Duration::from_millis(10));
        let v2 = storage.put("users", "alice", json!({"age": 31})).unwrap();

        // Version 2's previous_version is v1's write_id (unique per write)
        assert_eq!(v2.previous_version(), Some(v1.write_id()));

        // Current value should be v2
        let current = storage.get("users", "alice").unwrap();
        assert_eq!(current.version_id(), v2.version_id());
        
        // Values with different content have different version_ids (distinction_ids)
        assert_ne!(v1.version_id(), v2.version_id());

        // Causal graph is keyed by write_id (unique per write)
        assert!(storage.causal_graph.contains(&v1.write_id()));
        assert!(storage.causal_graph.contains(&v2.write_id()));
    }

    #[test]
    fn test_causal_graph_populated() {
        let storage = create_storage();

        let v1 = storage.put("test", "key", json!(1)).unwrap();
        let v2 = storage.put("test", "key", json!(2)).unwrap();
        let v3 = storage.put("test", "key", json!(3)).unwrap();

        // Check causal graph structure (keyed by write_id)
        assert!(storage.causal_graph.contains(&v1.write_id()));
        assert!(storage.causal_graph.contains(&v2.write_id()));
        assert!(storage.causal_graph.contains(&v3.write_id()));

        // Check edges (v1 -> v2 -> v3) using write_ids
        let v2_id = v2.write_id();
        let ancestors_v2 = storage.causal_graph.ancestors(&v2_id);
        assert!(ancestors_v2.contains(&v1.write_id().to_string()));

        let v3_id = v3.write_id();
        let ancestors_v3 = storage.causal_graph.ancestors(&v3_id);
        assert!(ancestors_v3.contains(&v2.write_id().to_string()));
        assert!(ancestors_v3.contains(&v1.write_id().to_string()));
    }

    #[test]
    fn test_contains_key() {
        let storage = create_storage();

        assert!(!storage.contains_key("users", "alice"));
        storage.put("users", "alice", json!({})).unwrap();
        assert!(storage.contains_key("users", "alice"));
    }

    #[test]
    fn test_key_count() {
        let storage = create_storage();

        assert_eq!(storage.key_count(), 0);

        storage.put("users", "alice", json!({})).unwrap();
        assert_eq!(storage.key_count(), 1);

        storage.put("users", "bob", json!({})).unwrap();
        assert_eq!(storage.key_count(), 2);

        // Updating existing key shouldn't change count
        storage
            .put("users", "alice", json!({"updated": true}))
            .unwrap();
        assert_eq!(storage.key_count(), 2);
    }

    #[test]
    fn test_total_version_count_via_causal_graph() {
        let storage = create_storage();

        // Use distinct values (content addressing means same value = same ID)
        let v1 = storage.put("users", "alice", json!({"v": 1})).unwrap();
        assert_eq!(storage.total_version_count(), 1);

        let v2 = storage.put("users", "alice", json!({"v": 2})).unwrap();
        assert_eq!(storage.total_version_count(), 2);

        let v3 = storage.put("users", "bob", json!({"v": 3})).unwrap();
        assert_eq!(storage.total_version_count(), 3);

        // Verify all are in causal graph (keyed by write_id)
        assert!(storage.causal_graph.contains(&v1.write_id().to_string()));
        assert!(storage.causal_graph.contains(&v2.write_id().to_string()));
        assert!(storage.causal_graph.contains(&v3.write_id().to_string()));
    }

    #[test]
    fn test_list_namespaces() {
        let storage = create_storage();

        storage.put("users", "alice", json!({})).unwrap();
        storage.put("users", "bob", json!({})).unwrap();
        storage.put("sessions", "s1", json!({})).unwrap();
        storage.put("config", "app", json!({})).unwrap();

        let namespaces = storage.list_namespaces();
        assert_eq!(namespaces, vec!["config", "sessions", "users"]);
    }

    #[test]
    fn test_list_keys() {
        let storage = create_storage();

        storage.put("users", "alice", json!({})).unwrap();
        storage.put("users", "bob", json!({})).unwrap();
        storage.put("users", "charlie", json!({})).unwrap();
        storage.put("sessions", "s1", json!({})).unwrap();

        let user_keys = storage.list_keys("users");
        assert_eq!(user_keys, vec!["alice", "bob", "charlie"]);

        let session_keys = storage.list_keys("sessions");
        assert_eq!(session_keys, vec!["s1"]);
    }

    #[test]
    fn test_concurrent_writes() {
        let storage = Arc::new(create_storage());
        let mut handles = vec![];

        // Spawn 10 threads writing to different keys
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let handle = thread::spawn(move || {
                storage_clone
                    .put("concurrent", format!("key{}", i), json!(i))
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // All keys should exist
        assert_eq!(storage.key_count(), 10);

        // Causal graph should have 10 nodes
        assert_eq!(storage.total_version_count(), 10);
    }
}
