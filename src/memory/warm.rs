/// Chronicle Agent: Recent history layer with LCA architecture.
///
/// The Chronicle Agent acts like the hippocampus - recent episodic memory,
/// full detail, stored on disk. Data here was recently in Temperature but
/// got evicted due to capacity limits or time.
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Chronicle Agent's local root is `RootType::Chronicle` (📜 CHRONICLE).
///
/// ## Purpose
///
/// - Store recent history that's not in Temperature layer
/// - Provide full causal chain for time travel
/// - Serve as staging area before Archive consolidation
/// - Keep recent data accessible without RAM pressure
///
/// ## Persistence
///
/// Chronicle is disk-backed. Chronicle files are append-only
/// for durability. Index is in memory for fast lookup.
use crate::actions::ChronicleAction;
use crate::causal_graph::DistinctionId;
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;
use crate::types::{FullKey, VersionedValue};
#[cfg(test)]
use crate::types::VectorClock;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Chronicle agent configuration.
#[derive(Debug, Clone)]
pub struct ChronicleConfig {
    /// Maximum number of recent distinctions to keep in index
    pub index_capacity: usize,

    /// Idle time before considering for demotion to Archive
    pub idle_threshold: Duration,

    /// Chronicle file rotation size (bytes)
    pub rotation_size: usize,
}

impl Default for ChronicleConfig {
    fn default() -> Self {
        Self {
            index_capacity: 10_000,             // Keep 10K recent in index
            idle_threshold: Duration::hours(1), // Idle 1 hour → Archive candidate
            rotation_size: 10 * 1024 * 1024,    // 10MB files
        }
    }
}

/// Chronicle Agent - recent history with LCA architecture.
///
/// Like the hippocampus: recent, detailed, on disk (not RAM).
/// All operations are synthesized through the unified field.
pub struct ChronicleAgent {
    /// Configuration
    config: ChronicleConfig,

    /// LCA: Local root distinction (Root: CHRONICLE)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// In-memory index: distinction_id → (key, timestamp)
    /// For fast lookup without scanning disk
    index: DashMap<DistinctionId, IndexEntry>,

    /// Recent access window (for promotion to Temperature)
    /// Distinctions accessed recently are hot candidates
    recent_window: std::sync::Mutex<VecDeque<(DistinctionId, DateTime<Utc>)>>,

    /// Key → current distinction mapping (for quick lookup)
    current_mappings: DashMap<FullKey, DistinctionId>,

    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    promotions: AtomicU64,
    demotions: AtomicU64,
}

/// Index entry for fast lookup.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct IndexEntry {
    /// The key this distinction belongs to
    key: FullKey,
    /// When this distinction was created
    timestamp: DateTime<Utc>,
    /// When last accessed
    last_accessed: DateTime<Utc>,
}

