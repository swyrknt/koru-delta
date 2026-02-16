/// Archive Agent: Consolidated epochs layer with LCA architecture.
///
/// The Archive Agent acts like the cerebral cortex - long-term storage,
/// compressed patterns, vast capacity. Data here is old but organized
/// into epochs for efficient access.
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Archive Agent's local root is `RootType::Archive` (🗄️ ARCHIVE).
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
/// Chronicle data is "distilled" into Archive through natural selection:
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
use crate::actions::ArchiveAction;
use crate::causal_graph::DistinctionId;
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;
use crate::types::{FullKey, VersionedValue};
#[cfg(test)]
use crate::types::VectorClock;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Archive agent configuration.
#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    /// Number of epochs to maintain
    pub epoch_count: usize,

    /// Epoch duration (how much time per epoch)
    pub epoch_duration: Duration,

    /// Maximum distinctions per epoch before compression
    pub max_distinctions_per_epoch: usize,

    /// Fitness threshold for keeping (references >= this)
    pub fitness_threshold: usize,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            epoch_count: 7,                      // 7 epochs (like days/weeks)
            epoch_duration: Duration::days(1),   // Daily epochs
            max_distinctions_per_epoch: 100_000, // Compress after 100K
            fitness_threshold: 2,                // 2+ references = keep
        }
    }
}

/// Archive Agent - consolidated long-term storage with LCA architecture.
///
/// Like the cerebral cortex: vast, compressed, pattern-organized.
/// All operations are synthesized through the unified field.
pub struct ArchiveAgent {
    /// Configuration
    config: ArchiveConfig,

    /// LCA: Local root distinction (Root: ARCHIVE)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

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
struct Epoch {
    /// Epoch number (kept for debugging)
    _number: usize,

    /// Time range (kept for debugging)
    _start_time: DateTime<Utc>,
    _end_time: DateTime<Utc>,

    /// Index: distinction_id → metadata
    index: HashMap<DistinctionId, EpochEntry>,

    /// Approximate size (for compression decisions)
    distinction_count: usize,
}

/// Entry within an epoch.
#[derive(Debug, Clone)]
struct EpochEntry {
    /// Original key
    key: FullKey,
    /// When created (kept for debugging)
    _timestamp: DateTime<Utc>,
    /// Fitness score (kept for future use)
    _fitness: usize,
    /// Compressed data reference
    data_ref: String,
}

impl ArchiveAgent {
    /// Create new archive agent with default configuration.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Archive (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self::with_config(ArchiveConfig::default(), shared_engine)
    }

    /// Create new archive agent with custom configuration.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the ARCHIVE root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(config: ArchiveConfig, shared_engine: &SharedEngine) -> Self {
        let local_root = shared_engine.root(RootType::Archive).clone();
        let field = FieldHandle::new(shared_engine);

        let agent = Self {
            config,
            local_root,
            field,
            epochs: DashMap::new(),
            current_epoch: AtomicU64::new(0),
            consolidations: AtomicU64::new(0),
            compressions: AtomicU64::new(0),
            archives: AtomicU64::new(0),
        };

        // Initialize first epoch
        agent.create_epoch(0);

