/// Cold Memory: Consolidated epochs layer.
///
/// Cold memory acts like the cerebral cortex - long-term storage,
/// compressed patterns, vast capacity. Data here is old but organized
/// into epochs for efficient access.
///
/// ## Purpose
///
/// - Store old data that's rarely accessed but needs to be kept
/// - Compress patterns (deduplication across time)
/// - Enable efficient archival to Deep memory
/// - Provide bounded storage growth
///
/// ## Consolidation
///
/// Warm data is "distilled" into Cold through natural selection:
/// - High-fitness distinctions kept (referenced, important)
/// - Low-fitness distinctions archived or discarded
/// - Patterns extracted and compressed
///
/// ## Epoch Structure
///
/// Data is organized into epochs (time-based chunks):
/// - Epoch 0: Oldest, most compressed
/// - Epoch N: Newest, less compressed
/// - Each epoch has an index for fast lookup
use crate::causal_graph::DistinctionId;
#[cfg(test)]
use crate::types::VectorClock;
use crate::types::{FullKey, VersionedValue};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Cold memory configuration.
#[derive(Debug, Clone)]
pub struct ColdConfig {
    /// Number of epochs to maintain
    pub epoch_count: usize,

    /// Epoch duration (how much time per epoch)
    pub epoch_duration: Duration,

    /// Maximum distinctions per epoch before compression
    pub max_distinctions_per_epoch: usize,

    /// Fitness threshold for keeping (references >= this)
    pub fitness_threshold: usize,
}

impl Default for ColdConfig {
    fn default() -> Self {
        Self {
            epoch_count: 7,                      // 7 epochs (like days/weeks)
            epoch_duration: Duration::days(1),   // Daily epochs
            max_distinctions_per_epoch: 100_000, // Compress after 100K
            fitness_threshold: 2,                // 2+ references = keep
        }
    }
}

/// Cold Memory layer - consolidated long-term storage.
///
/// Like the cerebral cortex: vast, compressed, pattern-organized.
pub struct ColdMemory {
    /// Configuration
    config: ColdConfig,

    /// Epochs (0 = oldest, N = newest)
    /// Each epoch has its own index and data
    epochs: DashMap<usize, Epoch>,

    /// Current epoch number
    current_epoch: AtomicU64,

    /// Statistics
    consolidations: AtomicU64,
    compressions: AtomicU64,
    archives: AtomicU64,
}

/// A single epoch of consolidated data.
#[derive(Debug)]
#[allow(dead_code)]
struct Epoch {
    /// Epoch number
    number: usize,

    /// Time range
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,

    /// Index: distinction_id → metadata
    index: HashMap<DistinctionId, EpochEntry>,

    /// Approximate size (for compression decisions)
    distinction_count: usize,
}

/// Entry within an epoch.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EpochEntry {
    /// Original key
    key: FullKey,
    /// When created
    timestamp: DateTime<Utc>,
    /// Fitness score (references)
    fitness: usize,
    /// Compressed data reference
    data_ref: String,
}

impl ColdMemory {
    /// Create new cold memory with default configuration.
    pub fn new() -> Self {
        Self::with_config(ColdConfig::default())
    }

    /// Create new cold memory with custom configuration.
    pub fn with_config(config: ColdConfig) -> Self {
        let memory = Self {
            config,
            epochs: DashMap::new(),
            current_epoch: AtomicU64::new(0),
            consolidations: AtomicU64::new(0),
            compressions: AtomicU64::new(0),
            archives: AtomicU64::new(0),
        };

        // Initialize first epoch
        memory.create_epoch(0);

        memory
    }

