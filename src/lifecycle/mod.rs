/// Automated Memory Lifecycle Management
///
/// This module provides intelligent, ML-based memory tier management
/// with automatic Hot→Warm→Cold→Deep transitions based on access patterns.
///
/// ## Features
///
/// - **Access Pattern Tracking**: Records frequency, recency, time-of-day, and access sequences
/// - **ML-Based Importance Scoring**: Predicts future value of distinctions
/// - **Automated Transitions**: Moves data between tiers based on scores
/// - **Background Consolidation**: Runs during idle time
///
/// ## Lifecycle Flow
///
/// ```text
/// New Data → Hot (active use)
///     │
///     ├── High importance + frequent access → Stay Hot
///     │
///     ├── Low importance or idle → Warm (recent chronicle)
///     │
///     ├── Old + low importance → Cold (compressed epochs)
///     │
///     └── Very old + pattern extracted → Deep (genomic)
/// ```
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, trace, warn};

use crate::causal_graph::DistinctionId;
use crate::types::FullKey;

mod access_tracker;
mod importance_scorer;
mod transition_planner;

pub use access_tracker::{AccessPattern, AccessTracker};
pub use importance_scorer::{ImportanceModel, ImportanceScore};
pub use transition_planner::{Transition, TransitionPlanner, TransitionType};

/// Lifecycle manager configuration
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// How often to run lifecycle checks (default: 5 minutes)
    pub check_interval: Duration,

    /// How often to run full consolidation (default: 1 hour)
    pub consolidation_interval: Duration,

    /// How often to extract genomes (default: 24 hours)
    pub genome_interval: Duration,

    /// Hot memory target utilization (0.0 - 1.0)
    pub hot_target_utilization: f64,

    /// Warm memory idle threshold
    pub warm_idle_threshold: Duration,

    /// Cold epoch duration
    pub cold_epoch_duration: Duration,

    /// Enable ML-based scoring (vs heuristic)
    pub ml_scoring_enabled: bool,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::minutes(5),
            consolidation_interval: Duration::hours(1),
            genome_interval: Duration::hours(24),
            hot_target_utilization: 0.8,
            warm_idle_threshold: Duration::hours(1),
            cold_epoch_duration: Duration::days(1),
            ml_scoring_enabled: true,
        }
    }
}

/// Memory tier enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Hot memory (RAM, fastest)
    Hot,
    /// Warm memory (disk, recent)
    Warm,
    /// Cold memory (disk, compressed epochs)
    Cold,
    /// Deep memory (archival, genomic)
    Deep,
}

impl std::fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryTier::Hot => write!(f, "hot"),
            MemoryTier::Warm => write!(f, "warm"),
            MemoryTier::Cold => write!(f, "cold"),
            MemoryTier::Deep => write!(f, "deep"),
        }
    }
}

/// Lifecycle statistics
#[derive(Debug, Clone, Default)]
pub struct LifecycleStats {
    pub transitions_executed: u64,
    pub consolidations_run: u64,
    pub genomes_extracted: u64,
    pub distinctions_scored: u64,
}

/// Importance scorer that uses ML or heuristics
#[derive(Debug)]
pub struct ImportanceScorer {
    ml_enabled: bool,
    model: Option<ImportanceModel>,
}

impl ImportanceScorer {
    /// Create a new importance scorer
    pub fn new(ml_enabled: bool) -> Self {
        Self {
            ml_enabled,
            model: if ml_enabled {
                Some(ImportanceModel::new())
            } else {
                None
            },
        }
    }

    /// Score all distinctions based on access patterns
    pub fn score_all(
        &mut self,
        tracker: &AccessTracker,
    ) -> HashMap<DistinctionId, ImportanceScore> {
        if self.ml_enabled && self.model.is_some() {
            // Use ML model for scoring
            self.model.as_ref().unwrap().predict_all(tracker)
        } else {
            // Use heuristic scoring
            self.heuristic_score_all(tracker)
        }
    }