        agent
    }

    /// Consolidate data from Chronicle into Archive.
    ///
    /// Takes distinctions from Chronicle that are old enough and:
    /// 1. Scores their fitness
    /// 2. Keeps high-fitness, archives low-fitness
    /// 3. Adds to current epoch
    ///
    /// # LCA Pattern
    ///
    /// Consolidation synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔArchive_Action`
    pub fn consolidate(
        &self,
        distinctions: Vec<(DistinctionId, FullKey, VersionedValue, usize)>, // (id, key, value, reference_count)
    ) -> ConsolidationResult {
        let mut kept = 0;
        let mut archived = 0;

        let epoch_num = self.current_epoch.load(Ordering::Relaxed) as usize;

        for (id, key, versioned, ref_count) in distinctions {
            let fitness = ref_count;

            // Synthesize archive action
            let action = ArchiveAction::Archive {
                distinction_ids: vec![id.clone()],
            };
            let _ = self.synthesize_action_internal(action);

            if fitness >= self.config.fitness_threshold {
                // Keep in archive
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

    /// Get a value from archive.
    ///
    /// Searches through epochs from newest to oldest.
    ///
    /// # LCA Pattern
    ///
    /// Retrieval synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔRetrieve_Action`
    pub fn get(&self, id: &DistinctionId) -> Option<(FullKey, String)> {
        let current = self.current_epoch.load(Ordering::Relaxed) as usize;

        // Synthesize retrieve action
        let action = ArchiveAction::Retrieve {
            pattern: id.clone(),
        };
        let _ = self.synthesize_action_internal(action);

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

    /// Check if a distinction is in archive.
    pub fn contains(&self, id: &DistinctionId) -> bool {
        self.get(id).is_some()
    }

    /// Rotate to a new epoch (called periodically).
    ///
    /// # LCA Pattern
    ///
    /// Epoch start synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔEpochStart_Action`
    pub fn rotate_epoch(&self) {
        let current = self.current_epoch.load(Ordering::Relaxed);
        let new_epoch = current + 1;

        // Synthesize epoch start action
        let action = ArchiveAction::EpochStart {
            timestamp: Utc::now(),
        };
        let _ = self.synthesize_action_internal(action);

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
    pub fn stats(&self) -> ArchiveStats {
        ArchiveStats {
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
            _number: number,
            _start_time: now,
            _end_time: now + self.config.epoch_duration,
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
                    _timestamp: timestamp,
                    _fitness: fitness,
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
            // Synthesize compress action
            let action = ArchiveAction::Compress {
                epoch_id: format!("epoch_{}", epoch_num),
            };
            let _ = self.synthesize_action_internal(action);

            // TODO: Implement compression
            self.compressions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Consolidate a distinction into archive.
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

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: ArchiveAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        engine.synthesize(&self.local_root, &action_distinction)
    }
}

impl Default for ArchiveAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use ArchiveAgent::new(&shared_engine)
        panic!("ArchiveAgent requires a SharedEngine - use ArchiveAgent::new()")
    }
}

/// LCA Trait Implementation for ArchiveAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for ArchiveAgent {
    type ActionData = ArchiveAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: ArchiveAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
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

/// Archive agent statistics.
#[derive(Debug, Clone)]
pub struct ArchiveStats {
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

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

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
    fn test_new_archive_agent() {
        let engine = create_test_engine();
        let archive = ArchiveAgent::new(&engine);

        assert_eq!(archive.current_epoch(), 0);
        assert_eq!(archive.epoch_count(), 1);
    }

    #[test]
    fn test_consolidate() {
        let engine = create_test_engine();
        let archive = ArchiveAgent::new(&engine);

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

        let result = archive.consolidate(distinctions);

        // Threshold is 2, so v2 (fitness 1) should be archived
        assert_eq!(result.kept, 2);
        assert_eq!(result.archived, 1);

        // Check stats
        let stats = archive.stats();
        assert_eq!(stats.consolidations, 1);
        assert_eq!(stats.archives, 1);
    }

    #[test]
    fn test_contains() {
        let engine = create_test_engine();
        let archive = ArchiveAgent::new(&engine);

        let distinctions = vec![(
            "v1".to_string(),
            FullKey::new("ns", "k1"),
            create_versioned(json!(1), "v1"),
            5,
        )];

        archive.consolidate(distinctions);

        assert!(archive.contains(&"v1".to_string()));
        assert!(!archive.contains(&"v2".to_string()));
    }

    #[test]
    fn test_rotate_epoch() {
        let engine = create_test_engine();
        let archive = ArchiveAgent::new(&engine);

        assert_eq!(archive.current_epoch(), 0);

        archive.rotate_epoch();
        assert_eq!(archive.current_epoch(), 1);
        assert_eq!(archive.epoch_count(), 2);

        archive.rotate_epoch();
        assert_eq!(archive.current_epoch(), 2);
        assert_eq!(archive.epoch_count(), 3);
    }

    #[test]
    fn test_epoch_limit() {
        let config = ArchiveConfig {
            epoch_count: 3,
            epoch_duration: Duration::days(1),
            max_distinctions_per_epoch: 100_000,
            fitness_threshold: 2,
        };
        let engine = create_test_engine();
        let archive = ArchiveAgent::with_config(config, &engine);

        // Rotate past limit
        for _ in 0..5 {
            archive.rotate_epoch();
        }

        // Should only have 3 epochs (current + 2 previous)
        assert_eq!(archive.epoch_count(), 3);
        assert_eq!(archive.current_epoch(), 5);
    }

    #[test]
    fn test_total_distinctions() {
        let engine = create_test_engine();
        let archive = ArchiveAgent::new(&engine);

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

        archive.consolidate(distinctions);

        assert_eq!(archive.total_distinctions(), 2);
    }

    #[test]
    fn test_custom_config() {
        let config = ArchiveConfig {
            epoch_count: 10,
            epoch_duration: Duration::hours(6),
            max_distinctions_per_epoch: 50_000,
            fitness_threshold: 5,
        };
        let engine = create_test_engine();
        let archive = ArchiveAgent::with_config(config, &engine);

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

        let result = archive.consolidate(distinctions);

        // Threshold is 5, so v1 (fitness 3) should be archived
        assert_eq!(result.kept, 1);
        assert_eq!(result.archived, 1);
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = ArchiveAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = ArchiveAction::EpochStart {
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

}
