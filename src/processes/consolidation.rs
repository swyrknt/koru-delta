/// Consolidation Process: The sleep cycle of memory.
///
/// This process moves data through the memory layers:
/// - Hot → Warm (when evicted from hot)
/// - Warm → Cold (when idle too long)
///
/// Like sleep consolidating memories from short-term to long-term.
use crate::causal_graph::DistinctionId;
use crate::memory::{ColdMemory, HotMemory, WarmMemory};
use crate::types::{FullKey, VersionedValue};
use std::sync::atomic::{AtomicU64, Ordering};

/// Consolidation configuration.
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// How often to run consolidation (seconds)
    pub interval_secs: u64,
    
    /// Batch size for moving distinctions
    pub batch_size: usize,
    
    /// Idle threshold for demotion from Warm to Cold
    pub demotion_idle_threshold: std::time::Duration,
    
    /// Ratio of distinctions to consolidate (0.0-1.0)
    pub consolidation_ratio: f64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300, // 5 minutes
            batch_size: 100,
            demotion_idle_threshold: std::time::Duration::from_secs(3600), // 1 hour
            consolidation_ratio: 0.5,
        }
    }
}

/// Consolidation Process - moves data between memory layers.
pub struct ConsolidationProcess {
    config: ConsolidationConfig,
    hot_to_warm: AtomicU64,
    warm_to_cold: AtomicU64,
    cycle_count: AtomicU64,
}

impl ConsolidationProcess {
    /// Create new consolidation process.
    pub fn new() -> Self {
        Self::with_config(ConsolidationConfig::default())
    }
    
    /// Create with custom config.
    pub fn with_config(config: ConsolidationConfig) -> Self {
        Self {
            config,
            hot_to_warm: AtomicU64::new(0),
            warm_to_cold: AtomicU64::new(0),
            cycle_count: AtomicU64::new(0),
        }
    }
    
    /// Get the cycle count.
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count.load(Ordering::Relaxed)
    }
    
    /// Handle eviction from Hot memory - move to Warm.
    ///
    /// Called when HotMemory evicts a value.
    pub fn handle_hot_eviction(
        &self,
        warm: &WarmMemory,
        key: FullKey,
        versioned: VersionedValue,
    ) {
        warm.put(key, versioned);
        self.hot_to_warm.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Consolidate Warm to Cold based on idle time.
    ///
    /// Finds idle distinctions in Warm and moves them to Cold.
    pub fn consolidate_warm_to_cold(
        &self,
        warm: &WarmMemory,
        cold: &ColdMemory,
        reference_counts: &std::collections::HashMap<DistinctionId, usize>,
    ) -> ConsolidationResult {
        // Find demotion candidates from Warm
        let candidates = warm.find_demotion_candidates(self.config.batch_size);
        
        let mut moved = 0;
        let mut failed = 0;
        
        for id in candidates {
            // Get the distinction details from Warm
            if let Some((key, _versioned)) = warm.get(&id) {
                // Get reference count for fitness
                let ref_count = reference_counts.get(&id).copied().unwrap_or(0);
                
                // Create placeholder versioned value (in real impl, would get actual data)
                let versioned = crate::types::VersionedValue::new(
                    std::sync::Arc::new(serde_json::json!({})),
                    chrono::Utc::now(),
                    id.clone(), // write_id
                    id.clone(), // distinction_id
                    None,
                );
                
                // Consolidate to Cold
                let distinctions = vec![(id.clone(), key, versioned, ref_count)];
                let result = cold.consolidate(distinctions);
                
                moved += result.kept;
                
                // Demote from Warm (remove from index)
                warm.demote(&id);
                
                if result.archived > 0 {
                    failed += result.archived;
                }
            }
        }
        
        self.warm_to_cold.fetch_add(moved as u64, Ordering::Relaxed);
        
        ConsolidationResult {
            distinctions_moved: moved,
            distinctions_failed: failed,
        }
    }
    
    /// Promote frequently accessed items from Warm to Hot.
    ///
    /// Called to bring hot candidates back into fast memory.
    pub fn promote_to_hot(
        &self,
        warm: &WarmMemory,
        hot: &HotMemory,
        epoch_num: usize,
        limit: usize,
    ) -> usize {
        let candidates = warm.find_promotion_candidates(limit);
        
        let mut promoted = 0;
        for (key, id) in candidates {
            // In real impl, would fetch actual value from Warm storage
            let versioned = crate::types::VersionedValue::new(
                std::sync::Arc::new(serde_json::json!({})),
                chrono::Utc::now(),
                id.clone(), // write_id
                id,         // distinction_id
                None,
            );
            
            hot.put(key, versioned);
            promoted += 1;
        }
        
        promoted
    }
    
    /// Get statistics.
    pub fn stats(&self) -> ConsolidationStats {
        ConsolidationStats {
            hot_to_warm: self.hot_to_warm.load(Ordering::Relaxed),
            warm_to_cold: self.warm_to_cold.load(Ordering::Relaxed),
        }
    }
    
    /// Get interval.
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.interval_secs)
    }
}

impl Default for ConsolidationProcess {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of consolidation.
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    pub distinctions_moved: usize,
    pub distinctions_failed: usize,
}

/// Consolidation statistics.
#[derive(Debug, Clone)]
pub struct ConsolidationStats {
    pub hot_to_warm: u64,
    pub warm_to_cold: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{HotConfig, HotMemory, WarmConfig, WarmMemory};
    use serde_json::json;
    use std::sync::Arc;

    fn create_versioned(id: &str) -> VersionedValue {
        crate::types::VersionedValue::new(
            Arc::new(json!({"id": id})),
            chrono::Utc::now(),
            id.to_string(), // write_id
            id.to_string(), // distinction_id
            None,
        )
    }

    #[test]
    fn test_handle_hot_eviction() {
        let consolidation = ConsolidationProcess::new();
        let warm = WarmMemory::new();
        let key = crate::types::FullKey::new("ns", "key1");
        let versioned = create_versioned("v1");
        
        consolidation.handle_hot_eviction(&warm, key.clone(), versioned);
        
        assert!(warm.contains_key(&key));
        assert_eq!(consolidation.stats().hot_to_warm, 1);
    }

    #[test]
    fn test_consolidation_stats() {
        let consolidation = ConsolidationProcess::new();
        let warm = WarmMemory::new();
        
        // Simulate some evictions
        for i in 0..5 {
            let key = crate::types::FullKey::new("ns", &format!("key{}", i));
            let versioned = create_versioned(&format!("v{}", i));
            consolidation.handle_hot_eviction(&warm, key, versioned);
        }
        
        let stats = consolidation.stats();
        assert_eq!(stats.hot_to_warm, 5);
    }

    #[test]
    fn test_config() {
        let config = ConsolidationConfig {
            interval_secs: 600,
            batch_size: 50,
            demotion_idle_threshold: std::time::Duration::from_secs(600),
            consolidation_ratio: 0.5,
        };
        let consolidation = ConsolidationProcess::with_config(config);
        
        assert_eq!(consolidation.interval().as_secs(), 600);
    }
}