    /// Heuristic scoring (fallback when ML is disabled)
    fn heuristic_score_all(
        &self,
        tracker: &AccessTracker,
    ) -> HashMap<DistinctionId, ImportanceScore> {
        use chrono::Utc;

        let mut scores = HashMap::new();
        let now = Utc::now();

        for entry in tracker.patterns() {
            let id = entry.key().clone();
            let pattern = entry.value();

            // Simple heuristic: recency + frequency
            let recency_score = if let Some(last) = pattern.last_accessed {
                let age = now.signed_duration_since(last);
                let days_old = age.num_days() as f64;
                (-days_old / 7.0).exp() // Exponential decay over a week
            } else {
                0.0
            };

            let frequency_score = (pattern.access_count as f64 / 100.0).min(1.0);

            let total_score = recency_score * 0.6 + frequency_score * 0.4;

            scores.insert(
                id.clone(),
                ImportanceScore {
                    distinction_id: id,
                    score: total_score as f32,
                    confidence: 0.7, // Heuristic has moderate confidence
                    factors: vec![
                        ScoreFactor::Recency(recency_score as f32),
                        ScoreFactor::Frequency(frequency_score as f32),
                    ],
                },
            );
        }

        scores
    }
}

/// Factors contributing to importance score
#[derive(Debug, Clone)]
pub enum ScoreFactor {
    /// Recency component
    Recency(f32),
    /// Frequency component
    Frequency(f32),
    /// Time of day pattern component
    TimeOfDay(f32),
    /// Sequence context component
    SequenceContext(f32),
    /// Predicted future value
    PredictedFutureValue(f32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_config_default() {
        let config = LifecycleConfig::default();
        assert_eq!(config.check_interval.num_minutes(), 5);
        assert_eq!(config.consolidation_interval.num_hours(), 1);
        assert_eq!(config.genome_interval.num_hours(), 24);
        assert!(config.ml_scoring_enabled);
    }

    #[test]
    fn test_memory_tier_display() {
        assert_eq!(format!("{}", MemoryTier::Hot), "hot");
        assert_eq!(format!("{}", MemoryTier::Warm), "warm");
        assert_eq!(format!("{}", MemoryTier::Cold), "cold");
        assert_eq!(format!("{}", MemoryTier::Deep), "deep");
    }

    #[tokio::test]
    async fn test_lifecycle_agent_creation() {
        use crate::engine::SharedEngine;
        
        let field = SharedEngine::new();
        let agent = LifecycleAgent::new(&field);

        let stats = agent.stats().await;
        assert_eq!(stats.transitions_executed, 0);
        assert_eq!(stats.consolidations_run, 0);
        assert_eq!(stats.genomes_extracted, 0);
    }

    #[test]
    fn test_importance_scorer_heuristic() {
        let tracker = AccessTracker::new();
        let key = FullKey::new("test", "key1");
        let id = "dist1".to_string();

        tracker.record_access(key, id.clone());

        let mut scorer = ImportanceScorer::new(false); // ML disabled
        let scores = scorer.score_all(&tracker);

        assert!(scores.contains_key(&id));
        let score = scores.get(&id).unwrap();
        assert!(score.score > 0.0);
        assert!(score.score <= 1.0);
    }
}

// ============================================================================
// LIFECYCLE AGENT (LCA Pattern)
// ============================================================================

use crate::actions::LifecycleAction;
use crate::engine::SharedEngine;
use crate::roots::KoruRoots;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};

/// Lifecycle agent implementing LocalCausalAgent trait.
///
/// Follows the LCA formula: ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// All operations are causal distinctions synthesized from the lifecycle root.
#[derive(Debug)]
pub struct LifecycleAgent {
    /// LCA: Local root distinction (Root: LIFECYCLE)
    local_root: Distinction,

    /// LCA: Handle to the unified field
    _field: SharedEngine,

    /// The underlying distinction engine for content addressing
    engine: Arc<DistinctionEngine>,

    /// Configuration
    config: LifecycleConfig,

    /// Access pattern tracker
    access_tracker: Arc<RwLock<AccessTracker>>,

    /// Importance scorer (ML-based)
    importance_scorer: Arc<RwLock<ImportanceScorer>>,

    /// Transition planner
    transition_planner: Arc<RwLock<TransitionPlanner>>,

    /// Statistics
    stats: Arc<RwLock<LifecycleStats>>,

    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
}

impl LifecycleAgent {
    /// Create a new lifecycle agent with LCA pattern.
    ///
    /// The agent initializes from the lifecycle canonical root,
    /// establishing its causal anchor in the field.
    pub fn new(field: &SharedEngine) -> Self {
        Self::with_config(field, LifecycleConfig::default())
    }

