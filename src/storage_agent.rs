//! Storage Agent - LCA-compliant storage with synthesis-based operations.
//!
//! This module implements the StorageAgent following the Local Causal Agent pattern.
//! All storage operations are synthesized: `ΔNew = ΔLocal_Root ⊕ ΔStorageAction`
//!
//! # LCA Architecture
//!
//! The StorageAgent is a Local Causal Agent with:
//! - `local_root`: RootType::Storage (canonical storage root)
//! - `ActionData`: StorageAction (Store, Retrieve, History, Query, Delete)
//! - All operations synthesize before mutating state
//!
//! # Emergent Behavior
//!
//! Through synthesis, the storage captures:
//! - Causal graph: How writes emerge from previous writes
//! - Reference graph: What values reference what
//! - Content addressing: Same content = same distinction
//! - Time travel: Full history preserved via synthesis chain

use crate::actions::StorageAction;
use crate::causal_graph::LineageAgent;
use crate::engine::{FieldHandle, SharedEngine};
use crate::error::{DeltaError, DeltaResult};
use crate::mapper::DocumentMapper;
use crate::reference_graph::ReferenceGraph;
use crate::roots::RootType;
use crate::types::{
    FullKey, HistoryEntry, Tombstone, VectorClock, VersionedValue,
};
use chrono::Utc;
use dashmap::DashMap;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Storage Agent implementing LCA pattern for all storage operations.
///
/// All mutations happen through synthesis: `ΔNew = ΔLocal_Root ⊕ ΔStorageAction`
#[derive(Debug)]
pub struct StorageAgent {
    /// LCA: Local root distinction (Root: STORAGE)
    local_root: Distinction,

    /// LCA: Handle to the unified field
    _field: FieldHandle,

    /// The underlying distinction engine for content addressing
    engine: Arc<DistinctionEngine>,

    /// Causal graph: tracks how distinctions emerge from one another
    causal_graph: LineageAgent,

    /// Reference graph: tracks what distinctions reference what
    reference_graph: ReferenceGraph,

    /// Current (latest) value for each key
    /// Maps FullKey → VersionedValue
    current_state: DashMap<FullKey, VersionedValue>,

    /// All versions (for history and time travel)
    /// Maps version_id → VersionedValue
    version_store: DashMap<String, VersionedValue>,

    /// Deduplicated value storage
    /// Maps version_id → `Arc<JsonValue>`
    value_store: DashMap<String, Arc<JsonValue>>,

    /// Tombstones for deleted keys
    /// Maps FullKey → Tombstone
    tombstones: DashMap<FullKey, Tombstone>,
}

impl StorageAgent {
    /// Create a new storage agent with LCA initialization.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Storage (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn new(shared_engine: &SharedEngine) -> Self {
        let local_root = shared_engine.root(RootType::Storage).clone();
        let _field = FieldHandle::new(shared_engine);
        let engine = Arc::clone(shared_engine.inner());

