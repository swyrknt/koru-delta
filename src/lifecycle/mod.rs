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
use tokio::time::{interval, Interval};
use tracing::{debug, info, trace, warn};

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

/// Automated lifecycle manager for memory tiers
pub struct LifecycleManager {
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

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            config: config.clone(),
            access_tracker: Arc::new(RwLock::new(AccessTracker::new())),
            importance_scorer: Arc::new(RwLock::new(ImportanceScorer::new(config.ml_scoring_enabled))),
            transition_planner: Arc::new(RwLock::new(TransitionPlanner::new())),
            stats: Arc::new(RwLock::new(LifecycleStats::default())),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with default configuration
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(LifecycleConfig::default())
    }

    /// Start the lifecycle manager
    ///
    /// This spawns background tasks that manage memory tiers
    pub async fn start(&self) {
        info!("Starting automated lifecycle manager");

        let check_interval = self.config.check_interval;
        let consolidation_interval = self.config.consolidation_interval;
        let genome_interval = self.config.genome_interval;

        // Spawn check task
        let check_handle = self.spawn_check_task(check_interval);

        // Spawn consolidation task
        let consolidation_handle = self.spawn_consolidation_task(consolidation_interval);

        // Spawn genome task
        let genome_handle = self.spawn_genome_task(genome_interval);

        // Wait for any task to complete (they run forever unless shutdown)
        tokio::select! {
            _ = check_handle => warn!("Check task exited unexpectedly"),
            _ = consolidation_handle => warn!("Consolidation task exited unexpectedly"),
            _ = genome_handle => warn!("Genome task exited unexpectedly"),
        }
    }

    /// Stop the lifecycle manager
    pub fn stop(&self) {
        info!("Stopping lifecycle manager");
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Record an access for tracking
    pub async fn record_access(&self, key: &FullKey, distinction_id: &DistinctionId) {
        let tracker = self.access_tracker.write().await;
        tracker.record_access(key.clone(), distinction_id.clone());
    }

    /// Get current statistics
    pub async fn stats(&self) -> LifecycleStats {
        self.stats.read().await.clone()
    }

    /// Spawn the periodic check task
    fn spawn_check_task(&self, interval_duration: Duration) -> tokio::task::JoinHandle<()> {
        let tracker = Arc::clone(&self.access_tracker);
        let scorer = Arc::clone(&self.importance_scorer);
        let planner = Arc::clone(&self.transition_planner);
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut int = Self::create_interval(interval_duration);

            loop {
                int.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                trace!("Running lifecycle check");

                // 1. Score all distinctions
                let scores = {
                    let tracker = tracker.read().await;
                    let mut scorer = scorer.write().await;
                    let scores = scorer.score_all(&tracker);
                    
                    // Update stats
                    let mut stats_guard = stats.write().await;
                    stats_guard.distinctions_scored = scores.len() as u64;
                    
                    scores
                };

                // 2. Plan transitions
                let transitions = {
                    let planner = planner.read().await;
                    planner.plan_transitions(&scores)
                };

                // 3. Execute transitions
                for transition in transitions {
                    if let Err(e) = Self::execute_transition(&transition).await {
                        warn!(error = %e, "Failed to execute transition");
                    } else {
                        let mut stats_guard = stats.write().await;
                        stats_guard.transitions_executed += 1;
                    }
                }
            }
        })
    }

    /// Spawn the consolidation task
    fn spawn_consolidation_task(&self, interval_duration: Duration) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut int = Self::create_interval(interval_duration);

            loop {
                int.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                info!("Running memory consolidation");

                if let Err(e) = Self::run_consolidation().await {
                    warn!(error = %e, "Consolidation failed");
                } else {
                    let mut stats_guard = stats.write().await;
                    stats_guard.consolidations_run += 1;
                }
            }
        })
    }

    /// Spawn the genome extraction task
    fn spawn_genome_task(&self, interval_duration: Duration) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut int = Self::create_interval(interval_duration);

            loop {
                int.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                info!("Extracting genome");

                if let Err(e) = Self::run_genome_extraction().await {
                    warn!(error = %e, "Genome extraction failed");
                } else {
                    let mut stats_guard = stats.write().await;
                    stats_guard.genomes_extracted += 1;
                }
            }
        })
    }

    /// Create a tokio interval from a chrono Duration
    fn create_interval(duration: Duration) -> Interval {
        let secs = duration.num_seconds().max(1) as u64;
        interval(tokio::time::Duration::from_secs(secs))
    }
    
    /// Execute a single transition
    async fn execute_transition(
        _transition: &Transition,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Placeholder - actual implementation would move data between tiers
        Ok(())
    }
    
    /// Run consolidation across all tiers
    async fn run_consolidation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Running consolidation");
        Ok(())
    }
    
    /// Run genome extraction
    async fn run_genome_extraction() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Running genome extraction");
        Ok(())
    }
}

/// Importance scorer that uses ML or heuristics
pub struct ImportanceScorer {
    ml_enabled: bool,
    model: Option<ImportanceModel>,
}

impl ImportanceScorer {
    /// Create a new importance scorer
    pub fn new(ml_enabled: bool) -> Self {
        Self {
            ml_enabled,
            model: if ml_enabled { Some(ImportanceModel::new()) } else { None },
        }
    }

    /// Score all distinctions based on access patterns
    pub fn score_all(&mut self, tracker: &AccessTracker) -> HashMap<DistinctionId, ImportanceScore> {
        if self.ml_enabled && self.model.is_some() {
            // Use ML model for scoring
            self.model.as_ref().unwrap().predict_all(tracker)
        } else {
            // Use heuristic scoring
            self.heuristic_score_all(tracker)
        }
    }

    /// Heuristic scoring (fallback when ML is disabled)
    fn heuristic_score_all(&self, tracker: &AccessTracker) -> HashMap<DistinctionId, ImportanceScore> {
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
    async fn test_lifecycle_manager_creation() {
        let manager = LifecycleManager::default();

        let stats = manager.stats().await;
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