impl ChronicleAgent {
    /// Create new chronicle agent with default configuration.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Chronicle (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self::with_config(ChronicleConfig::default(), shared_engine)
    }

    /// Create new chronicle agent with custom configuration.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the CHRONICLE root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(config: ChronicleConfig, shared_engine: &SharedEngine) -> Self {
        let capacity = config.index_capacity;
        let local_root = shared_engine.root(RootType::Chronicle).clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            config,
            local_root,
            field,
            index: DashMap::with_capacity(capacity),
            recent_window: std::sync::Mutex::new(VecDeque::with_capacity(capacity)),
            current_mappings: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            promotions: AtomicU64::new(0),
            demotions: AtomicU64::new(0),
        }
    }

    /// Get a value from chronicle.
    ///
    /// Returns the versioned value if found, updates access time.
    /// Returns None if not in chronicle (might be in Archive or doesn't exist).
    ///
    /// # LCA Pattern
    ///
    /// Recall is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔRecall_Action`
    pub fn get(&self, id: &DistinctionId) -> Option<(FullKey, VersionedValue)> {
        let _entry = self.index.get(id)?;

        // Synthesize recall action
        let action = ChronicleAction::Recall {
            query: id.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        // Update access time
        self.update_access_time(id);
        self.hits.fetch_add(1, Ordering::Relaxed);

        // Placeholder: in real impl, would read from disk
        // For testing, we need to store actual values somewhere
        None
    }

    /// Get by key - find current version for this key.
    pub fn get_by_key(&self, key: &FullKey) -> Option<DistinctionId> {
        self.current_mappings.get(key).map(|e| e.clone())
    }

    /// Put a value into chronicle (from Temperature eviction).
    ///
    /// Writes to disk chronicle, updates index.
    ///
    /// # LCA Pattern
    ///
    /// Record is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔRecord_Action`
    pub fn put(&self, key: FullKey, versioned: VersionedValue) {
        let id = versioned.write_id().to_string();
        let timestamp = versioned.timestamp;

        // Synthesize record action
        let action = ChronicleAction::Record {
            event_id: id.clone(),
            timestamp,
        };
        let _ = self.synthesize_action_internal(action);

        // Update current mapping
        self.current_mappings.insert(key.clone(), id.clone());

        // Check if we need to make room in index
        if self.index.len() >= self.config.index_capacity {
            self.evict_oldest_index_entry();
        }

        // Add to index
        self.index.insert(
            id.clone(),
            IndexEntry {
                key,
                timestamp,
                last_accessed: Utc::now(),
            },
        );

        // Add to recent window
        self.add_to_recent_window(id);

        // TODO: Append to disk chronicle
    }

    /// Check if a distinction is in chronicle.
    pub fn contains(&self, id: &DistinctionId) -> bool {
        self.index.contains_key(id)
    }

    /// Check if a key has a current mapping in chronicle.
    pub fn contains_key(&self, key: &FullKey) -> bool {
        self.current_mappings.contains_key(key)
    }

    /// Get number of entries in index.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Get current index capacity.
    pub fn capacity(&self) -> usize {
        self.config.index_capacity
    }

    /// Get statistics.
    pub fn stats(&self) -> ChronicleStats {
        ChronicleStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            promotions: self.promotions.load(Ordering::Relaxed),
            demotions: self.demotions.load(Ordering::Relaxed),
            current_size: self.len(),
            capacity: self.config.index_capacity,
        }
    }

    /// Find distinctions that should be promoted to Temperature.
    ///
    /// Based on recent access patterns.
    pub fn find_promotion_candidates(&self, limit: usize) -> Vec<(FullKey, DistinctionId)> {
        let Ok(recent) = self.recent_window.lock() else {
            return Vec::new();
        };

        let mut candidates = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Look at recent accesses
        for (id, _timestamp) in recent.iter().rev().take(limit * 2) {
            if seen.insert(id.clone()) {
                if let Some(entry) = self.index.get(id) {
                    candidates.push((entry.key.clone(), id.clone()));
                    if candidates.len() >= limit {
                        break;
                    }
                }
            }
        }

        candidates
    }

    /// Find distinctions that should be demoted to Archive.
    ///
    /// Based on idle time (not accessed recently).
    pub fn find_demotion_candidates(&self, limit: usize) -> Vec<DistinctionId> {
        let now = Utc::now();
        let threshold = self.config.idle_threshold;

        let mut candidates: Vec<_> = self
            .index
            .iter()
            .filter_map(|entry| {
                let idle_time = now.signed_duration_since(entry.last_accessed);
                if idle_time > threshold {
                    Some((entry.key().clone(), idle_time))
                } else {
                    None
                }
            })
            .collect();

        // Sort by idle time (most idle first)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        candidates
            .into_iter()
            .take(limit)
            .map(|(id, _)| id)
            .collect()
    }

    /// Promote a distinction to Temperature (remove from Chronicle index).
    ///
    /// # LCA Pattern
    ///
    /// Promote is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔPromote_Action`
    pub fn promote(&self, id: &DistinctionId) {
        // Synthesize promote action
        let action = ChronicleAction::Promote {
            distinction_id: id.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        self.index.remove(id);
        self.promotions.fetch_add(1, Ordering::Relaxed);
        // Note: actual value would be returned and put in Temperature
    }

    /// Demote a distinction to Archive (remove from index, keep on disk).
    ///
    /// # LCA Pattern
    ///
    /// Demote is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔDemote_Action`
    pub fn demote(&self, id: &DistinctionId) {
        // Synthesize demote action
        let action = ChronicleAction::Demote {
            distinction_id: id.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        self.index.remove(id);
        self.demotions.fetch_add(1, Ordering::Relaxed);
        // Note: still on disk, just not in fast index
    }

    /// Update access time for a distinction.
    fn update_access_time(&self, id: &DistinctionId) {
        if let Some(mut entry) = self.index.get_mut(id) {
            entry.last_accessed = Utc::now();
        }
        self.add_to_recent_window(id.clone());
    }

    /// Add to recent access window.
    fn add_to_recent_window(&self, id: DistinctionId) {
        if let Ok(mut window) = self.recent_window.lock() {
            window.push_back((id, Utc::now()));

            // Trim if too large
            while window.len() > self.config.index_capacity {
                window.pop_front();
            }
        }
    }

    /// Evict oldest index entry (not the data, just the index).
    fn evict_oldest_index_entry(&self) {
        // Find oldest by last_accessed
        let oldest = self
            .index
            .iter()
            .min_by_key(|entry| entry.last_accessed)
            .map(|entry| entry.key().clone());

        if let Some(id) = oldest {
            self.index.remove(&id);
        }
    }

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: ChronicleAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        engine.synthesize(&self.local_root, &action_distinction)
    }
}

impl Default for ChronicleAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use ChronicleAgent::new(&shared_engine)
        panic!("ChronicleAgent requires a SharedEngine - use ChronicleAgent::new()")
    }
}

