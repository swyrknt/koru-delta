/// Genome Update Process: DNA maintenance.
///
/// This process periodically extracts a genome from the current system state,
/// enabling disaster recovery through re-expression. Like maintaining
/// stem cells for regeneration.
///
/// ## Purpose
///
/// - Periodically extract genome (minimal recreation info)
/// - Store genome in Deep memory
/// - Enable 1KB portable backups
/// - Provide recovery point-in-time snapshots
///
/// ## Frequency
///
/// By default, daily. Configurable based on:
/// - Mutation rate (how fast system changes)
/// - Recovery requirements (how much data loss is acceptable)
/// - Storage constraints (how many genomes to keep)
use crate::causal_graph::CausalGraph;
use crate::memory::{DeepMemory, Genome};
use chrono::Utc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Genome update configuration.
#[derive(Debug, Clone)]
pub struct GenomeUpdateConfig {
    /// How often to update genome (seconds)
    pub interval_secs: u64,
    
    /// Maximum genomes to keep
    pub max_genomes: usize,
    
    /// Whether to auto-cleanup old genomes
    pub auto_cleanup: bool,
}

impl Default for GenomeUpdateConfig {
    fn default() -> Self {
        Self {
            interval_secs: 86400, // Daily
            max_genomes: 7,       // Keep 7 genomes (week)
            auto_cleanup: true,
        }
    }
}

/// Genome Update Process - maintains system DNA.
pub struct GenomeUpdateProcess {
    config: GenomeUpdateConfig,
    updates_performed: AtomicU64,
    cleanups_performed: AtomicU64,
}

impl GenomeUpdateProcess {
    /// Create new genome update process.
    pub fn new() -> Self {
        Self::with_config(GenomeUpdateConfig::default())
    }
    
    /// Create with custom config.
    pub fn with_config(config: GenomeUpdateConfig) -> Self {
        Self {
            config,
            updates_performed: AtomicU64::new(0),
            cleanups_performed: AtomicU64::new(0),
        }
    }
    
    /// Perform genome update.
    ///
    /// Extracts current genome and stores in Deep memory.
    pub fn update(
        &self,
        deep: &DeepMemory,
        causal_graph: &CausalGraph,
        epoch_number: usize,
        distinction_count: usize,
    ) -> Option<Genome> {
        let genome = deep.extract_genome(causal_graph, epoch_number, distinction_count);
        
        self.updates_performed.fetch_add(1, Ordering::Relaxed);
        
        // Cleanup old genomes if needed
        if self.config.auto_cleanup {
            self.cleanup_old_genomes(deep);
        }
        
        Some(genome)
    }
    
    /// Restore from latest genome.
    ///
    /// Expresses the most recent genome to restore system state.
    pub fn restore_latest(&self, deep: &DeepMemory) -> Option<crate::memory::ExpressionResult> {
        let genome = deep.latest_genome()?;
        Some(deep.express_genome(&genome))
    }
    
    /// Restore from specific genome.
    pub fn restore(&self, deep: &DeepMemory, genome_id: &str) -> Option<crate::memory::ExpressionResult> {
        let genome = deep.get_genome(genome_id)?;
        Some(deep.express_genome(&genome))
    }
    
    /// Export genome to bytes.
    pub fn export_genome(deep: &DeepMemory, genome_id: &str) -> Option<Vec<u8>> {
        let genome = deep.get_genome(genome_id)?;
        DeepMemory::serialize_genome(&genome).ok()
    }
    
    /// Import genome from bytes.
    pub fn import_genome(deep: &DeepMemory, bytes: &[u8]) -> Option<Genome> {
        let genome = DeepMemory::deserialize_genome(bytes).ok()?;
        let id = format!("imported_{}", Utc::now().timestamp());
        deep.genome().insert(id, genome.clone());
        Some(genome)
    }
    
