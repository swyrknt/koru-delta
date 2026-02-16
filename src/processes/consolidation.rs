/// Sleep Agent: The sleep cycle of memory with LCA architecture.
///
/// The Sleep Agent moves data through the memory layers:
/// - Hot → Warm (when evicted from hot)
/// - Warm → Cold (when idle too long)
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Sleep Agent's local root is `RootType::Sleep` (🌙 SLEEP).
///
/// ## Sleep Phases
///
/// Like biological sleep, the agent cycles through phases:
/// - **Awake**: Normal operation, no consolidation
/// - **LightSleep**: Hot → Warm consolidation (quick, frequent)
/// - **DeepSleep**: Warm → Cold consolidation (thorough, slower)
/// - **REM**: Pattern extraction and dreaming (random synthesis)
///
/// ## Analogy
///
/// Like sleep consolidating memories from short-term to long-term.
/// The hippocampus (Warm) transfers to cortex (Cold) during deep sleep.
use crate::actions::{SleepAction, SleepPhase};
use crate::causal_graph::DistinctionId;
use crate::engine::{FieldHandle, SharedEngine};
use crate::memory::{ColdMemory, HotMemory, WarmMemory};
use crate::roots::RootType;
use crate::types::{FullKey, VectorClock, VersionedValue};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Sleep agent configuration.
#[derive(Debug, Clone)]
pub struct SleepConfig {
    /// How often to run consolidation (seconds)
    pub interval_secs: u64,

    /// Batch size for moving distinctions
    pub batch_size: usize,

    /// Idle threshold for demotion from Warm to Cold
    pub demotion_idle_threshold: std::time::Duration,

    /// Ratio of distinctions to consolidate (0.0-1.0)
    pub consolidation_ratio: f64,
}

impl Default for SleepConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300, // 5 minutes
            batch_size: 100,
            demotion_idle_threshold: std::time::Duration::from_secs(3600), // 1 hour
            consolidation_ratio: 0.5,
        }
    }
}

/// Sleep Agent - moves data between memory layers with LCA architecture.
///
/// Like sleep consolidating memories from short-term to long-term.
/// All operations are synthesized through the unified field.
#[derive(Debug)]
pub struct SleepAgent {
    /// Configuration
    config: SleepConfig,

    /// LCA: Local root distinction (Root: SLEEP)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// Current sleep phase
    phase: std::sync::Mutex<SleepPhase>,

    /// Statistics
    hot_to_warm: AtomicU64,
    warm_to_cold: AtomicU64,
    cycle_count: AtomicU64,
}

impl SleepAgent {
    /// Create new sleep agent.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Sleep (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    /// - `phase` = Awake (initial state)
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self::with_config(SleepConfig::default(), shared_engine)
    }

    /// Create with custom config.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the SLEEP root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(config: SleepConfig, shared_engine: &SharedEngine) -> Self {
        let local_root = shared_engine.root(RootType::Sleep).clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            config,
            local_root,
            field,
            phase: std::sync::Mutex::new(SleepPhase::Awake),
            hot_to_warm: AtomicU64::new(0),
            warm_to_cold: AtomicU64::new(0),
            cycle_count: AtomicU64::new(0),
        }
    }

    /// Get the current sleep phase.
    pub fn phase(&self) -> SleepPhase {
        self.phase.lock().map(|g| *g).unwrap_or(SleepPhase::Awake)
    }

    /// Enter a sleep phase.
    ///
    /// # LCA Pattern
    ///
    /// Phase transition synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔEnterPhase_Action`
    pub fn enter_phase(&self, phase: SleepPhase) {
        // Synthesize enter phase action
        let action = SleepAction::EnterPhase { phase };
        let _ = self.synthesize_action_internal(action);

        if let Ok(mut current) = self.phase.lock() {
            *current = phase;
        }
    }

    /// Dream - random synthesis exploration.
    ///
    /// # LCA Pattern
    ///
    /// Dreaming synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔDream_Action`
    pub fn dream(&self) {
        // Synthesize dream action
        let action = SleepAction::Dream;
        let _ = self.synthesize_action_internal(action);

        // TODO: Implement random synthesis exploration
        // This would explore the field through random synthesis paths
    }

    /// Wake from sleep.
    ///
    /// # LCA Pattern
    ///
    /// Waking synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔWake_Action`
    pub fn wake(&self) {
        // Synthesize wake action
        let action = SleepAction::Wake;
        let _ = self.synthesize_action_internal(action);

        self.enter_phase(SleepPhase::Awake);
    }

    /// Get the cycle count.
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count.load(Ordering::Relaxed)
    }

    /// Handle eviction from Hot memory - move to Warm.
    ///
    /// Called when HotMemory evicts a value.
    ///
    /// # LCA Pattern
    ///
    /// Consolidation synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔConsolidate_Action`
    pub fn handle_hot_eviction(&self, warm: &WarmMemory, key: FullKey, versioned: VersionedValue) {
        // Synthesize consolidate action
        let action = SleepAction::Consolidate {
            from_tier: "hot".to_string(),
            to_tier: "warm".to_string(),
        };
        let _ = self.synthesize_action_internal(action);

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
        // Enter deep sleep phase for this consolidation
        self.enter_phase(SleepPhase::DeepSleep);

        // Synthesize consolidate action
        let action = SleepAction::Consolidate {
            from_tier: "warm".to_string(),
            to_tier: "cold".to_string(),
        };
        let _ = self.synthesize_action_internal(action);

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
                    VectorClock::new(),
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
        _epoch_num: usize,
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
                VectorClock::new(),
            );

            hot.put(key, versioned);
            promoted += 1;
        }

        promoted
    }

    /// Run a full consolidation cycle.
    ///
    /// Cycles through sleep phases and performs consolidation.
    pub fn run_cycle(&self, warm: &WarmMemory, cold: &ColdMemory) {
        self.cycle_count.fetch_add(1, Ordering::Relaxed);

        // Light sleep: quick consolidation
        self.enter_phase(SleepPhase::LightSleep);

        // Deep sleep: thorough consolidation
        self.enter_phase(SleepPhase::DeepSleep);
        let _ = self.consolidate_warm_to_cold(warm, cold, &std::collections::HashMap::new());

        // REM: pattern extraction and dreaming
        self.enter_phase(SleepPhase::Rem);
        self.dream();

        // Wake up
        self.wake();
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

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: SleepAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        engine.synthesize(&self.local_root, &action_distinction)
    }
}

