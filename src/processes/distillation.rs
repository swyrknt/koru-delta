/// Evolution Agent: Natural selection of distinctions with LCA architecture.
///
/// This agent "evolves" history by keeping only the "fit" distinctions
/// and archiving/discarding the "unfit" ones. Like evolution selecting
/// for beneficial traits.
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Evolution Agent's local root is `RootType::Evolution` (🧬 EVOLUTION).
///
/// ## Fitness Criteria
///
/// - High reference count = fit (many things point to it)
/// - Many causal descendants = fit (important in history)
/// - Recent = fit (still relevant)
/// - Low fitness = archive to Deep or discard
///
/// ## The Algorithm
///
/// 1. Score each distinction's fitness
/// 2. Classify as fit or unfit
/// 3. Keep fit distinctions in working memory
/// 4. Archive unfit to Deep (or discard if truly unimportant)
use crate::actions::EvolutionAction;
use crate::causal_graph::CausalGraph;
use crate::engine::{FieldHandle, SharedEngine};
use crate::memory::{ColdMemory, DeepMemory};
use crate::reference_graph::ReferenceGraph;
use crate::roots::RootType;
use chrono::{DateTime, Duration, Utc};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Evolution agent configuration.
#[derive(Debug, Clone)]
pub struct EvolutionConfig {
    /// How often to run evolution (seconds)
    pub interval_secs: u64,

    /// Fitness threshold (references >= this are "fit")
    pub fitness_threshold: usize,

    /// Age threshold (older than this loses fitness)
    pub age_threshold: Duration,

    /// Descendant bonus (each descendant adds this much fitness)
    pub descendant_bonus: usize,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            interval_secs: 3600,              // Hourly
            fitness_threshold: 2,             // 2+ references = fit
            age_threshold: Duration::days(7), // Week old = less fit
            descendant_bonus: 1,              // Each descendant adds 1
        }
    }
}

/// Evolution Agent - natural selection of distinctions with LCA architecture.
///
/// Like evolution: fitness-based selection, survival of the fittest.
/// All operations are synthesized through the unified field.
#[derive(Debug)]
pub struct EvolutionAgent {
    /// Configuration
    config: EvolutionConfig,

    /// LCA: Local root distinction (Root: EVOLUTION)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// Statistics
    distinctions_evaluated: AtomicU64,
    distinctions_preserved: AtomicU64,
    distinctions_archived: AtomicU64,
}

/// Fitness score for a distinction.
#[derive(Debug, Clone)]
pub struct Fitness {
    pub distinction_id: String,
    pub reference_count: usize,
    pub descendant_count: usize,
    pub age_days: i64,
    pub total_score: i64,
}

