/// Evolutionary processes for automated memory management.
///
/// These processes run continuously to maintain the memory hierarchy:
/// - Consolidation: rhythmic movement between layers
/// - Distillation: fitness-based natural selection
/// - GenomeUpdate: DNA maintenance and disaster recovery
pub mod consolidation;
pub mod distillation;
pub mod genome_update;

use crate::engine::SharedEngine;

pub use consolidation::{ConsolidationConfig, ConsolidationProcess, ConsolidationResult};
pub use distillation::{DistillationConfig, DistillationProcess, DistillationResult, Fitness};
pub use genome_update::{GenomeUpdateConfig, GenomeUpdateProcess};

/// Process runner for all evolutionary processes.
///
/// Coordinates the three processes into a unified rhythm.
pub struct ProcessRunner {
    consolidation: ConsolidationProcess,
    distillation: DistillationProcess,
    genome_update: GenomeUpdateProcess,
}

impl ProcessRunner {
    /// Create new process runner with default configs.
    ///
    /// # LCA Pattern
    ///
    /// Requires a SharedEngine to initialize LCA agents.
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self {
            consolidation: ConsolidationProcess::new(shared_engine),
            distillation: DistillationProcess::new(shared_engine),
            genome_update: GenomeUpdateProcess::new(),
        }
    }

    /// Create with custom configurations.
    ///
    /// # LCA Pattern
    ///
    /// Requires a SharedEngine to initialize LCA agents.
    pub fn with_config(
        consolidation: ConsolidationConfig,
        distillation: DistillationConfig,
        genome: GenomeUpdateConfig,
        shared_engine: &SharedEngine,
    ) -> Self {
        Self {
            consolidation: ConsolidationProcess::with_config(consolidation, shared_engine),
            distillation: DistillationProcess::with_config(distillation, shared_engine),
            genome_update: GenomeUpdateProcess::with_config(genome),
        }
    }

    /// Get the consolidation process.
    pub fn consolidation(&self) -> &ConsolidationProcess {
        &self.consolidation
    }

    /// Get the distillation process.
    pub fn distillation(&self) -> &DistillationProcess {
        &self.distillation
    }

    /// Get the genome update process.
    pub fn genome_update(&self) -> &GenomeUpdateProcess {
        &self.genome_update
    }
}

impl Default for ProcessRunner {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use ProcessRunner::new(&shared_engine)
        panic!("ProcessRunner requires a SharedEngine - use ProcessRunner::new()")
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
        let runner = ProcessRunner::new(&engine);

        // Should have all three processes
        assert_eq!(runner.consolidation().cycle_count(), 0);
        assert_eq!(runner.distillation().stats().distinctions_evaluated, 0);
        assert_eq!(runner.genome_update().stats().updates_performed, 0);
    }

    #[test]
    fn test_process_runner_with_config() {
        let engine = create_test_engine();
        let config = ProcessRunner::with_config(
            ConsolidationConfig {
                interval_secs: 3600,
                batch_size: 100,
                demotion_idle_threshold: std::time::Duration::from_secs(600),
                consolidation_ratio: 0.5,
            },
            DistillationConfig {
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
}
