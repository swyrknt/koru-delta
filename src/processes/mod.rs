/// Evolutionary processes for automated memory management.
///
/// These processes run continuously to maintain the memory hierarchy:
/// - Consolidation: rhythmic movement between layers
/// - Distillation: fitness-based natural selection
/// - GenomeUpdate: DNA maintenance and disaster recovery
///
/// ## LCA Architecture
///
/// ProcessAgent implements `LocalCausalAgent`, making all process operations
/// causal distinctions. The formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
pub mod consolidation;
pub mod distillation;
pub mod genome_update;

use crate::actions::{ProcessAction, ProcessConfig, ProcessType};
use crate::engine::SharedEngine;
use crate::roots::KoruRoots;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use std::sync::Arc;

pub use consolidation::{ConsolidationResult, SleepAgent, SleepConfig};
pub use distillation::{EvolutionAgent, EvolutionConfig, EvolutionResult, EvolutionStats, Fitness};
pub use genome_update::{GenomeUpdateConfig, GenomeUpdateProcess};

/// Process agent implementing LocalCausalAgent trait.
///
/// Follows the LCA formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
/// All process operations are causal distinctions synthesized from the process root.
#[derive(Debug)]
pub struct ProcessAgent {
    /// LCA: Local root distinction (Root: PROCESS)
    local_root: Distinction,

    /// LCA: Handle to the unified field
    _field: SharedEngine,

    /// The underlying distinction engine for content addressing
    engine: Arc<DistinctionEngine>,

    /// Consolidation process
    consolidation: SleepAgent,

    /// Distillation process
    distillation: EvolutionAgent,

    /// Genome update process
    genome_update: GenomeUpdateProcess,
}

impl ProcessAgent {
    /// Create new process agent with default configs.
    ///
    /// The agent initializes from the process canonical root,
    /// establishing its causal anchor in the field.
    pub fn new(shared_engine: &SharedEngine) -> Self {
        let engine = Arc::clone(shared_engine.inner());
        let roots = KoruRoots::initialize(&engine);
        let local_root = roots.process.clone();

        Self {
            local_root,
            _field: shared_engine.clone(),
            engine,
            consolidation: SleepAgent::new(shared_engine),
            distillation: EvolutionAgent::new(shared_engine),
            genome_update: GenomeUpdateProcess::new(),
        }
    }

    /// Create with custom configurations.
    pub fn with_config(
        consolidation: SleepConfig,
        distillation: EvolutionConfig,
        genome: GenomeUpdateConfig,
        shared_engine: &SharedEngine,
    ) -> Self {
        let engine = Arc::clone(shared_engine.inner());
        let roots = KoruRoots::initialize(&engine);
        let local_root = roots.process.clone();

        Self {
            local_root,
            _field: shared_engine.clone(),
            engine,
            consolidation: SleepAgent::with_config(consolidation, shared_engine),
            distillation: EvolutionAgent::with_config(distillation, shared_engine),
            genome_update: GenomeUpdateProcess::with_config(genome),
        }
    }

    /// Get the local root distinction.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Apply a process action, synthesizing new state.
    ///
    /// This is the primary interface for process operations following
    /// the LCA formula: `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`
    pub fn apply_action(&mut self, action: ProcessAction) -> Distinction {
        let engine = Arc::clone(&self.engine);
        let new_root = self.synthesize_action(action, &engine);
        self.local_root = new_root.clone();
        new_root
    }

    /// Get the consolidation process.
    pub fn consolidation(&self) -> &SleepAgent {
        &self.consolidation
    }

    /// Get the distillation process.
    pub fn distillation(&self) -> &EvolutionAgent {
        &self.distillation
    }

    /// Get the genome update process.
    pub fn genome_update(&self) -> &GenomeUpdateProcess {
        &self.genome_update
    }

    /// Spawn a process with synthesis.
    pub fn spawn_process_synthesized(
        &mut self,
        process_type: ProcessType,
        config: ProcessConfig,
    ) -> Distinction {
        let action = ProcessAction::SpawnProcess {
            process_type,
            config,
        };
        self.apply_action(action)
    }

    /// Pause a process with synthesis.
    pub fn pause_process_synthesized(&mut self, process_id: String) -> Distinction {
        let action = ProcessAction::PauseProcess { process_id };
        self.apply_action(action)
    }