impl Default for SleepAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use SleepAgent::new(&shared_engine)
        panic!("SleepAgent requires a SharedEngine - use SleepAgent::new()")
    }
}

/// LCA Trait Implementation for SleepAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for SleepAgent {
    type ActionData = SleepAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: SleepAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
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

/// Backward-compatible type alias for existing code.
pub type ConsolidationProcess = SleepAgent;

/// Backward-compatible type alias for existing code.
pub type ConsolidationConfig = SleepConfig;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SharedEngine;
    use crate::memory::WarmMemory;
    use serde_json::json;
    use std::sync::Arc;

    fn create_versioned(id: &str) -> VersionedValue {
        crate::types::VersionedValue::new(
            Arc::new(json!({"id": id})),
            chrono::Utc::now(),
            id.to_string(), // write_id
            id.to_string(), // distinction_id
            None,           // previous_version
            VectorClock::new(),
        )
    }

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

    #[test]
    fn test_new_sleep_agent() {
        let engine = create_test_engine();
        let sleep = SleepAgent::new(&engine);

        assert_eq!(sleep.phase(), SleepPhase::Awake);
        assert_eq!(sleep.cycle_count(), 0);
    }

    #[test]
    fn test_phase_transitions() {
        let engine = create_test_engine();
        let sleep = SleepAgent::new(&engine);

        assert_eq!(sleep.phase(), SleepPhase::Awake);

        sleep.enter_phase(SleepPhase::LightSleep);
        assert_eq!(sleep.phase(), SleepPhase::LightSleep);

        sleep.enter_phase(SleepPhase::DeepSleep);
        assert_eq!(sleep.phase(), SleepPhase::DeepSleep);

        sleep.enter_phase(SleepPhase::Rem);
        assert_eq!(sleep.phase(), SleepPhase::Rem);

        sleep.wake();
        assert_eq!(sleep.phase(), SleepPhase::Awake);
    }

    #[test]
    fn test_handle_hot_eviction() {
        let engine = create_test_engine();
        let sleep = SleepAgent::new(&engine);
        let warm = WarmMemory::new(&engine);
        let key = crate::types::FullKey::new("ns", "key1");
        let versioned = create_versioned("v1");

        sleep.handle_hot_eviction(&warm, key.clone(), versioned);

        assert!(warm.contains_key(&key));
        assert_eq!(sleep.stats().hot_to_warm, 1);
    }

    #[test]
    fn test_consolidation_stats() {
        let engine = create_test_engine();
        let sleep = SleepAgent::new(&engine);
        let warm = WarmMemory::new(&engine);

        // Simulate some evictions
        for i in 0..5 {
            let key = crate::types::FullKey::new("ns", format!("key{}", i));
            let versioned = create_versioned(&format!("v{}", i));
            sleep.handle_hot_eviction(&warm, key, versioned);
        }

        let stats = sleep.stats();
        assert_eq!(stats.hot_to_warm, 5);
    }

    #[test]
    fn test_config() {
        let config = SleepConfig {
            interval_secs: 600,
            batch_size: 50,
            demotion_idle_threshold: std::time::Duration::from_secs(600),
            consolidation_ratio: 0.5,
        };
        let engine = create_test_engine();
        let sleep = SleepAgent::with_config(config, &engine);

        assert_eq!(sleep.interval().as_secs(), 600);
    }

    #[test]
    fn test_dream() {
        let engine = create_test_engine();
        let sleep = SleepAgent::new(&engine);

        // Enter REM phase and dream
        sleep.enter_phase(SleepPhase::Rem);
        sleep.dream();

        // Should still be in REM phase
        assert_eq!(sleep.phase(), SleepPhase::Rem);
    }

    #[test]
    fn test_run_cycle() {
        let engine = create_test_engine();
        let sleep = SleepAgent::new(&engine);
        let warm = WarmMemory::new(&engine);
        let cold = ColdMemory::new(&engine);

        assert_eq!(sleep.cycle_count(), 0);

        sleep.run_cycle(&warm, &cold);

        assert_eq!(sleep.cycle_count(), 1);
        assert_eq!(sleep.phase(), SleepPhase::Awake); // Should wake after cycle
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = SleepAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = SleepAction::EnterPhase {
            phase: SleepPhase::DeepSleep,
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
        let _consolidation: ConsolidationProcess = SleepAgent::new(&engine);
        let _config: ConsolidationConfig = SleepConfig::default();
    }
}