    /// Cleanup old genomes beyond max_genomes limit.
    fn cleanup_old_genomes(&self, deep: &DeepMemory) {
        let count = deep.genome_count();
        
        if count > self.config.max_genomes {
            let to_remove = count - self.config.max_genomes;
            
            // Get oldest genomes and remove them
            let mut genomes: Vec<_> = deep
                .genome()
                .iter()
                .map(|e| (e.key().clone(), e.extracted_at))
                .collect();
            
            genomes.sort_by_key(|(_, ts)| *ts);
            
            for (id, _) in genomes.into_iter().take(to_remove) {
                deep.genome().remove(&id);
                self.cleanups_performed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    /// Get interval.
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.interval_secs)
    }
    
    /// Get max genomes.
    pub fn max_genomes(&self) -> usize {
        self.config.max_genomes
    }
    
    /// Get statistics.
    pub fn stats(&self) -> GenomeUpdateStats {
        GenomeUpdateStats {
            updates_performed: self.updates_performed.load(Ordering::Relaxed),
            cleanups_performed: self.cleanups_performed.load(Ordering::Relaxed),
        }
    }
    
    /// Extract a minimal genome from the system.
    ///
    /// Creates a "DNA" snapshot of the causal topology.
    pub fn extract_genome(&self) -> Genome {
        use crate::memory::{CausalTopology, EpochSummary, ReferencePattern};
        
        // Create a minimal genome representation
        // In full implementation, would extract from causal graph
        Genome {
            version: 1,
            extracted_at: Utc::now(),
            roots: vec![],
            topology: CausalTopology {
                paths: vec![],
                branches: vec![],
                convergences: vec![],
            },
            patterns: vec![],
            epoch_summary: EpochSummary {
                epoch_number: 0,
                distinction_count: 0,
                start_time: Utc::now(),
                end_time: Utc::now(),
            },
        }
    }
}

impl Default for GenomeUpdateProcess {
    fn default() -> Self {
        Self::new()
    }
}

/// Genome update statistics.
#[derive(Debug, Clone)]
pub struct GenomeUpdateStats {
    pub updates_performed: u64,
    pub cleanups_performed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_graph::CausalGraph;
    use crate::memory::DeepMemory;

    #[test]
    fn test_update() {
        let process = GenomeUpdateProcess::new();
        let deep = DeepMemory::new();
        let causal_graph = CausalGraph::new();
        
        causal_graph.add_node("root".to_string());
        
        let genome = process.update(&deep, &causal_graph, 0, 100);
        
        assert!(genome.is_some());
        assert_eq!(deep.genome_count(), 1);
        assert_eq!(process.stats().updates_performed, 1);
    }

    #[test]
    fn test_restore_latest() {
        let process = GenomeUpdateProcess::new();
        let deep = DeepMemory::new();
        let causal_graph = CausalGraph::new();
        
        causal_graph.add_node("root".to_string());
        
        process.update(&deep, &causal_graph, 0, 100);
        
        let result = process.restore_latest(&deep);
        assert!(result.is_some());
    }

    #[test]
    fn test_cleanup() {
        let config = GenomeUpdateConfig {
            interval_secs: 86400,
            max_genomes: 3,
            auto_cleanup: true,
        };
        let process = GenomeUpdateProcess::with_config(config);
        let deep = DeepMemory::new();
        let causal_graph = CausalGraph::new();
        
        causal_graph.add_node("root".to_string());
        
        // Create 5 genomes
        for i in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            process.update(&deep, &causal_graph, i, 100);
        }
        
        // Should only have 3 (max_genomes)
        assert_eq!(deep.genome_count(), 3);
        assert!(process.stats().cleanups_performed > 0);
    }

    #[test]
    fn test_config() {
        let config = GenomeUpdateConfig {
            interval_secs: 3600,
            max_genomes: 10,
            auto_cleanup: false,
        };
        let process = GenomeUpdateProcess::with_config(config);
        
        assert_eq!(process.interval().as_secs(), 3600);
        assert_eq!(process.max_genomes(), 10);
    }

    #[test]
    fn test_stats() {
        let process = GenomeUpdateProcess::new();
        let deep = DeepMemory::new();
        let causal_graph = CausalGraph::new();
        
        causal_graph.add_node("root".to_string());
        
        for _ in 0..3 {
            process.update(&deep, &causal_graph, 0, 100);
        }
        
        let stats = process.stats();
        assert_eq!(stats.updates_performed, 3);
    }
}