/// LCA Trait Implementation for ChronicleAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for ChronicleAgent {
    type ActionData = ChronicleAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: ChronicleAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

/// Chronicle agent statistics.
#[derive(Debug, Clone)]
pub struct ChronicleStats {
    pub hits: u64,
    pub misses: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub current_size: usize,
    pub capacity: usize,
}

impl ChronicleStats {
    /// Calculate hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate utilization (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.current_size as f64 / self.capacity as f64
        }
    }
}

/// Backward-compatible type alias for existing code.
pub type WarmMemory = ChronicleAgent;

/// Backward-compatible type alias for existing code.
pub type WarmConfig = ChronicleConfig;

/// Backward-compatible type alias for existing code.
pub type WarmStats = ChronicleStats;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Arc;

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

    fn create_versioned(value: serde_json::Value, id: &str) -> VersionedValue {
        VersionedValue::new(
            Arc::new(value),
            Utc::now(),
            id.to_string(), // write_id
            id.to_string(), // distinction_id
            None,
            VectorClock::new(),
        )
    }

    #[test]
    fn test_put_and_contains() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::new(&engine);
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({"name": "Alice"}), "v1");

        chronicle.put(key.clone(), versioned);

        assert!(chronicle.contains(&"v1".to_string()));
        assert!(chronicle.contains_key(&key));
        assert_eq!(chronicle.len(), 1);
    }

    #[test]
    fn test_get_by_key() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::new(&engine);
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");

        chronicle.put(key.clone(), versioned);

        let id = chronicle.get_by_key(&key).unwrap();
        assert_eq!(id, "v1");
    }

    #[test]
    fn test_capacity_and_eviction() {
        let config = ChronicleConfig {
            index_capacity: 3,
            idle_threshold: Duration::hours(1),
            rotation_size: 10_000_000,
        };
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::with_config(config, &engine);

        // Add 5 items (capacity is 3)
        for i in 0..5 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            chronicle.put(key, versioned);
        }

        // Should still be at capacity
        assert_eq!(chronicle.len(), 3);
    }

    #[test]
    fn test_stats() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::with_config(
            ChronicleConfig {
                index_capacity: 100,
                idle_threshold: Duration::hours(1),
                rotation_size: 10_000_000,
            },
            &engine,
        );

        // Add items
        for i in 0..10 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            chronicle.put(key, versioned);
        }

        let stats = chronicle.stats();
        assert_eq!(stats.current_size, 10);
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.utilization(), 0.1);
    }

    #[test]
    fn test_update_existing_key() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::new(&engine);
        let key = FullKey::new("users", "alice");

        let v1 = create_versioned(json!({"v": 1}), "v1");
        let v2 = create_versioned(json!({"v": 2}), "v2");

        chronicle.put(key.clone(), v1);
        chronicle.put(key.clone(), v2);

        // Current mapping should be v2
        assert_eq!(chronicle.get_by_key(&key).unwrap(), "v2");

        // Both versions should be in index
        assert!(chronicle.contains(&"v1".to_string()));
        assert!(chronicle.contains(&"v2".to_string()));
    }

    #[test]
    fn test_promote_and_demote() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::new(&engine);
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");

        chronicle.put(key, versioned);
        assert_eq!(chronicle.len(), 1);

        // Promote (remove from chronicle)
        chronicle.promote(&"v1".to_string());
        assert_eq!(chronicle.len(), 0);
        assert_eq!(chronicle.stats().promotions, 1);

        // Add again and demote
        let key2 = FullKey::new("users", "bob");
        let versioned2 = create_versioned(json!({}), "v2");
        chronicle.put(key2, versioned2);

        chronicle.demote(&"v2".to_string());
        assert_eq!(chronicle.len(), 0);
        assert_eq!(chronicle.stats().demotions, 1);
    }

    #[test]
    fn test_find_promotion_candidates() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::new(&engine);

        // Add some items
        for i in 0..5 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            chronicle.put(key, versioned);
        }

        // Access some to make them "recent"
        for i in 0..3 {
            chronicle.add_to_recent_window(format!("v{}", i));
        }

        let candidates = chronicle.find_promotion_candidates(10);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_is_empty() {
        let engine = create_test_engine();
        let chronicle = ChronicleAgent::new(&engine);
        assert!(chronicle.is_empty());

        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");
        chronicle.put(key, versioned);

        assert!(!chronicle.is_empty());
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = ChronicleAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = ChronicleAction::Record {
            event_id: "test123".to_string(),
            timestamp: Utc::now(),
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
    fn test_backward_compatible_aliases() {
        // Ensure backward compatibility works
        let engine = create_test_engine();
        let _warm_memory: WarmMemory = ChronicleAgent::new(&engine);
        let _config: WarmConfig = ChronicleConfig::default();
        let engine2 = create_test_engine();
        let agent = ChronicleAgent::with_config(_config, &engine2);
        let _stats: WarmStats = agent.stats();
    }
}