    /// Consolidate data from Warm into Cold.
    ///
    /// Takes distinctions from Warm that are old enough and:
    /// 1. Scores their fitness
    /// 2. Keeps high-fitness, archives low-fitness
    /// 3. Adds to current epoch
    pub fn consolidate(
        &self,
        distinctions: Vec<(DistinctionId, FullKey, VersionedValue, usize)>, // (id, key, value, reference_count)
    ) -> ConsolidationResult {
        let mut kept = 0;
        let mut archived = 0;

        let epoch_num = self.current_epoch.load(Ordering::Relaxed) as usize;

        for (id, key, versioned, ref_count) in distinctions {
            let fitness = ref_count;

            if fitness >= self.config.fitness_threshold {
                // Keep in cold memory
                self.add_to_epoch(epoch_num, id, key, versioned.timestamp, fitness);
                kept += 1;
            } else {
                // Archive (would go to Deep)
                archived += 1;
                self.archives.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.consolidations.fetch_add(1, Ordering::Relaxed);

        // Check if current epoch needs compression
        self.maybe_compress_epoch(epoch_num);

        ConsolidationResult { kept, archived }
    }

    /// Get a value from cold memory.
    ///
    /// Searches through epochs from newest to oldest.
    pub fn get(&self, id: &DistinctionId) -> Option<(FullKey, String)> {
        let current = self.current_epoch.load(Ordering::Relaxed) as usize;

        // Search from newest to oldest
        for epoch_num in (0..=current).rev() {
            if let Some(epoch) = self.epochs.get(&epoch_num) {
                if let Some(entry) = epoch.index.get(id) {
                    return Some((entry.key.clone(), entry.data_ref.clone()));
                }
            }
        }

        None
    }

    /// Get distinction ID by key (reverse lookup).
    ///
    /// Searches through epochs from newest to oldest.
    pub fn get_by_key(&self, key: &FullKey) -> Option<DistinctionId> {
        let current = self.current_epoch.load(Ordering::Relaxed) as usize;

        // Search from newest to oldest
        for epoch_num in (0..=current).rev() {
            if let Some(epoch) = self.epochs.get(&epoch_num) {
                // Find entry with matching key
                for (id, entry) in &epoch.index {
                    if &entry.key == key {
                        return Some(id.clone());
                    }
                }
            }
        }

        None
    }

    /// Check if a distinction is in cold memory.
    pub fn contains(&self, id: &DistinctionId) -> bool {
        self.get(id).is_some()
    }

    /// Rotate to a new epoch (called periodically).
    pub fn rotate_epoch(&self) {
        let current = self.current_epoch.load(Ordering::Relaxed);
        let new_epoch = current + 1;

        // Remove oldest epoch if we have too many
        let to_remove = new_epoch as i64 - self.config.epoch_count as i64;
        if to_remove >= 0 {
            self.epochs.remove(&(to_remove as usize));
        }

        // Create new epoch
        self.create_epoch(new_epoch as usize);
        self.current_epoch.store(new_epoch, Ordering::Relaxed);
    }

    /// Get current epoch number.
    pub fn current_epoch(&self) -> usize {
        self.current_epoch.load(Ordering::Relaxed) as usize
    }

    /// Get epoch count.
    pub fn epoch_count(&self) -> usize {
        self.epochs.len()
    }

    /// Get total distinctions across all epochs.
    pub fn total_distinctions(&self) -> usize {
        self.epochs.iter().map(|e| e.distinction_count).sum()
    }

    /// Get statistics.
    pub fn stats(&self) -> ColdStats {
        ColdStats {
            consolidations: self.consolidations.load(Ordering::Relaxed),
            compressions: self.compressions.load(Ordering::Relaxed),
            archives: self.archives.load(Ordering::Relaxed),
            epoch_count: self.epoch_count(),
            total_distinctions: self.total_distinctions(),
        }
    }

    /// Extract patterns from an epoch (for Deep memory).
    ///
    /// Returns common patterns across distinctions.
    pub fn extract_patterns(&self, epoch_num: usize) -> Vec<Pattern> {
        // Real implementation would analyze data and extract common structures
        // Validate epoch_num by checking it exists
        if self.epochs.contains_key(&epoch_num) {
            // TODO: Actually extract patterns from this epoch
        }
        vec![]
    }

    /// Create a new epoch.
    fn create_epoch(&self, number: usize) {
        let now = Utc::now();
        let epoch = Epoch {
            number,
            start_time: now,
            end_time: now + self.config.epoch_duration,
            index: HashMap::new(),
            distinction_count: 0,
        };

        self.epochs.insert(number, epoch);
    }

    /// Add a distinction to an epoch.
    fn add_to_epoch(
        &self,
        epoch_num: usize,
        id: DistinctionId,
        key: FullKey,
        timestamp: DateTime<Utc>,
        fitness: usize,
    ) {
        if let Some(mut epoch) = self.epochs.get_mut(&epoch_num) {
            epoch.index.insert(
                id.clone(),
                EpochEntry {
                    key,
                    timestamp,
                    fitness,
                    data_ref: format!("epoch_{}/data_{}", epoch_num, id),
                },
            );
            epoch.distinction_count += 1;
        }
    }

    /// Compress an epoch if it's too large.
    fn maybe_compress_epoch(&self, epoch_num: usize) {
        let should_compress = self
            .epochs
            .get(&epoch_num)
            .map(|e| e.distinction_count >= self.config.max_distinctions_per_epoch)
            .unwrap_or(false);

        if should_compress {
            // TODO: Implement compression
            self.compressions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Consolidate a distinction into cold memory.
    ///
    /// Adds the distinction to the current epoch.
    pub fn consolidate_distinction(&self, _id: &DistinctionId) {
        // In real implementation, would fetch from storage and add to epoch
        // For now, just increment counter
        self.consolidations.fetch_add(1, Ordering::Relaxed);
    }

    /// Compress old epochs to save space.
    pub fn compress_old_epochs(&self) {
        let current = self.current_epoch.load(Ordering::Relaxed) as usize;

        // Compress all epochs except the current one
        for epoch_num in 0..current {
            self.maybe_compress_epoch(epoch_num);
        }
    }
}

impl Default for ColdMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a consolidation operation.
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    pub kept: usize,
    pub archived: usize,
}

/// Extracted pattern from an epoch.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: String,
    pub frequency: usize,
    pub template: String,
}

/// Cold memory statistics.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ColdStats {
    pub consolidations: u64,
    pub compressions: u64,
    pub archives: u64,
    pub epoch_count: usize,
    pub total_distinctions: usize,
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
            id.to_string(), // write_id
            id.to_string(), // distinction_id (same for tests)
            None,
            VectorClock::new(),
        )
    }

    #[test]
    fn test_new_cold_memory() {
        let cold = ColdMemory::new();

        assert_eq!(cold.current_epoch(), 0);
        assert_eq!(cold.epoch_count(), 1);
    }

    #[test]
    fn test_consolidate() {
        let cold = ColdMemory::new();

        // Create distinctions with varying fitness
        let distinctions = vec![
            (
                "v1".to_string(),
                FullKey::new("ns", "k1"),
                create_versioned(json!(1), "v1"),
                5,
            ), // High fitness
            (
                "v2".to_string(),
                FullKey::new("ns", "k2"),
                create_versioned(json!(2), "v2"),
                1,
            ), // Low fitness
            (
                "v3".to_string(),
                FullKey::new("ns", "k3"),
                create_versioned(json!(3), "v3"),
                3,
            ), // Medium fitness
        ];

        let result = cold.consolidate(distinctions);

        // Threshold is 2, so v2 (fitness 1) should be archived
        assert_eq!(result.kept, 2);
        assert_eq!(result.archived, 1);

        // Check stats
        let stats = cold.stats();
        assert_eq!(stats.consolidations, 1);
        assert_eq!(stats.archives, 1);
    }

    #[test]
    fn test_contains() {
        let cold = ColdMemory::new();

        let distinctions = vec![(
            "v1".to_string(),
            FullKey::new("ns", "k1"),
            create_versioned(json!(1), "v1"),
            5,
        )];

        cold.consolidate(distinctions);

        assert!(cold.contains(&"v1".to_string()));
        assert!(!cold.contains(&"v2".to_string()));
    }

    #[test]
    fn test_rotate_epoch() {
        let cold = ColdMemory::new();

        assert_eq!(cold.current_epoch(), 0);

        cold.rotate_epoch();
        assert_eq!(cold.current_epoch(), 1);
        assert_eq!(cold.epoch_count(), 2);

        cold.rotate_epoch();
        assert_eq!(cold.current_epoch(), 2);
        assert_eq!(cold.epoch_count(), 3);
    }

    #[test]
    fn test_epoch_limit() {
        let config = ColdConfig {
            epoch_count: 3,
            epoch_duration: Duration::days(1),
            max_distinctions_per_epoch: 100_000,
            fitness_threshold: 2,
        };
        let cold = ColdMemory::with_config(config);

        // Rotate past limit
        for _ in 0..5 {
            cold.rotate_epoch();
        }

        // Should only have 3 epochs (current + 2 previous)
        assert_eq!(cold.epoch_count(), 3);
        assert_eq!(cold.current_epoch(), 5);
    }

    #[test]
    fn test_total_distinctions() {
        let cold = ColdMemory::new();

        let distinctions = vec![
            (
                "v1".to_string(),
                FullKey::new("ns", "k1"),
                create_versioned(json!(1), "v1"),
                5,
            ),
            (
                "v2".to_string(),
                FullKey::new("ns", "k2"),
                create_versioned(json!(2), "v2"),
                3,
            ),
        ];

        cold.consolidate(distinctions);

        assert_eq!(cold.total_distinctions(), 2);
    }

    #[test]
    fn test_custom_config() {
        let config = ColdConfig {
            epoch_count: 10,
            epoch_duration: Duration::hours(6),
            max_distinctions_per_epoch: 50_000,
            fitness_threshold: 5,
        };
        let cold = ColdMemory::with_config(config);

        let distinctions = vec![
            (
                "v1".to_string(),
                FullKey::new("ns", "k1"),
                create_versioned(json!(1), "v1"),
                3,
            ), // Below threshold
            (
                "v2".to_string(),
                FullKey::new("ns", "k2"),
                create_versioned(json!(2), "v2"),
                5,
            ), // At threshold
        ];

        let result = cold.consolidate(distinctions);

        // Threshold is 5, so v1 (fitness 3) should be archived
        assert_eq!(result.kept, 1);
        assert_eq!(result.archived, 1);
    }
}