    /// Resume a process with synthesis.
    pub fn resume_process_synthesized(&mut self, process_id: String) -> Distinction {
        let action = ProcessAction::ResumeProcess { process_id };
        self.apply_action(action)
    }

    /// Terminate a process with synthesis.
    pub fn terminate_process_synthesized(&mut self, process_id: String) -> Distinction {
        let action = ProcessAction::TerminateProcess { process_id };
        self.apply_action(action)
    }

    /// Send heartbeat with synthesis.
    pub fn heartbeat_synthesized(&mut self, process_id: String) -> Distinction {
        let action = ProcessAction::Heartbeat { process_id };
        self.apply_action(action)
    }

    /// Get status with synthesis.
    pub fn get_status_synthesized(&mut self, process_id: String) -> Distinction {
        let action = ProcessAction::GetStatus { process_id };
        self.apply_action(action)
    }

    /// List processes with synthesis.
    pub fn list_processes_synthesized(&mut self) -> Distinction {
        let action = ProcessAction::ListProcesses;
        self.apply_action(action)
    }
}

// LCA Trait Implementation
impl LocalCausalAgent for ProcessAgent {
    type ActionData = ProcessAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: ProcessAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

impl Default for ProcessAgent {
    fn default() -> Self {
        let field = SharedEngine::new();
        Self::new(&field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

    #[test]
    fn test_process_runner_new() {
        let engine = create_test_engine();
        let runner = ProcessAgent::new(&engine);

        // Should have all three processes
        assert_eq!(runner.consolidation().cycle_count(), 0);
        assert_eq!(runner.distillation().stats().distinctions_evaluated, 0);
        assert_eq!(runner.genome_update().stats().updates_performed, 0);
    }

    #[test]
    fn test_process_runner_with_config() {
        let engine = create_test_engine();
        let config = ProcessAgent::with_config(
            SleepConfig {
                interval_secs: 3600,
                batch_size: 100,
                demotion_idle_threshold: std::time::Duration::from_secs(600),
                consolidation_ratio: 0.5,
            },
            EvolutionConfig {
                interval_secs: 7200,
                fitness_threshold: 3,
                ..Default::default()
            },
            GenomeUpdateConfig {
                interval_secs: 43200,
                max_genomes: 14,
                auto_cleanup: true,
            },
            &engine,
        );

        // Verify configs were applied
        assert_eq!(config.consolidation().interval().as_secs(), 3600);
        assert_eq!(config.distillation().fitness_threshold(), 3);
        assert_eq!(config.genome_update().max_genomes(), 14);
    }

    // LCA Tests
    mod lca_tests {
        use super::*;
        use koru_lambda_core::LocalCausalAgent;

        fn setup_agent() -> ProcessAgent {
            let field = SharedEngine::new();
            ProcessAgent::new(&field)
        }

        #[test]
        fn test_process_agent_implements_lca_trait() {
            let agent = setup_agent();

            // Verify trait is implemented
            let _root = agent.get_current_root();
        }

        #[test]
        fn test_process_agent_has_unique_local_root() {
            let field = SharedEngine::new();
            let agent1 = ProcessAgent::new(&field);
            let agent2 = ProcessAgent::new(&field);

            // Each agent should have the same process root from canonical roots
            assert_eq!(
                agent1.local_root().id(),
                agent2.local_root().id(),
                "Process agents share the same canonical root"
            );
        }

        #[test]
        fn test_spawn_process_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent
                .spawn_process_synthesized(ProcessType::Consolidation, ProcessConfig::default());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_pause_process_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.pause_process_synthesized("process-1".to_string());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after pause synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_resume_process_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.resume_process_synthesized("process-1".to_string());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after resume synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_terminate_process_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.terminate_process_synthesized("process-1".to_string());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after terminate synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_heartbeat_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.heartbeat_synthesized("process-1".to_string());

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after heartbeat synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_list_processes_synthesizes() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let new_root = agent.list_processes_synthesized();

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after list processes synthesis"
            );
            assert_eq!(new_root.id(), root_after);
        }

        #[test]
        fn test_apply_action_changes_root() {
            let mut agent = setup_agent();
            let root_before = agent.local_root().id().to_string();

            let action = ProcessAction::ListProcesses;
            let new_root = agent.apply_action(action);

            let root_after = agent.local_root().id().to_string();
            assert_ne!(
                root_before, root_after,
                "Local root should change after apply_action"
            );
            assert_eq!(new_root.id(), root_after);
        }
    }
}