impl EvolutionAgent {
    /// Create new evolution agent.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Evolution (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self::with_config(EvolutionConfig::default(), shared_engine)
    }

    /// Create with custom config.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the EVOLUTION root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(config: EvolutionConfig, shared_engine: &SharedEngine) -> Self {
        let local_root = shared_engine.root(RootType::Evolution).clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            config,
            local_root,
            field,
            distinctions_evaluated: AtomicU64::new(0),
            distinctions_preserved: AtomicU64::new(0),
            distinctions_archived: AtomicU64::new(0),
        }
    }

    /// Calculate fitness for a distinction.
    ///
    /// Fitness = references + descendants - age_penalty
    ///
    /// # LCA Pattern
    ///
    /// Evaluation synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔEvaluateFitness_Action`
    pub fn calculate_fitness(
        &self,
        distinction_id: &str,
        reference_graph: &ReferenceGraph,
        causal_graph: &CausalGraph,
        timestamp: DateTime<Utc>,
    ) -> Fitness {
        // Synthesize evaluate fitness action
        let action = EvolutionAction::EvaluateFitness {
            candidate_id: distinction_id.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        let reference_count = reference_graph.reference_count(&distinction_id.to_string());
        let descendant_count = causal_graph.descendants(distinction_id).len();
        let age = Utc::now().signed_duration_since(timestamp);
        let age_days = age.num_days();

        // Calculate score
        let mut score = reference_count as i64;
        score += (descendant_count * self.config.descendant_bonus) as i64;

        // Age penalty: lose 1 point per day after threshold
        if age > self.config.age_threshold {
            let excess_days = (age - self.config.age_threshold).num_days();
            score -= excess_days;
        }

        Fitness {
            distinction_id: distinction_id.to_string(),
            reference_count,
            descendant_count,
            age_days,
            total_score: score,
        }
    }

    /// Classify distinctions as fit or unfit.
    ///
    /// # LCA Pattern
    ///
    /// Classification synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔSelect_Action`
    pub fn classify_distinctions(
        &self,
        distinctions: &[(String, DateTime<Utc>)], // (id, timestamp)
        reference_graph: &ReferenceGraph,
        causal_graph: &CausalGraph,
    ) -> Classification {
        let ids: Vec<String> = distinctions.iter().map(|(id, _)| id.clone()).collect();

        // Synthesize select action
        let action = EvolutionAction::Select {
            population_ids: ids.clone(),
        };
        let _ = self.synthesize_action_internal(action);

        let mut fit = Vec::new();
        let mut unfit = Vec::new();

        for (id, timestamp) in distinctions {
            let fitness = self.calculate_fitness(id, reference_graph, causal_graph, *timestamp);
            self.distinctions_evaluated.fetch_add(1, Ordering::Relaxed);

            if fitness.total_score >= self.config.fitness_threshold as i64 {
                fit.push(fitness);
            } else {
                unfit.push(fitness);
            }
        }

        Classification { fit, unfit }
    }

    /// Evolve a cold epoch - keep fit, archive unfit.
    ///
    /// # LCA Pattern
    ///
    /// Evolution synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔPreserve_Action` for fit
    /// and `ΔNew = ΔLocal_Root ⊕ ΔArchive_Action` for unfit
    pub fn evolve_epoch(
        &self,
        _cold: &ColdMemory,
        deep: &DeepMemory,
        epoch_num: usize,
        _reference_graph: &ReferenceGraph,
        _causal_graph: &CausalGraph,
    ) -> EvolutionResult {
        // TODO: In real implementation, would iterate over epoch's distinctions
        // For now, placeholder

        let preserved = 0;
        let archived = 0;

        // Archive the epoch
        deep.archive_epoch(
            format!("epoch_{}", epoch_num),
            0, // Would be actual count
            0, // Would be actual size
        );

        self.distinctions_preserved
            .fetch_add(preserved, Ordering::Relaxed);
        self.distinctions_archived
            .fetch_add(archived, Ordering::Relaxed);

        EvolutionResult {
            distinctions_preserved: preserved,
            distinctions_archived: archived,
        }
    }

    /// Preserve fit distinctions.
    ///
    /// # LCA Pattern
    ///
    /// Preservation synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔPreserve_Action`
    pub fn preserve(&self, fit_ids: &[String]) {
        // Synthesize preserve action
        let action = EvolutionAction::Preserve {
            fit_ids: fit_ids.to_vec(),
        };
        let _ = self.synthesize_action_internal(action);

        self.distinctions_preserved
            .fetch_add(fit_ids.len() as u64, Ordering::Relaxed);
    }

    /// Archive unfit distinctions.
    ///
    /// # LCA Pattern
    ///
    /// Archival synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔArchive_Action`
    pub fn archive_unfit(&self, unfit_ids: &[String]) {
        // Synthesize archive action
        let action = EvolutionAction::Archive {
            unfit_ids: unfit_ids.to_vec(),
        };
        let _ = self.synthesize_action_internal(action);

        self.distinctions_archived
            .fetch_add(unfit_ids.len() as u64, Ordering::Relaxed);
    }

    /// Get the fitness threshold.
    pub fn fitness_threshold(&self) -> usize {
        self.config.fitness_threshold
    }

    /// Get interval.
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.interval_secs)
    }

    /// Get statistics.
    pub fn stats(&self) -> EvolutionStats {
        EvolutionStats {
            distinctions_evaluated: self.distinctions_evaluated.load(Ordering::Relaxed),
            distinctions_preserved: self.distinctions_preserved.load(Ordering::Relaxed),
            distinctions_archived: self.distinctions_archived.load(Ordering::Relaxed),
            preservation_rate: self.preservation_rate(),
        }
    }

    /// Calculate preservation rate (0.0 to 1.0).
    fn preservation_rate(&self) -> f64 {
        let evaluated = self.distinctions_evaluated.load(Ordering::Relaxed);
        let preserved = self.distinctions_preserved.load(Ordering::Relaxed);

        if evaluated == 0 {
            0.0
        } else {
            preserved as f64 / evaluated as f64
        }
    }

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: EvolutionAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        engine.synthesize(&self.local_root, &action_distinction)
    }
}

impl Default for EvolutionAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use EvolutionAgent::new(&shared_engine)
        panic!("EvolutionAgent requires a SharedEngine - use EvolutionAgent::new()")
    }
}

/// LCA Trait Implementation for EvolutionAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for EvolutionAgent {
    type ActionData = EvolutionAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: EvolutionAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

/// Classification result.
#[derive(Debug, Clone)]
pub struct Classification {
    pub fit: Vec<Fitness>,
    pub unfit: Vec<Fitness>,
}

/// Evolution result.
#[derive(Debug, Clone)]
pub struct EvolutionResult {
    pub distinctions_preserved: u64,
    pub distinctions_archived: u64,
}

/// Evolution statistics.
#[derive(Debug, Clone)]
pub struct EvolutionStats {
    pub distinctions_evaluated: u64,
    pub distinctions_preserved: u64,
    pub distinctions_archived: u64,
    pub preservation_rate: f64,
}

/// Backward-compatible type alias for existing code.
pub type DistillationProcess = EvolutionAgent;

/// Backward-compatible type alias for existing code.
pub type DistillationConfig = EvolutionConfig;

