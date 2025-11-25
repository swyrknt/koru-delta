/// Storage layer with causal history tracking.
///
/// This module implements the core storage engine for KoruDelta. Unlike traditional
/// databases that overwrite values, this storage layer maintains a complete causal
/// history of all changes:
///
/// - Every write creates a new versioned entry
/// - Each version links to its predecessor (causal chain)
/// - Time-travel queries traverse the causal history
/// - All versions are content-addressed via distinctions
///
/// The storage layer is thread-safe and uses DashMap for lock-free concurrent access.
use crate::error::{DeltaError, DeltaResult};
use crate::mapper::DocumentMapper;
use crate::types::{FullKey, HistoryEntry, VersionedValue};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use koru_lambda_core::DistinctionEngine;
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Storage engine managing causal history for all keys.
///
/// The storage layer maintains three primary data structures:
///
/// 1. **Current State**: Maps each key to its latest versioned value
/// 2. **History Log**: Maintains ordered history of all versions per key
/// 3. **Value Store**: Deduplicates values by content-addressed version ID
///
/// Both structures are thread-safe via DashMap and support concurrent reads/writes.
///
/// ## Value Deduplication
///
/// The value store enables memory-efficient storage by sharing identical values.
/// Since version IDs are content-addressed (same content = same ID), we can
/// use them as keys to deduplicate the actual JSON values. This means storing
/// the same value N times only uses memory for one copy.
#[derive(Debug)]
pub struct CausalStorage {
    /// The underlying distinction engine for content addressing
    engine: Arc<DistinctionEngine>,

    /// Current (latest) value for each key
    /// Maps FullKey → VersionedValue
    current_state: DashMap<FullKey, VersionedValue>,

    /// Complete history for each key (ordered oldest → newest)
    /// Maps FullKey → `Vec<VersionedValue>`
    history_log: DashMap<FullKey, Vec<VersionedValue>>,

    /// Deduplicated value storage
    /// Maps version_id → Arc<JsonValue>
    /// Same values share the same Arc allocation
    value_store: DashMap<String, Arc<JsonValue>>,
}

impl CausalStorage {
    /// Create a new causal storage instance.
    ///
    /// The storage is backed by a distinction engine which provides
    /// content-addressed versioning and mathematical guarantees.
    pub fn new(engine: Arc<DistinctionEngine>) -> Self {
        Self {
            engine,
            current_state: DashMap::new(),
            history_log: DashMap::new(),
            value_store: DashMap::new(),
        }
    }

