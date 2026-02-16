/// Temperature Agent: Working memory layer with LCA architecture.
///
/// The Temperature Agent acts like the prefrontal cortex - fast, limited capacity,
/// holds what's immediately relevant. Data here is in RAM for instant access.
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Temperature Agent's local root is `RootType::Temperature` (🔥 HOT).
///
/// ## Purpose
///
/// - Keep frequently/recently accessed distinctions in fast RAM
/// - Automatically evict cold data to Chronicle layer
/// - Provide bounded memory usage regardless of database size
///
/// ## Eviction Policy
///
/// LRU (Least Recently Used): When cache is full, evict the item
/// that hasn't been accessed longest.
use crate::actions::{TemperatureAction, TemperatureLevel};
use crate::causal_graph::DistinctionId;
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;
#[cfg(test)]
use crate::types::VectorClock;
use crate::types::{FullKey, VersionedValue};
use dashmap::DashMap;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temperature agent configuration.
#[derive(Debug, Clone)]
pub struct TemperatureConfig {
    /// Maximum number of distinctions in hot memory
    pub capacity: usize,

    /// Promote threshold: references >= this → hot candidate
    pub promote_threshold: usize,
}

impl Default for TemperatureConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,       // Default: 1000 hot distinctions
            promote_threshold: 3, // 3+ references = hot candidate
        }
    }
}

/// Temperature Agent - working memory with LCA architecture.
///
/// Like the prefrontal cortex: fast, limited, holds current focus.
/// All operations are synthesized through the unified field.
pub struct TemperatureAgent {
    /// Configuration
    config: TemperatureConfig,

    /// LCA: Local root distinction (Root: TEMPERATURE)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// LRU cache: distinction_id → versioned value
    /// Ordered by recency (front = most recent, back = least recent)
    cache: DashMap<DistinctionId, VersionedValue>,

    /// Access order for LRU (front = most recent)
    access_order: std::sync::Mutex<VecDeque<DistinctionId>>,

    /// Current → distinction mapping for quick lookup
    current_state: DashMap<FullKey, DistinctionId>,

    /// Statistics
    hits: AtomicUsize,
    misses: AtomicUsize,
    evictions: AtomicUsize,
}

