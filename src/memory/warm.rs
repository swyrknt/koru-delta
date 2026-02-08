/// Warm Memory: Recent chronicle layer.
///
/// Warm memory acts like the hippocampus - recent episodic memory,
/// full detail, stored on disk. Data here was recently in Hot but
/// got evicted due to capacity limits or time.
///
/// ## Purpose
///
/// - Store recent history that's not in Hot memory
/// - Provide full causal chain for time travel
/// - Serve as staging area before Cold consolidation
/// - Keep recent data accessible without RAM pressure
///
/// ## When Data Moves Here
///
/// - Evicted from Hot memory (LRU eviction)
/// - Explicit demotion from Hot
/// - After configurable idle time
///
/// ## Persistence
///
/// Warm memory is disk-backed. Chronicle files are append-only
/// for durability. Index is in memory for fast lookup.
use crate::causal_graph::DistinctionId;
use crate::types::{FullKey, VectorClock, VersionedValue};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

/// Warm memory configuration.
#[derive(Debug, Clone)]
pub struct WarmConfig {
    /// Maximum number of recent distinctions to keep in index
    pub index_capacity: usize,
    
    /// Idle time before considering for demotion to Cold
    pub idle_threshold: Duration,
    
    /// Chronicle file rotation size (bytes)
    pub rotation_size: usize,
}

impl Default for WarmConfig {
    fn default() -> Self {
        Self {
            index_capacity: 10_000,                    // Keep 10K recent in index
            idle_threshold: Duration::hours(1),        // Idle 1 hour → Cold candidate
            rotation_size: 10 * 1024 * 1024,           // 10MB files
        }
    }
}

/// Warm Memory layer - recent chronicle on disk.
///
/// Like the hippocampus: recent, detailed, on disk (not RAM).
pub struct WarmMemory {
    /// Configuration
    config: WarmConfig,
    
    /// In-memory index: distinction_id → (key, file_offset, timestamp)
    /// For fast lookup without scanning disk
    index: DashMap<DistinctionId, IndexEntry>,
    
    /// Recent access window (for promotion to Hot)
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

impl WarmMemory {
    /// Create new warm memory with default configuration.
    pub fn new() -> Self {
        Self::with_config(WarmConfig::default())
    }
    
    /// Create new warm memory with custom configuration.
    pub fn with_config(config: WarmConfig) -> Self {
        let capacity = config.index_capacity;
        Self {
            config,
            index: DashMap::with_capacity(capacity),
            recent_window: std::sync::Mutex::new(VecDeque::with_capacity(capacity)),
            current_mappings: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            promotions: AtomicU64::new(0),
            demotions: AtomicU64::new(0),
        }
    }
    
    /// Get a value from warm memory.
    ///
    /// Returns the versioned value if found, updates access time.
    /// Returns None if not in warm memory (might be in Cold or doesn't exist).
    pub fn get(&self, id: &DistinctionId) -> Option<(FullKey, VersionedValue)> {
        let _entry = self.index.get(id)?;
        
        // Update access time
        // TODO: In real implementation, read from disk
        // For now, return placeholder
        
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
    
    /// Put a value into warm memory (from Hot eviction).
    ///
    /// Writes to disk chronicle, updates index.
    pub fn put(&self, key: FullKey, versioned: VersionedValue) {
        let id = versioned.write_id().to_string();
        let timestamp = versioned.timestamp;
        
        // Update current mapping
        self.current_mappings.insert(key.clone(), id.clone());
        
        // Check if we need to make room in index
        if self.index.len() >= self.config.index_capacity {
            self.evict_oldest_index_entry();
        }
        
        // Add to index
        self.index.insert(id.clone(), IndexEntry {
            key,
            timestamp,
            last_accessed: Utc::now(),
        });
        
        // Add to recent window
        self.add_to_recent_window(id);
        
        // TODO: Append to disk chronicle
    }
    
    /// Check if a distinction is in warm memory.
    pub fn contains(&self, id: &DistinctionId) -> bool {
        self.index.contains_key(id)
    }
    
    /// Check if a key has a current mapping in warm memory.
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
    pub fn stats(&self) -> WarmStats {
        WarmStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            promotions: self.promotions.load(Ordering::Relaxed),
            demotions: self.demotions.load(Ordering::Relaxed),
            current_size: self.len(),
            capacity: self.config.index_capacity,
        }
    }
    
    /// Find distinctions that should be promoted to Hot.
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
    
    /// Find distinctions that should be demoted to Cold.
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
        