/// Backward-compatible type alias for existing code.
pub type DistillationResult = EvolutionResult;

/// Backward-compatible type alias for existing code.
pub type DistillationStats = EvolutionStats;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_graph::CausalGraph;
    use crate::engine::SharedEngine;
    use crate::reference_graph::ReferenceGraph;

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

    #[test]
    fn test_calculate_fitness() {
        let engine = create_test_engine();
        let evolution = EvolutionAgent::new(&engine);
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new(&create_test_engine());

        // Add node with no references
        ref_graph.add_node("test".to_string());
        causal_graph.add_node("test".to_string());

        let fitness = evolution.calculate_fitness(
            "test",
            &ref_graph,
            &causal_graph,
            Utc::now(), // Now, so no age penalty
        );

        assert_eq!(fitness.reference_count, 0);
        assert_eq!(fitness.total_score, 0);
    }

    #[test]
    fn test_fitness_with_references() {
        let engine = create_test_engine();
        let evolution = EvolutionAgent::new(&engine);
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new(&create_test_engine());

        // Setup: test2 references test1
        ref_graph.add_node("test1".to_string());
        ref_graph.add_node("test2".to_string());
        ref_graph.add_reference("test2".to_string(), "test1".to_string());

        causal_graph.add_node("test1".to_string());

        let fitness =
            evolution.calculate_fitness("test1", &ref_graph, &causal_graph, Utc::now());

        assert_eq!(fitness.reference_count, 1);
        assert_eq!(fitness.total_score, 1);
    }

    #[test]
    fn test_classify_distinctions() {
        let engine = create_test_engine();
        let evolution = EvolutionAgent::with_config(
            EvolutionConfig {
                fitness_threshold: 2,
                ..Default::default()
            },
            &engine,
        );
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new(&create_test_engine());

        // Setup distinctions
        let distinctions = vec![
            ("high_fit".to_string(), Utc::now()),
            ("low_fit".to_string(), Utc::now()),
        ];

        // Add references: high_fit has 3, low_fit has 0
        ref_graph.add_node("high_fit".to_string());
        ref_graph.add_node("low_fit".to_string());
        ref_graph.add_node("ref1".to_string());
        ref_graph.add_node("ref2".to_string());
        ref_graph.add_node("ref3".to_string());
        ref_graph.add_reference("ref1".to_string(), "high_fit".to_string());
        ref_graph.add_reference("ref2".to_string(), "high_fit".to_string());
        ref_graph.add_reference("ref3".to_string(), "high_fit".to_string());

        causal_graph.add_node("high_fit".to_string());
        causal_graph.add_node("low_fit".to_string());

        let classification =
            evolution.classify_distinctions(&distinctions, &ref_graph, &causal_graph);

        assert_eq!(classification.fit.len(), 1);
        assert_eq!(classification.unfit.len(), 1);
        assert_eq!(classification.fit[0].distinction_id, "high_fit");
    }

    #[test]
    fn test_age_penalty() {
        let engine = create_test_engine();
        let evolution = EvolutionAgent::new(&engine);
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new(&create_test_engine());

        ref_graph.add_node("old".to_string());
        causal_graph.add_node("old".to_string());

        // Created 30 days ago
        let old_timestamp = Utc::now() - Duration::days(30);

        let fitness =
            evolution.calculate_fitness("old", &ref_graph, &causal_graph, old_timestamp);

        // Should have negative score due to age (30 - 7 = 23 days over threshold)
        assert!(fitness.total_score < 0);
    }

    #[test]
    fn test_stats() {
        let engine = create_test_engine();
        let evolution = EvolutionAgent::new(&engine);

        // Simulate some evaluations
        let ref_graph = ReferenceGraph::new();
        let causal_graph = CausalGraph::new(&create_test_engine());

        for i in 0..10 {
            let id = format!("test{}", i);
            ref_graph.add_node(id.clone());
            causal_graph.add_node(id);
        }

        let distinctions: Vec<_> = (0..10)
            .map(|i| (format!("test{}", i), Utc::now()))
            .collect();

        let _ = evolution.classify_distinctions(&distinctions, &ref_graph, &causal_graph);

        let stats = evolution.stats();
        assert_eq!(stats.distinctions_evaluated, 10);
    }

    #[test]
    fn test_preserve_and_archive() {
        let engine = create_test_engine();
        let evolution = EvolutionAgent::new(&engine);

        let fit = vec!["fit1".to_string(), "fit2".to_string()];
        let unfit = vec!["unfit1".to_string()];

        evolution.preserve(&fit);
        evolution.archive_unfit(&unfit);

        let stats = evolution.stats();
        assert_eq!(stats.distinctions_preserved, 2);
        assert_eq!(stats.distinctions_archived, 1);
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = EvolutionAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = EvolutionAction::EvaluateFitness {
            candidate_id: "test123".to_string(),
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
        let _distillation: DistillationProcess = EvolutionAgent::new(&engine);
        let _config: DistillationConfig = EvolutionConfig::default();
    }
}