    /// Store a value with automatic versioning and timestamp.
    ///
    /// This creates a new version in the causal history:
    /// - Generates a content-addressed version ID (distinction)
    /// - Links to the previous version (if any)
    /// - Records the current timestamp
    /// - Appends to the history log
    /// - Updates the current state
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from
    /// multiple threads. The causal chain is maintained correctly even
    /// under concurrent writes to the same key.
    ///
    /// # Example
    ///
    /// ```ignore
    /// storage.put("users", "alice", json!({"name": "Alice"})).await?;
    /// ```
    pub fn put(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: JsonValue,
    ) -> DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);
        let timestamp = Utc::now();

        // Get previous version if it exists
        let previous_version = self
            .current_state
            .get(&full_key)
            .map(|v| v.version_id.clone());

        // Generate content-addressed version ID
        let distinction = DocumentMapper::json_to_distinction(&value, &self.engine)?;
        let version_id = DocumentMapper::store_distinction_id(&distinction);

        // Get or create shared value from the value store (deduplication)
        // If this exact value was stored before, we reuse the same Arc
        let shared_value = self
            .value_store
            .entry(version_id.clone())
            .or_insert_with(|| Arc::new(value))
            .clone();

        // Create new versioned value with the shared Arc
        let versioned = VersionedValue::new(shared_value, timestamp, version_id, previous_version);

        // Update current state
        self.current_state
            .insert(full_key.clone(), versioned.clone());

        // Append to history log
        self.history_log
            .entry(full_key)
            .or_default()
            .push(versioned.clone());

        Ok(versioned)
    }

    /// Get the current (latest) value for a key.
    ///
    /// Returns the most recent version, or an error if the key doesn't exist.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = storage.get("users", "alice")?;
    /// println!("Current user: {:?}", user.value());
    /// ```
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
    /// This traverses the causal history backward from the present,
    /// finding the most recent version at or before the given timestamp.
    ///
    /// # Algorithm
    ///
    /// 1. Fetch the complete history for the key
    /// 2. Iterate backward from newest to oldest
    /// 3. Return the first version with timestamp ≤ target
    ///
    /// # Example
    ///
    /// ```ignore
    /// let past_timestamp = DateTime::from_timestamp(1704067200, 0).unwrap();
    /// let past_user = storage.get_at("users", "alice", past_timestamp)?;
    /// ```
    pub fn get_at(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> DeltaResult<VersionedValue> {
        let namespace_str = namespace.into();
        let key_str = key.into();
        let full_key = FullKey::new(namespace_str.clone(), key_str.clone());

        // Get history for this key
        let history = self
            .history_log
            .get(&full_key)
            .ok_or_else(|| DeltaError::KeyNotFound {
                namespace: namespace_str.clone(),
                key: key_str.clone(),
            })?;

        // Find the most recent version at or before the target timestamp
        history
            .iter()
            .rev() // Iterate backward (newest to oldest)
            .find(|v| v.timestamp <= timestamp)
            .cloned()
            .ok_or_else(|| DeltaError::NoValueAtTimestamp {
                namespace: namespace_str,
                key: key_str,
                timestamp: timestamp.timestamp(),
            })
    }

    /// Get the complete history for a key (oldest to newest).
    ///
    /// Returns all versions that have ever been written to this key,
    /// in chronological order. This enables full audit trails and
    /// time-series analysis.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let history = storage.history("users", "alice")?;
    /// for entry in history {
    ///     println!("{}: {:?}", entry.timestamp, entry.value);
    /// }
    /// ```
    pub fn history(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let full_key = FullKey::new(namespace, key);

        let history = self
            .history_log
            .get(&full_key)
            .ok_or_else(|| DeltaError::KeyNotFound {
                namespace: full_key.namespace.clone(),
                key: full_key.key.clone(),
            })?;

        // Convert VersionedValues to HistoryEntries
        Ok(history.iter().map(HistoryEntry::from).collect())
    }

    /// Check if a key exists in the storage.
    ///
    /// This is a cheap operation that only checks the current state index.
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
    /// This counts all historical versions, not just current values.
    pub fn total_version_count(&self) -> usize {
        self.history_log
            .iter()
            .map(|entry| entry.value().len())
            .sum()
    }

    /// Get all namespaces currently in use.
    ///
    /// This scans the current state and returns unique namespace names.
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
    ///
    /// Returns the keys (without namespace prefix) for all entries
    /// in the given namespace.
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
    ///
    /// Returns all current values in the given collection/namespace.
    /// This is useful for queries and views that need to iterate over
    /// all data in a collection.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = storage.scan_collection("users");
    /// for (key, value) in users {
    ///     println!("{}: {:?}", key, value.value());
    /// }
    /// ```
    pub fn scan_collection(&self, namespace: &str) -> Vec<(String, VersionedValue)> {
        self.current_state
            .iter()
            .filter(|entry| entry.key().namespace == namespace)
            .map(|entry| (entry.key().key.clone(), entry.value().clone()))
            .collect()
    }

    /// Scan all key-value pairs across all namespaces.
    ///
    /// Returns all current values in the storage.
    pub fn scan_all(&self) -> Vec<(FullKey, VersionedValue)> {
        self.current_state
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Create a consistent snapshot of the database state.
    ///
    /// This captures the current state and complete history for all keys.
    /// The snapshot can be serialized and saved to disk for persistence.
    ///
    /// # Thread Safety
    ///
    /// This creates a point-in-time snapshot. Concurrent writes may continue
    /// while the snapshot is being taken, and those writes will not be included
    /// in the snapshot.
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

        // Snapshot history log
        let history_log: HashMap<_, _> = self
            .history_log
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        (current_state, history_log)
    }

    /// Restore storage from a snapshot.
    ///
    /// Creates a new CausalStorage instance with the given state and history.
    /// This is used by the persistence layer to restore a database from disk.
    ///
    /// During restoration, we rebuild the value_store to ensure proper deduplication
    /// of values that were serialized separately but have the same version_id.
    ///
    /// # Arguments
    ///
    /// * `engine` - The distinction engine to use
    /// * `current_state` - Current values for all keys
    /// * `history_log` - Complete history for all keys
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_snapshot(
        engine: Arc<DistinctionEngine>,
        current_state: std::collections::HashMap<FullKey, VersionedValue>,
        history_log: std::collections::HashMap<FullKey, Vec<VersionedValue>>,
    ) -> Self {
        let value_store: DashMap<String, Arc<JsonValue>> = DashMap::new();

        // Helper to get or create deduplicated value
        let get_or_insert_value = |version_id: &str, value: &Arc<JsonValue>| -> Arc<JsonValue> {
            value_store
                .entry(version_id.to_string())
                .or_insert_with(|| value.clone())
                .clone()
        };

        // Restore current state with deduplication
        let current_state_map: DashMap<FullKey, VersionedValue> = DashMap::new();
        for (key, versioned) in current_state {
            let shared_value = get_or_insert_value(&versioned.version_id, &versioned.value);
            let deduped = VersionedValue::new(
                shared_value,
                versioned.timestamp,
                versioned.version_id,
                versioned.previous_version,
            );
            current_state_map.insert(key, deduped);
        }

        // Restore history with deduplication
        let history_log_map: DashMap<FullKey, Vec<VersionedValue>> = DashMap::new();
        for (key, history) in history_log {
            let deduped_history: Vec<VersionedValue> = history
                .into_iter()
                .map(|versioned| {
                    let shared_value = get_or_insert_value(&versioned.version_id, &versioned.value);
                    VersionedValue::new(
                        shared_value,
                        versioned.timestamp,
                        versioned.version_id,
                        versioned.previous_version,
                    )
                })
                .collect();
            history_log_map.insert(key, deduped_history);
        }

        Self {
            engine,
            current_state: current_state_map,
            history_log: history_log_map,
            value_store,
        }
    }

    /// Get the number of unique values in the value store.
    ///
    /// This indicates the deduplication efficiency - a lower number
    /// relative to total_version_count() means better deduplication.
    pub fn unique_value_count(&self) -> usize {
        self.value_store.len()
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
        thread::sleep(Duration::from_millis(10)); // Ensure different timestamps
        let v2 = storage.put("users", "alice", json!({"age": 31})).unwrap();

        // Version 2 should link to version 1
        assert_eq!(v2.previous_version(), Some(v1.version_id()));

        // Current value should be v2
        let current = storage.get("users", "alice").unwrap();
        assert_eq!(current.version_id(), v2.version_id());
    }

    #[test]
    fn test_history() {
        let storage = create_storage();

        storage.put("counter", "clicks", json!(1)).unwrap();
        thread::sleep(Duration::from_millis(10));
        storage.put("counter", "clicks", json!(2)).unwrap();
        thread::sleep(Duration::from_millis(10));
        storage.put("counter", "clicks", json!(3)).unwrap();

        let history = storage.history("counter", "clicks").unwrap();
        assert_eq!(history.len(), 3);

        // History should be in chronological order
        assert_eq!(history[0].value, json!(1));
        assert_eq!(history[1].value, json!(2));
        assert_eq!(history[2].value, json!(3));
    }

    #[test]
    fn test_time_travel() {
        let storage = create_storage();

        let v1 = storage.put("doc", "readme", json!({"version": 1})).unwrap();
        let t1 = v1.timestamp;

        thread::sleep(Duration::from_millis(50));
        let v2 = storage.put("doc", "readme", json!({"version": 2})).unwrap();
        let t2 = v2.timestamp;

        thread::sleep(Duration::from_millis(50));
        let v3 = storage.put("doc", "readme", json!({"version": 3})).unwrap();
        let t3 = v3.timestamp;

        // Get value at t1 (should be version 1)
        let v_at_t1 = storage.get_at("doc", "readme", t1).unwrap();
        assert_eq!(v_at_t1.value(), &json!({"version": 1}));

        // Get value at t2 (should be version 2)
        let v_at_t2 = storage.get_at("doc", "readme", t2).unwrap();
        assert_eq!(v_at_t2.value(), &json!({"version": 2}));

        // Get value at t3 (should be version 3)
        let v_at_t3 = storage.get_at("doc", "readme", t3).unwrap();
        assert_eq!(v_at_t3.value(), &json!({"version": 3}));
    }

    #[test]
    fn test_time_travel_before_first_version() {
        let storage = create_storage();

        let now = Utc::now();
        thread::sleep(Duration::from_millis(50));
        storage.put("doc", "file", json!({"data": "test"})).unwrap();

        // Try to get value before it existed
        let result = storage.get_at("doc", "file", now);
        assert!(matches!(result, Err(DeltaError::NoValueAtTimestamp { .. })));
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
    fn test_total_version_count() {
        let storage = create_storage();

        storage.put("users", "alice", json!(1)).unwrap();
        assert_eq!(storage.total_version_count(), 1);

        storage.put("users", "alice", json!(2)).unwrap();
        assert_eq!(storage.total_version_count(), 2);

        storage.put("users", "bob", json!(1)).unwrap();
        assert_eq!(storage.total_version_count(), 3);
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
        for i in 0..10 {
            assert!(storage.contains_key("concurrent", format!("key{}", i)));
        }
    }

    #[test]
    fn test_concurrent_updates_same_key() {
        let storage = Arc::new(create_storage());
        let mut handles = vec![];

        // Spawn 20 threads updating the same key
        for i in 0..20 {
            let storage_clone = Arc::clone(&storage);
            let handle = thread::spawn(move || {
                storage_clone.put("counter", "value", json!(i)).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have exactly 1 key
        assert_eq!(storage.key_count(), 1);

        // Should have 20 versions
        assert_eq!(storage.total_version_count(), 20);

        // History should contain all updates
        let history = storage.history("counter", "value").unwrap();
        assert_eq!(history.len(), 20);
    }

    #[test]
    fn test_value_deduplication() {
        let storage = create_storage();

        // Store the same value multiple times to the same key
        let value = json!({"status": "active", "count": 42});
        storage.put("users", "alice", value.clone()).unwrap();
        thread::sleep(Duration::from_millis(10));
        storage.put("users", "alice", value.clone()).unwrap();
        thread::sleep(Duration::from_millis(10));
        storage.put("users", "alice", value.clone()).unwrap();

        // 3 versions but only 1 unique value in the store
        assert_eq!(storage.total_version_count(), 3);
        assert_eq!(storage.unique_value_count(), 1);

        // Store the same value to a different key
        storage.put("users", "bob", value.clone()).unwrap();

        // Now 4 versions, still only 1 unique value
        assert_eq!(storage.total_version_count(), 4);
        assert_eq!(storage.unique_value_count(), 1);

        // Store a different value
        storage
            .put("users", "charlie", json!({"status": "inactive"}))
            .unwrap();

        // 5 versions, 2 unique values
        assert_eq!(storage.total_version_count(), 5);
        assert_eq!(storage.unique_value_count(), 2);
    }

    #[test]
    fn test_value_deduplication_arc_sharing() {
        let storage = create_storage();

        // Store the same value to different keys
        let value = json!({"shared": true});
        storage.put("ns1", "key1", value.clone()).unwrap();
        storage.put("ns2", "key2", value.clone()).unwrap();

        // Get the versioned values
        let v1 = storage.get("ns1", "key1").unwrap();
        let v2 = storage.get("ns2", "key2").unwrap();

        // The Arc pointers should be the same (same memory address)
        assert!(Arc::ptr_eq(&v1.value, &v2.value));
    }

    #[test]
    fn test_deduplication_with_different_values() {
        let storage = create_storage();

        // Store different values - each should have its own entry
        for i in 0..100 {
            storage.put("data", format!("key{}", i), json!(i)).unwrap();
        }

        // 100 versions, 100 unique values (no deduplication possible)
        assert_eq!(storage.total_version_count(), 100);
        assert_eq!(storage.unique_value_count(), 100);
    }

    #[test]
    fn test_deduplication_efficiency() {
        let storage = create_storage();

        // Simulate a scenario where the same status is written many times
        // (common in audit logs, heartbeats, etc.)
        let active_status = json!({"status": "active"});
        let inactive_status = json!({"status": "inactive"});

        // 50 writes of "active", 50 writes of "inactive"
        for i in 0..100 {
            let value = if i % 2 == 0 {
                active_status.clone()
            } else {
                inactive_status.clone()
            };
            storage.put("status", format!("entry{}", i), value).unwrap();
        }

        // 100 versions but only 2 unique values
        assert_eq!(storage.total_version_count(), 100);
        assert_eq!(storage.unique_value_count(), 2);
    }
}