        candidates.into_iter().take(limit).map(|(id, _)| id).collect()
    }
    
    /// Promote a distinction to Hot (remove from Warm index).
    pub fn promote(&self, id: &DistinctionId) {
        self.index.remove(id);
        self.promotions.fetch_add(1, Ordering::Relaxed);
        // Note: actual value would be returned and put in Hot
    }
    
    /// Demote a distinction to Cold (remove from index, keep on disk).
    pub fn demote(&self, id: &DistinctionId) {
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
}

impl Default for WarmMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Warm memory statistics.
#[derive(Debug, Clone)]
pub struct WarmStats {
    pub hits: u64,
    pub misses: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub current_size: usize,
    pub capacity: usize,
}

impl WarmStats {
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

    fn create_versioned(value: serde_json::Value, id: &str) -> VersionedValue {
        VersionedValue::new(
            Arc::new(value),
            Utc::now(),
            id.to_string(),  // write_id
            id.to_string(),  // distinction_id
            None,
            VectorClock::new(),
        )
    }

    #[test]
    fn test_put_and_contains() {
        let warm = WarmMemory::new();
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({"name": "Alice"}), "v1");
        
        warm.put(key.clone(), versioned);
        
        assert!(warm.contains(&"v1".to_string()));
        assert!(warm.contains_key(&key));
        assert_eq!(warm.len(), 1);
    }

    #[test]
    fn test_get_by_key() {
        let warm = WarmMemory::new();
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");
        
        warm.put(key.clone(), versioned);
        
        let id = warm.get_by_key(&key).unwrap();
        assert_eq!(id, "v1");
    }

    #[test]
    fn test_capacity_and_eviction() {
        let config = WarmConfig {
            index_capacity: 3,
            idle_threshold: Duration::hours(1),
            rotation_size: 10_000_000,
        };
        let warm = WarmMemory::with_config(config);
        
        // Add 5 items (capacity is 3)
        for i in 0..5 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            warm.put(key, versioned);
        }
        
        // Should still be at capacity
        assert_eq!(warm.len(), 3);
    }

    #[test]
    fn test_stats() {
        let warm = WarmMemory::with_config(WarmConfig {
            index_capacity: 100,
            idle_threshold: Duration::hours(1),
            rotation_size: 10_000_000,
        });
        
        // Add items
        for i in 0..10 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            warm.put(key, versioned);
        }
        
        let stats = warm.stats();
        assert_eq!(stats.current_size, 10);
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.utilization(), 0.1);
    }

    #[test]
    fn test_update_existing_key() {
        let warm = WarmMemory::new();
        let key = FullKey::new("users", "alice");
        
        let v1 = create_versioned(json!({"v": 1}), "v1");
        let v2 = create_versioned(json!({"v": 2}), "v2");
        
        warm.put(key.clone(), v1);
        warm.put(key.clone(), v2);
        
        // Current mapping should be v2
        assert_eq!(warm.get_by_key(&key).unwrap(), "v2");
        
        // Both versions should be in index
        assert!(warm.contains(&"v1".to_string()));
        assert!(warm.contains(&"v2".to_string()));
    }

    #[test]
    fn test_promote_and_demote() {
        let warm = WarmMemory::new();
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");
        
        warm.put(key, versioned);
        assert_eq!(warm.len(), 1);
        
        // Promote (remove from warm)
        warm.promote(&"v1".to_string());
        assert_eq!(warm.len(), 0);
        assert_eq!(warm.stats().promotions, 1);
        
        // Add again and demote
        let key2 = FullKey::new("users", "bob");
        let versioned2 = create_versioned(json!({}), "v2");
        warm.put(key2, versioned2);
        
        warm.demote(&"v2".to_string());
        assert_eq!(warm.len(), 0);
        assert_eq!(warm.stats().demotions, 1);
    }

    #[test]
    fn test_find_promotion_candidates() {
        let warm = WarmMemory::new();
        
        // Add some items
        for i in 0..5 {
            let key = FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(json!(i), &format!("v{}", i));
            warm.put(key, versioned);
        }
        
        // Access some to make them "recent"
        for i in 0..3 {
            warm.add_to_recent_window(format!("v{}", i));
        }
        
        let candidates = warm.find_promotion_candidates(10);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_is_empty() {
        let warm = WarmMemory::new();
        assert!(warm.is_empty());
        
        let key = FullKey::new("users", "alice");
        let versioned = create_versioned(json!({}), "v1");
        warm.put(key, versioned);
        
        assert!(!warm.is_empty());
    }
}