impl TemperatureAgent {
    /// Create new temperature agent with default configuration.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Temperature (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self::with_config(TemperatureConfig::default(), shared_engine)
    }

    /// Create new temperature agent with custom configuration.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the TEMPERATURE root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(config: TemperatureConfig, shared_engine: &SharedEngine) -> Self {
        let capacity = config.capacity;
        let local_root = shared_engine.root(RootType::Temperature).clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            config,
            local_root,
            field,
            cache: DashMap::with_capacity(capacity),
            access_order: std::sync::Mutex::new(VecDeque::with_capacity(capacity)),
            current_state: DashMap::new(),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            evictions: AtomicUsize::new(0),
        }
    }

    /// Get a value from hot memory.
    ///
    /// If found, promotes to most-recent position (LRU update).
    /// Returns None if not in hot memory (need to fetch from Warm).
    ///
    /// # LCA Pattern
    ///
    /// Access is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔAccess_Action`
    pub fn get(&self, key: &FullKey) -> Option<VersionedValue> {
        // Check current state mapping
        let distinction_id = match self.current_state.get(key) {
            Some(id) => id.clone(),
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        // Synthesize the access action
        let action = TemperatureAction::Access {
            distinction_id: distinction_id.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        // Check cache
        let result = self.cache.get(&distinction_id).map(|v| v.clone());

        if result.is_some() {
            // Hit! Update LRU order
            self.update_lru(distinction_id);
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            // Miss (in current_state but not in cache - should be rare)
            self.misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Get by distinction ID directly.
    pub fn get_by_id(&self, id: &DistinctionId) -> Option<VersionedValue> {
        let result = self.cache.get(id).map(|v| v.clone());

        if result.is_some() {
            // Synthesize access action
            let action = TemperatureAction::Access {
                distinction_id: id.clone(),
            };
            let _ = self.synthesize_action_internal(action);

            self.update_lru(id.clone());
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Put a value into hot memory.
    ///
    /// If at capacity, evicts least-recently-used item to make room.
    /// Updates current state mapping.
    ///
    /// # LCA Pattern
    ///
    /// Store is synthesized: `ΔNew = ΔLocal_Root ⊕ ΔHeat_Action`
    pub fn put(&self, key: FullKey, versioned: VersionedValue) -> Option<Evicted> {
        let id = versioned.write_id().to_string();

        // Check if we're updating an existing key with a new version
        if let Some(old_id) = self.current_state.get(&key) {
            if *old_id != id {
                // Remove old version from cache
                self.cache.remove(&*old_id);
                // Remove from LRU order
                if let Ok(mut order) = self.access_order.lock() {
                    order.retain(|x| x != &*old_id);
                }
            }
        }

        // Synthesize heat action (promote to hot)
        let action = TemperatureAction::Heat {
            distinction_id: id.clone(),
            level: TemperatureLevel::Hot,
        };
        let _ = self.synthesize_action_internal(action);

        // Update current state mapping
        self.current_state.insert(key, id.clone());

        // Check if we need to evict (only if this is a new distinction)
        let should_evict =
            self.cache.len() >= self.config.capacity && !self.cache.contains_key(&id);

        let evicted = if should_evict { self.evict_lru() } else { None };

        // Insert/update cache
        self.cache.insert(id.clone(), versioned);
        self.update_lru(id);

        evicted
    }

    /// Check if a key is in hot memory.
    pub fn contains_key(&self, key: &FullKey) -> bool {
        self.current_state.contains_key(key)
    }

    /// Check if a distinction ID is cached.
    pub fn contains_id(&self, id: &DistinctionId) -> bool {
        self.cache.contains_key(id)
    }

    /// Get current cache size.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get configured capacity.
    pub fn capacity(&self) -> usize {
        self.config.capacity
    }

    /// Get cache statistics.
    pub fn stats(&self) -> TemperatureStats {
        TemperatureStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            current_size: self.len(),
            capacity: self.config.capacity,
        }
    }

    /// Get all keys currently in hot memory.
    pub fn keys(&self) -> Vec<FullKey> {
        self.current_state.iter().map(|e| e.key().clone()).collect()
    }

    /// Clear hot memory (evict all to warm).
    ///
    /// # LCA Pattern
    ///
    /// Each eviction synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔEvict_Action`
    pub fn clear(&self) -> Vec<(FullKey, VersionedValue)> {
        let mut evicted = Vec::new();

        for entry in self.cache.iter() {
            // Find the key for this distinction
            for state_entry in self.current_state.iter() {
                if state_entry.value() == entry.key() {
                    evicted.push((state_entry.key().clone(), entry.value().clone()));
                    break;
                }
            }

            // Synthesize evict action
            let action = TemperatureAction::Evict {
                distinction_id: entry.key().clone(),
            };
            let _ = self.synthesize_action_internal(action);
        }

        self.cache.clear();
        self.current_state.clear();
        if let Ok(mut order) = self.access_order.lock() {
            order.clear();
        }

        evicted
    }

    /// Update LRU order - move to front (most recent).
    fn update_lru(&self, id: DistinctionId) {
        if let Ok(mut order) = self.access_order.lock() {
            // Remove if exists
            order.retain(|x| x != &id);
            // Add to front
            order.push_front(id);
        }
    }

    /// Evict least-recently-used item.
    ///
    /// # LCA Pattern
    ///
    /// Eviction synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔEvict_Action`
    fn evict_lru(&self) -> Option<Evicted> {
        let victim_id = {
            let order = self.access_order.lock().ok()?;
            order.back().cloned()
        }?;

        let versioned = self.cache.remove(&victim_id).map(|(_, v)| v)?;

        // Synthesize evict action
        let action = TemperatureAction::Evict {
            distinction_id: victim_id.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        // Find and remove from current_state
        let mut key_to_remove = None;
        for entry in self.current_state.iter() {
            if entry.value() == &victim_id {
                key_to_remove = Some(entry.key().clone());
                break;
            }
        }

        if let Some(key) = key_to_remove {
            self.current_state.remove(&key);
        }

        // Remove from LRU order
        if let Ok(mut order) = self.access_order.lock() {
            order.retain(|x| x != &victim_id);
        }

        self.evictions.fetch_add(1, Ordering::Relaxed);

        Some(Evicted {
            distinction_id: victim_id,
            versioned,
        })
    }

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: TemperatureAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        engine.synthesize(&self.local_root, &action_distinction)
    }
}

impl Default for TemperatureAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use TemperatureAgent::new(&shared_engine)
        panic!("TemperatureAgent requires a SharedEngine - use TemperatureAgent::new()")
    }
}

/// LCA Trait Implementation for TemperatureAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for TemperatureAgent {
    type ActionData = TemperatureAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: TemperatureAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

/// An item evicted from hot memory (should go to warm).
pub struct Evicted {
    pub distinction_id: DistinctionId,
    pub versioned: VersionedValue,
}

/// Temperature agent statistics.
#[derive(Debug, Clone)]
pub struct TemperatureStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub current_size: usize,
    pub capacity: usize,
}

impl TemperatureStats {
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
    fn test_put_and_get() {
        let engine = create_test_engine();
        let agent = TemperatureAgent::new(&engine);
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({"name": "Alice"}), "v1");

        agent.put(key.clone(), versioned.clone());

        let retrieved = agent.get(&key).unwrap();
        assert_eq!(retrieved.write_id(), "v1");
    }

    #[test]
    fn test_lru_eviction() {
        let config = TemperatureConfig {
            capacity: 2,
            promote_threshold: 1,
        };
        let engine = create_test_engine();
        let agent = TemperatureAgent::with_config(config, &engine);

        // Fill cache
        let key1 = FullKey::new("ns", "key1");
        let key2 = FullKey::new("ns", "key2");
        let key3 = FullKey::new("ns", "key3");

        let v1 = create_versioned(json!(1), "v1");
        let v2 = create_versioned(json!(2), "v2");
        let v3 = create_versioned(json!(3), "v3");

        agent.put(key1.clone(), v1);
        agent.put(key2.clone(), v2);

        // Access key1 to make it recently used
        agent.get(&key1);

        // Add key3 - should evict key2 (least recent)
        let evicted = agent.put(key3.clone(), v3);

        assert!(evicted.is_some(), "Should have evicted");
        assert_eq!(evicted.unwrap().distinction_id, "v2");
        assert!(agent.get(&key2).is_none(), "key2 should be evicted");
        assert!(agent.get(&key1).is_some(), "key1 should still be present");
    }

    #[test]
    fn test_hit_rate() {
        let engine = create_test_engine();
        let agent = TemperatureAgent::new(&engine);
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");

        agent.put(key.clone(), versioned);

        // Hit
        agent.get(&key);
        agent.get(&key);

        // Miss
        let missing = FullKey::new("users", "bob");
        agent.get(&missing);

        let stats = agent.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_update_existing() {
        let engine = create_test_engine();
        let agent = TemperatureAgent::new(&engine);
        let key = FullKey::new("users", "alice");

        let v1 = create_versioned(json!({"v": 1}), "v1");
        let v2 = create_versioned(json!({"v": 2}), "v2");

        agent.put(key.clone(), v1);
        agent.put(key.clone(), v2); // Update

        let current = agent.get(&key).unwrap();
        assert_eq!(current.write_id(), "v2");
        assert_eq!(agent.len(), 1); // Still just 1 entry
    }

    #[test]
    fn test_contains() {
        let engine = create_test_engine();
        let agent = TemperatureAgent::new(&engine);
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");

        assert!(!agent.contains_key(&key));

        agent.put(key.clone(), versioned);

        assert!(agent.contains_key(&key));
        assert!(agent.contains_id(&"v1".to_string()));
    }

    #[test]
    fn test_clear() {
        let engine = create_test_engine();
        let agent = TemperatureAgent::new(&engine);

        agent.put(FullKey::new("a", "1"), create_versioned(json!(1), "v1"));
        agent.put(FullKey::new("a", "2"), create_versioned(json!(2), "v2"));

        let evicted = agent.clear();

        assert_eq!(evicted.len(), 2);
        assert!(agent.is_empty());
    }

    #[test]
    fn test_stats() {
        let agent = TemperatureAgent::with_config(
            TemperatureConfig {
                capacity: 10,
                promote_threshold: 1,
            },
            &create_test_engine(),
        );

        // Add 5 items
        for i in 0..5 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            agent.put(key, versioned);
        }

        let stats = agent.stats();
        assert_eq!(stats.current_size, 5);
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.utilization(), 0.5);
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = TemperatureAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = TemperatureAction::Access {
            distinction_id: "test123".to_string(),
        };
        let engine_arc = Arc::clone(agent.field.engine_arc());
        let new_root = agent.synthesize_action(action, &engine_arc);
        assert!(!new_root.id().is_empty());
        assert_ne!(new_root.id(), root_id);

        // Test update_local_root
        agent.update_local_root(new_root.clone());
        assert_eq!(agent.get_current_root().id(), new_root.id());
    }
}