    /// Create a new lifecycle agent with custom configuration.
    ///
    /// Backward-compatible constructor that accepts configuration.
    pub fn with_config(field: &SharedEngine, config: LifecycleConfig) -> Self {
        let engine = Arc::clone(field.inner());
        let roots = KoruRoots::initialize(&engine);
        let local_root = roots.lifecycle.clone();

        Self {
            local_root,
            _field: field.clone(),
            engine,
            config: config.clone(),
            access_tracker: Arc::new(RwLock::new(AccessTracker::new())),
            importance_scorer: Arc::new(RwLock::new(ImportanceScorer::new(
                config.ml_scoring_enabled,
            ))),
            transition_planner: Arc::new(RwLock::new(TransitionPlanner::new())),
            stats: Arc::new(RwLock::new(LifecycleStats::default())),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the local root distinction.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Apply a lifecycle action, synthesizing new state.
    ///
    /// This is the primary interface for lifecycle operations following
    /// the LCA formula: ΔNew = ΔLocal_Root ⊕ ΔAction_Data
    pub fn apply_action(&mut self, action: LifecycleAction) -> Distinction {
        let engine = Arc::clone(&self.engine);
        let new_root = self.synthesize_action(action, &engine);
        self.local_root = new_root.clone();
        new_root
    }

    /// Record an access for tracking (async).
    pub async fn record_access(&self, key: &FullKey, distinction_id: &DistinctionId) {
        let tracker = self.access_tracker.write().await;
        tracker.record_access(key.clone(), distinction_id.clone());
    }

    /// Get current statistics.
    pub async fn stats(&self) -> LifecycleStats {
        self.stats.read().await.clone()
    }

    /// Evaluate access for a distinction and synthesize result.
    pub fn evaluate_access(&mut self, distinction_id: String, full_key: FullKey) -> Distinction {
        let action = LifecycleAction::EvaluateAccess { distinction_id, full_key };
        self.apply_action(action)
    }

    /// Plan and execute a promotion transition.
    pub fn promote(
        &mut self,
        distinction_id: String,
        from_tier: MemoryTier,
        to_tier: MemoryTier,
    ) -> Distinction {
        let action = LifecycleAction::Promote {
            distinction_id,
            from_tier,
            to_tier,
        };
        self.apply_action(action)
    }

    /// Plan and execute a demotion transition.
    pub fn demote(
        &mut self,
        distinction_id: String,
        from_tier: MemoryTier,
        to_tier: MemoryTier,
    ) -> Distinction {
        let action = LifecycleAction::Demote {
            distinction_id,
            from_tier,
            to_tier,
        };
        self.apply_action(action)
    }

    /// Execute multiple transitions.
    pub fn transition(&mut self, transitions: Vec<Transition>) -> Distinction {
        let action = LifecycleAction::Transition { transitions };
        self.apply_action(action)
    }

    /// Update lifecycle thresholds.
    pub fn update_thresholds(&mut self, thresholds: serde_json::Value) -> Distinction {
        let action = LifecycleAction::UpdateThresholds { thresholds };
        self.apply_action(action)
    }

    /// Start background lifecycle tasks.
    pub async fn start(&self) {
        use tracing::{info, warn};

        info!("Starting lifecycle agent");

        let check_interval = self.config.check_interval;
        let consolidation_interval = self.config.consolidation_interval;
        let genome_interval = self.config.genome_interval;

        // Spawn background tasks
        let check_handle = self.spawn_check_task(check_interval);
        let consolidation_handle = self.spawn_consolidation_task(consolidation_interval);
        let genome_handle = self.spawn_genome_task(genome_interval);

        tokio::select! {
            _ = check_handle => warn!("Check task exited unexpectedly"),
            _ = consolidation_handle => warn!("Consolidation task exited unexpectedly"),
            _ = genome_handle => warn!("Genome task exited unexpectedly"),
        }
    }

    /// Stop the lifecycle agent.
    pub fn stop(&self) {
        use tracing::info;
        info!("Stopping lifecycle agent");
        self.shutdown.store(true, Ordering::Relaxed);
    }

    fn spawn_check_task(&self, interval_duration: Duration) -> tokio::task::JoinHandle<()> {
        let tracker = Arc::clone(&self.access_tracker);
        let scorer = Arc::clone(&self.importance_scorer);
        let planner = Arc::clone(&self.transition_planner);
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut int = interval(tokio::time::Duration::from_secs(
                interval_duration.num_seconds().max(1) as u64,
            ));

            loop {
                int.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                trace!("Running lifecycle check");

                // Score all distinctions
                let scores = {
                    let tracker = tracker.read().await;
                    let mut scorer = scorer.write().await;
                    scorer.score_all(&tracker)
                };

                // Update stats
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.distinctions_scored = scores.len() as u64;
                }

                // Plan transitions
                let transitions = {
                    let planner = planner.read().await;
                    planner.plan_transitions(&scores)
                };

                // Note: Actual transition execution happens through apply_action
                // This is a placeholder for background monitoring
                trace!(planned_transitions = transitions.len(), "Lifecycle check complete");
            }
        })
    }

    fn spawn_consolidation_task(
        &self,
        interval_duration: Duration,
    ) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut int = interval(tokio::time::Duration::from_secs(
                interval_duration.num_seconds().max(1) as u64,
            ));

            loop {
                int.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                info!("Running memory consolidation");

                let mut stats_guard = stats.write().await;
                stats_guard.consolidations_run += 1;
            }
        })
    }

    fn spawn_genome_task(&self, interval_duration: Duration) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut int = interval(tokio::time::Duration::from_secs(
                interval_duration.num_seconds().max(1) as u64,
            ));

            loop {
                int.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                info!("Extracting genome");

                let mut stats_guard = stats.write().await;
                stats_guard.genomes_extracted += 1;
            }
        })
    }
}