        Self {
            local_root,
            _field,
            engine,
            causal_graph: LineageAgent::new(shared_engine),
            reference_graph: ReferenceGraph::new(),
            current_state: DashMap::new(),
            version_store: DashMap::new(),
            value_store: DashMap::new(),
            tombstones: DashMap::new(),
        }
    }

    /// Get the current local root.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Get a reference to the underlying distinction engine.
    pub fn engine(&self) -> Arc<DistinctionEngine> {
        Arc::clone(&self.engine)
    }

    /// Store a value, synthesizing the Store action.
    ///
    /// Formula: `ΔNew = ΔLocal_Root ⊕ ΔStore`
    pub fn put(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: JsonValue,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Synthesize the Store action
        let action = StorageAction::Store {
            namespace: namespace.clone(),
            key: key.clone(),
            value_json: value.clone(),
        };

        let _new_root = self.synthesize_action(action, &self.engine.clone());

        // Apply the store operation
        self.apply_store(&namespace, &key, value)
    }

    /// Retrieve a value, synthesizing the Retrieve action.
    ///
    /// Formula: `ΔNew = ΔLocal_Root ⊕ ΔRetrieve`
    pub fn get(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Synthesize the Retrieve action
        let action = StorageAction::Retrieve {
            namespace: namespace.clone(),
            key: key.clone(),
        };

        let _new_root = self.synthesize_action(action, &self.engine.clone());

        // Apply the retrieve operation
        self.apply_retrieve(&namespace, &key)
    }

    /// Get history for a key, synthesizing the History action.
    ///
    /// Formula: `ΔNew = ΔLocal_Root ⊕ ΔHistory`
    pub fn history(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let namespace = namespace.into();
        let key = key.into();

        // Synthesize the History action
        let action = StorageAction::History {
            namespace: namespace.clone(),
            key: key.clone(),
        };

        let _new_root = self.synthesize_action(action, &self.engine.clone());

        // Apply the history operation
        self.apply_history(&namespace, &key)
    }

    /// Delete a key, synthesizing the Delete action.
    ///
    /// Formula: `ΔNew = ΔLocal_Root ⊕ ΔDelete`
    pub fn delete(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<Tombstone> {
        let namespace = namespace.into();
        let key = key.into();

        // Synthesize the Delete action
        let action = StorageAction::Delete {
            namespace: namespace.clone(),
            key: key.clone(),
        };

        let _new_root = self.synthesize_action(action, &self.engine.clone());

        // Apply the delete operation
        self.apply_delete(&namespace, &key)
    }

    /// Query with a pattern, synthesizing the Query action.
    ///
    /// Formula: `ΔNew = ΔLocal_Root ⊕ ΔQuery`
    pub fn query(
        &mut self,
        pattern: JsonValue,
    ) -> DeltaResult<Vec<(FullKey, VersionedValue)>> {
        // Synthesize the Query action
        let action = StorageAction::Query {
            pattern_json: pattern.clone(),
        };

        let _new_root = self.synthesize_action(action, &self.engine.clone());

        // Apply the query operation
        self.apply_query(&pattern)
    }

    /// Store multiple values in a batch operation.
    ///
    /// Each item is synthesized individually to maintain causal chain.
    pub fn put_batch(
        &mut self,
        items: Vec<(String, String, JsonValue)>,
    ) -> DeltaResult<Vec<VersionedValue>> {
        let mut results = Vec::with_capacity(items.len());

        for (namespace, key, value) in items {
            let versioned = self.put(namespace, key, value)?;
            results.push(versioned);
        }

        Ok(results)
    }

    /// Check if a key exists.
    pub fn contains_key(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> bool {
        let full_key = FullKey::new(namespace, key);
        self.current_state.contains_key(&full_key)
    }

    /// Get the number of keys in storage.
    pub fn key_count(&self) -> usize {
        self.current_state.len()
    }

    /// Get total version count.
    pub fn total_version_count(&self) -> usize {
        self.version_store.len()
    }

    /// List all namespaces.
    pub fn list_namespaces(&self) -> Vec<String> {
        let mut namespaces: std::collections::HashSet<String> = self
            .current_state
            .iter()
            .map(|entry| entry.key().namespace.clone())
            .collect();

        let mut result: Vec<String> = namespaces.drain().collect();
        result.sort();
        result
    }

    /// List keys in a namespace.
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

    /// Scan all entries in a namespace.
    pub fn scan_collection(&self, namespace: &str) -> Vec<(String, VersionedValue)> {
        let mut results: Vec<(String, VersionedValue)> = self
            .current_state
            .iter()
            .filter(|entry| entry.key().namespace == namespace)
            .map(|entry| (entry.key().key.clone(), entry.value().clone()))
            .collect();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    /// Scan all entries.
    pub fn scan_all(&self) -> Vec<(FullKey, VersionedValue)> {
        let mut results: Vec<(FullKey, VersionedValue)> = self
            .current_state
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        results.sort_by(|a, b| {
            let ns_cmp = a.0.namespace.cmp(&b.0.namespace);
            if ns_cmp == std::cmp::Ordering::Equal {
                a.0.key.cmp(&b.0.key)
            } else {
                ns_cmp
            }
        });
        results
    }

    /// Get reference to causal graph.
    pub fn causal_graph(&self) -> &LineageAgent {
        &self.causal_graph
    }

    /// Get reference to reference graph.
    pub fn reference_graph(&self) -> &ReferenceGraph {
        &self.reference_graph
    }

    /// Get tombstone for a key.
    pub fn get_tombstone(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> Option<Tombstone> {
        let full_key = FullKey::new(namespace, key);
        self.tombstones.get(&full_key).map(|t| t.clone())
    }

    /// Check if a key has a tombstone.
    pub fn has_tombstone(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> bool {
        let full_key = FullKey::new(namespace, key);
        self.tombstones.contains_key(&full_key)
    }

    /// Get all tombstones.
    pub fn get_all_tombstones(&self) -> Vec<Tombstone> {
        self.tombstones.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Insert a tombstone directly (for persistence replay).
    pub fn insert_tombstone(&self, tombstone: Tombstone) {
        let full_key = tombstone.key.clone();
        self.tombstones.insert(full_key, tombstone);
    }

    // ============================================================================
    // INTERNAL ACTION APPLICATION
    // ============================================================================
    // These methods apply actions after synthesis. They are private because
    // all external access should go through synthesize_action().

    /// Apply a Store action to the state.
    fn apply_store(
        &self,
        namespace: &str,
        key: &str,
        value: JsonValue,
    ) -> DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);
        let timestamp = Utc::now();

        // Get previous version if it exists (causal parent)
        let previous_version = self
            .current_state
            .get(&full_key)
            .map(|v| v.write_id.clone());

        // Compute distinction via koru-lambda-core
        let distinction = DocumentMapper::json_to_distinction(&value, &self.engine)?;
        let distinction_id = DocumentMapper::store_distinction_id(&distinction);

        // Generate unique write ID
        let write_id = format!(
            "{}_{}",
            distinction_id,
            timestamp.timestamp_nanos_opt().unwrap_or(0)
        );

        // Capture in causal graph
        self.causal_graph.add_node(write_id.clone());
        if let Some(ref parent_id) = previous_version {
            self.causal_graph
                .add_edge(parent_id.clone(), write_id.clone());
        }

        // Capture in reference graph
        self.reference_graph.add_node(write_id.clone());

        // Get or create shared value from the value store (deduplication)
        let shared_value = self
            .value_store
            .entry(distinction_id.clone())
            .or_insert_with(|| Arc::new(value))
            .clone();

        // Create new versioned value
        let versioned = VersionedValue::new(
            shared_value,
            timestamp,
            write_id.clone(),
            distinction_id,
            previous_version,
            VectorClock::new(),
        );

        // Store in version store
        self.version_store
            .insert(write_id.clone(), versioned.clone());

        // Update current state
        self.current_state
            .insert(full_key.clone(), versioned.clone());

        Ok(versioned)
    }

    /// Apply a Retrieve action.
    fn apply_retrieve(
        &self,
        namespace: &str,
        key: &str,
    ) -> DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);

        // Check for tombstone first
        if self.tombstones.get(&full_key).is_some() {
            return Err(DeltaError::KeyNotFound {
                namespace: namespace.to_string(),
                key: key.to_string(),
            });
        }

        // Get current value
        match self.current_state.get(&full_key) {
            Some(versioned) => Ok(versioned.clone()),
            None => Err(DeltaError::KeyNotFound {
                namespace: namespace.to_string(),
                key: key.to_string(),
            }),
        }
    }

    /// Apply a History action.
    fn apply_history(
        &self,
        namespace: &str,
        key: &str,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let full_key = FullKey::new(namespace, key);

        // Get current version
        let current = match self.current_state.get(&full_key) {
            Some(v) => v.clone(),
            None => {
                return Err(DeltaError::KeyNotFound {
                    namespace: namespace.to_string(),
                    key: key.to_string(),
                })
            }
        };

        // Build history by traversing previous_version chain
        let mut history = Vec::new();
        let mut current_write_id = Some(current.write_id.clone());

        while let Some(write_id) = current_write_id {
            if let Some(versioned) = self.version_store.get(&write_id) {
                history.push(HistoryEntry {
                    timestamp: versioned.timestamp,
                    value: (*versioned.value).clone(),
                    version_id: write_id.clone(),
                });
                current_write_id = versioned.previous_version.clone();
            } else {
                break;
            }
        }

        // Reverse to get chronological order
        history.reverse();
        Ok(history)
    }

    /// Apply a Delete action.
    fn apply_delete(
        &self,
        namespace: &str,
        key: &str,
    ) -> DeltaResult<Tombstone> {
        let full_key = FullKey::new(namespace, key);

        // Create tombstone
        let tombstone = Tombstone::new(
            full_key.clone(),
            "storage_agent", // deleted_by
            VectorClock::new(),
        );

        // Remove from current state
        self.current_state.remove(&full_key);

        // Add tombstone
        self.tombstones.insert(full_key, tombstone.clone());

        Ok(tombstone)
    }

    /// Apply a Query action.
    fn apply_query(
        &self,
        _pattern: &JsonValue,
    ) -> DeltaResult<Vec<(FullKey, VersionedValue)>> {
        // Basic implementation: return all entries
        // TODO: Implement actual pattern matching
        let results: Vec<(FullKey, VersionedValue)> = self
            .current_state
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        Ok(results)
    }
}

// ============================================================================
// LCA TRAIT IMPLEMENTATION
// ============================================================================

impl LocalCausalAgent for StorageAgent {
    type ActionData = StorageAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: StorageAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

// ============================================================================
// BACKWARD COMPATIBILITY
// ============================================================================



#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup_agent() -> StorageAgent {
        let shared_engine = SharedEngine::new();
        StorageAgent::new(&shared_engine)
    }

    #[test]
    fn test_storage_agent_implements_lca_trait() {
        let agent = setup_agent();
        
        // Verify trait is implemented
        let _root = agent.get_current_root();
    }

    #[test]
    fn test_put_synthesizes() {
        let mut agent = setup_agent();
        let root_before = agent.local_root().id().to_string();

        let result = agent.put("test", "key1", json!({"data": "value1"}));
        assert!(result.is_ok());

        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after synthesis");
    }

    #[test]
    fn test_get_synthesizes() {
        let mut agent = setup_agent();
        
        // First put a value
        agent.put("test", "key1", json!({"data": "value1"})).unwrap();
        let root_before = agent.local_root().id().to_string();

        // Then get it
        let result = agent.get("test", "key1");
        assert!(result.is_ok());

        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after get synthesis");
    }

    #[test]
    fn test_delete_synthesizes() {
        let mut agent = setup_agent();
        
        agent.put("test", "key1", json!({"data": "value1"})).unwrap();
        let root_before = agent.local_root().id().to_string();

        let result = agent.delete("test", "key1");
        assert!(result.is_ok());

        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after delete synthesis");
    }

    #[test]
    fn test_history_synthesizes() {
        let mut agent = setup_agent();
        
        agent.put("test", "key1", json!({"data": "value1"})).unwrap();
        let root_before = agent.local_root().id().to_string();

        let result = agent.history("test", "key1");
        assert!(result.is_ok());

        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after history synthesis");
    }

    #[test]
    fn test_basic_crud() {
        let mut agent = setup_agent();

        // Create
        let versioned = agent.put("test", "key1", json!({"data": "value1"})).unwrap();
        assert_eq!(versioned.value()["data"], "value1");

        // Read
        let retrieved = agent.get("test", "key1").unwrap();
        assert_eq!(retrieved.value()["data"], "value1");

        // Update
        let versioned2 = agent.put("test", "key1", json!({"data": "value2"})).unwrap();
        assert_eq!(versioned2.value()["data"], "value2");

        // History
        let history = agent.history("test", "key1").unwrap();
        assert_eq!(history.len(), 2);

        // Delete
        agent.delete("test", "key1").unwrap();
        assert!(agent.get("test", "key1").is_err());
    }

    #[test]
    fn test_contains_key() {
        let mut agent = setup_agent();
        
        assert!(!agent.contains_key("test", "key1"));
        
        agent.put("test", "key1", json!({"data": "value1"})).unwrap();
        
        assert!(agent.contains_key("test", "key1"));
    }

    #[test]
    fn test_list_namespaces() {
        let mut agent = setup_agent();
        
        agent.put("ns1", "key1", json!("value1")).unwrap();
        agent.put("ns2", "key1", json!("value2")).unwrap();
        
        let namespaces = agent.list_namespaces();
        assert_eq!(namespaces.len(), 2);
        assert!(namespaces.contains(&"ns1".to_string()));
        assert!(namespaces.contains(&"ns2".to_string()));
    }

    #[test]
    fn test_tombstone_prevents_reappearance() {
        let mut agent = setup_agent();
        
        agent.put("test", "key1", json!({"data": "value1"})).unwrap();
        agent.delete("test", "key1").unwrap();
        
        // Should return error with tombstone info
        let result = agent.get("test", "key1");
        assert!(result.is_err());
        assert!(agent.has_tombstone("test", "key1"));
    }
}