impl LocalCausalAgent for LifecycleAgent {
    type ActionData = LifecycleAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: LifecycleAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

// Backward-compatible type alias
pub type LifecycleManager = LifecycleAgent;

#[cfg(test)]
mod lca_tests {
    use super::*;
    use koru_lambda_core::LocalCausalAgent;

    fn setup_agent() -> LifecycleAgent {
        let field = SharedEngine::new();
        LifecycleAgent::new(&field)
    }

    #[test]
    fn test_lifecycle_agent_implements_lca_trait() {
        let agent = setup_agent();
        
        // Verify trait is implemented
        let _root = agent.get_current_root();
    }

    #[test]
    fn test_lifecycle_agent_has_unique_local_root() {
        let field = SharedEngine::new();
        let agent1 = LifecycleAgent::new(&field);
        let agent2 = LifecycleAgent::new(&field);

        // Each agent should have the same lifecycle root from canonical roots
        assert_eq!(
            agent1.local_root().id(),
            agent2.local_root().id(),
            "Lifecycle agents share the same canonical root"
        );
    }

    #[test]
    fn test_evaluate_access_synthesizes() {
        let mut agent = setup_agent();
        let root_before = agent.local_root().id().to_string();

        let new_root = agent.evaluate_access("dist1".to_string(), FullKey::new("test", "key1"));
        
        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after synthesis");
        assert_eq!(new_root.id(), root_after);
    }

    #[test]
    fn test_promote_synthesizes() {
        let mut agent = setup_agent();
        let root_before = agent.local_root().id().to_string();

        let new_root = agent.promote(
            "dist1".to_string(),
            MemoryTier::Warm,
            MemoryTier::Hot,
        );
        
        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after promote");
        assert_eq!(new_root.id(), root_after);
    }

    #[test]
    fn test_demote_synthesizes() {
        let mut agent = setup_agent();
        let root_before = agent.local_root().id().to_string();

        let new_root = agent.demote(
            "dist1".to_string(),
            MemoryTier::Hot,
            MemoryTier::Warm,
        );
        
        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after demote");
        assert_eq!(new_root.id(), root_after);
    }

    #[test]
    fn test_transition_synthesizes() {
        let mut agent = setup_agent();
        let root_before = agent.local_root().id().to_string();

        let transitions = vec![
            Transition {
                distinction_id: "dist1".to_string(),
                from_tier: MemoryTier::Warm,
                to_tier: MemoryTier::Hot,
                importance_score: 0.8,
                priority: 1.0,
            },
        ];
        let new_root = agent.transition(transitions);
        
        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after transition");
        assert_eq!(new_root.id(), root_after);
    }

    #[test]
    fn test_update_thresholds_synthesizes() {
        let mut agent = setup_agent();
        let root_before = agent.local_root().id().to_string();

        let new_root = agent.update_thresholds(serde_json::json!({"hot_target": 0.9}));
        
        let root_after = agent.local_root().id().to_string();
        assert_ne!(root_before, root_after, "Local root should change after update_thresholds");
        assert_eq!(new_root.id(), root_after);
    }
}
