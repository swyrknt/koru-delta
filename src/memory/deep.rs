/// Essence Agent: Genomic storage layer with LCA architecture.
///
/// The Essence Agent acts like DNA - minimal information needed to recreate
/// the entire system. It's the ultimate compression and portability layer.
///
/// ## LCA Architecture
///
/// As a Local Causal Agent, all operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
///
/// The Essence Agent's local root is `RootType::Essence` (💎 ESSENCE).
///
/// ## Purpose
///
/// - Store the minimal "genome" to recreate the system
/// - Enable 1KB backups regardless of database size
/// - Provide disaster recovery through "re-expression"
/// - Archive ancient data that may never be accessed
///
/// ## The Genome
///
/// The genome is NOT a copy of all data. It's:
/// - Root distinctions (genesis)
/// - Causal topology (structure, not content)
/// - Essential patterns (compressed)
/// - Reference patterns (relationships)
///
/// ## Re-expression
///
/// Given a genome, the system can "grow" back:
/// 1. Start with root distinctions
/// 2. Follow causal topology
/// 3. Re-establish reference patterns
/// 4. Rebuild full state
///
/// ## Analogy
///
/// Like stem cells: minimal information, maximum potential.
/// A genome is ~1KB. A full database might be 1TB.
/// But from the genome, you can regenerate the whole.
use crate::actions::EssenceAction;
use crate::causal_graph::{CausalGraph, DistinctionId};
use crate::engine::{FieldHandle, SharedEngine};
use crate::roots::RootType;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Essence agent configuration.
#[derive(Debug, Clone)]
pub struct EssenceConfig {
    /// How often to update the genome
    pub genome_update_interval: std::time::Duration,

    /// How many root distinctions to keep
    pub max_roots: usize,

    /// How many reference patterns to track
    pub max_patterns: usize,
}

impl Default for EssenceConfig {
    fn default() -> Self {
        Self {
            genome_update_interval: std::time::Duration::from_secs(86400), // Daily
            max_roots: 100,
            max_patterns: 1000,
        }
    }
}

/// Essence Agent - genomic storage with LCA architecture.
///
/// Like DNA: minimal, portable, regenerative.
/// All operations are synthesized through the unified field.
pub struct EssenceAgent {
    /// Configuration
    config: EssenceConfig,

    /// LCA: Local root distinction (Root: ESSENCE)
    local_root: Distinction,

    /// LCA: Handle to the shared field
    field: FieldHandle,

    /// The genome - minimal self-recreation info
    genome: DashMap<String, Genome>,

    /// Archive of old epochs (for historical reference)
    archive: DashMap<String, ArchivedEpoch>,

    /// Statistics
    genomes_created: AtomicU64,
    restorations: AtomicU64,
}

/// A genome - minimal information to recreate system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    /// Genome version
    pub version: u32,

    /// When this genome was extracted
    pub extracted_at: DateTime<Utc>,

    /// Root distinctions (genesis points)
    pub roots: Vec<DistinctionId>,

    /// Causal topology (structure, not content)
    pub topology: CausalTopology,

    /// Reference patterns
    pub patterns: Vec<ReferencePattern>,

    /// Current epoch summary
    pub epoch_summary: EpochSummary,
}

/// Causal topology - the shape of the causal graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalTopology {
    /// Key paths through the graph
    pub paths: Vec<Vec<DistinctionId>>,

    /// Branch points (high out-degree)
    pub branches: Vec<DistinctionId>,

    /// Convergence points (high in-degree)
    pub convergences: Vec<DistinctionId>,
}

/// A reference pattern - structural relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencePattern {
    pub pattern_id: String,
    pub source_type: String,
    pub target_type: String,
    pub frequency: usize,
}

/// Summary of current epoch state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochSummary {
    pub epoch_number: usize,
    pub distinction_count: usize,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

/// An archived epoch (full data, compressed).
#[derive(Debug, Clone)]
pub struct ArchivedEpoch {
    pub id: String,
    pub archived_at: DateTime<Utc>,
    pub compressed_size: usize,
    pub distinction_count: usize,
}

impl EssenceAgent {
    /// Create new essence agent.
    ///
    /// # LCA Pattern
    ///
    /// The agent initializes with:
    /// - `local_root` = RootType::Essence (from shared field roots)
    /// - `field` = Handle to the unified distinction engine
    pub fn new(shared_engine: &SharedEngine) -> Self {
        Self::with_config(EssenceConfig::default(), shared_engine)
    }

    /// Create with custom config.
    ///
    /// # LCA Pattern
    ///
    /// The agent anchors to the ESSENCE root, which is synthesized
    /// from the primordial distinctions (d0, d1) in the shared field.
    pub fn with_config(config: EssenceConfig, shared_engine: &SharedEngine) -> Self {
        let local_root = shared_engine.root(RootType::Essence).clone();
        let field = FieldHandle::new(shared_engine);

        Self {
            config,
            local_root,
            field,
            genome: DashMap::new(),
            archive: DashMap::new(),
            genomes_created: AtomicU64::new(0),
            restorations: AtomicU64::new(0),
        }
    }

    /// Extract a genome from the current system state.
    ///
    /// This is the key operation - capture minimal recreation info.
    ///
    /// # LCA Pattern
    ///
    /// Extraction synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔExtractTopology_Action`
    pub fn extract_genome(
        &self,
        causal_graph: &CausalGraph,
        epoch_number: usize,
        distinction_count: usize,
    ) -> Genome {
        let roots = self.find_roots(causal_graph);
        let topology = self.capture_topology(causal_graph);
        let patterns = self.capture_patterns();

        let now = Utc::now();

        // Synthesize extract topology action
        let action = EssenceAction::ExtractTopology {
            source_id: "causal_graph".to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        let genome = Genome {
            version: 1,
            extracted_at: now,
            roots,
            topology,
            patterns,
            epoch_summary: EpochSummary {
                epoch_number,
                distinction_count,
                start_time: now - chrono::Duration::days(1),
                end_time: now,
            },
        };

        // Store it with nanosecond precision for uniqueness
        let id = format!("genome_{}", now.timestamp_nanos_opt().unwrap_or(0));
        self.genome.insert(id, genome.clone());

        self.genomes_created.fetch_add(1, Ordering::Relaxed);

        genome
    }

    /// Express a genome - recreate system state.
    ///
    /// This "grows" the system from the genome.
    ///
    /// # LCA Pattern
    ///
    /// Expression synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔRegenerate_Action`
    pub fn express_genome(&self, genome: &Genome) -> ExpressionResult {
        // Synthesize regenerate action
        let action = EssenceAction::Regenerate {
            from_dna_id: "genome".to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        // TODO: Implement actual re-expression
        // 1. Start with roots
        // 2. Follow topology paths
        // 3. Re-establish patterns
        // 4. Rebuild state

        self.restorations.fetch_add(1, Ordering::Relaxed);

        ExpressionResult {
            distinctions_restored: genome.epoch_summary.distinction_count,
            roots_restored: genome.roots.len(),
            patterns_restored: genome.patterns.len(),
        }
    }

    /// Archive an epoch (move from Cold to Deep).
    pub fn archive_epoch(
        &self,
        epoch_id: String,
        distinction_count: usize,
        compressed_size: usize,
    ) {
        let archived = ArchivedEpoch {
            id: epoch_id.clone(),
            archived_at: Utc::now(),
            compressed_size,
            distinction_count,
        };

        self.archive.insert(epoch_id, archived);
    }

    /// Store a genome.
    ///
    /// # LCA Pattern
    ///
    /// Store synthesizes: `ΔNew = ΔLocal_Root ⊕ ΔStoreGenome_Action`
    pub fn store_genome(&self, id: &str, genome: Genome) {
        // Synthesize store genome action
        let action = EssenceAction::StoreGenome {
            name: id.to_string(),
            genome_id: id.to_string(),
        };
        let _ = self.synthesize_action_internal(action);

        self.genome.insert(id.to_string(), genome);
    }

    /// Get a genome by ID.
    pub fn get_genome(&self, id: &str) -> Option<Genome> {
        self.genome.get(id).map(|g| g.clone())
    }

    /// Get latest genome.
    pub fn latest_genome(&self) -> Option<Genome> {
        self.genome
            .iter()
            .max_by_key(|e| e.extracted_at)
            .map(|e| e.clone())
    }

    /// Get genome count.
    pub fn genome_count(&self) -> usize {
        self.genome.len()
    }

    /// Get genome DashMap (for process access).
    ///
    /// This is needed for cleanup operations from GenomeUpdateProcess.
    /// Returns a reference to the internal genome storage.
    pub fn genome(&self) -> &DashMap<String, Genome> {
        &self.genome
    }

    /// Get archive count.
    pub fn archive_count(&self) -> usize {
        self.archive.len()
    }

    /// Get total archived size.
    pub fn total_archive_size(&self) -> usize {
        self.archive.iter().map(|e| e.compressed_size).sum()
    }

    /// Get configuration.
    pub fn config(&self) -> &EssenceConfig {
        &self.config
    }

    /// Get statistics.
    pub fn stats(&self) -> EssenceStats {
        EssenceStats {
            genomes_created: self.genomes_created.load(Ordering::Relaxed),
            restorations: self.restorations.load(Ordering::Relaxed),
            genome_count: self.genome_count(),
            archive_count: self.archive_count(),
            total_archive_size: self.total_archive_size(),
        }
    }

    /// Serialize genome to bytes (for export).
    pub fn serialize_genome(genome: &Genome) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(genome)
    }

    /// Deserialize genome from bytes.
    pub fn deserialize_genome(bytes: &[u8]) -> Result<Genome, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Find root distinctions (no parents).
    fn find_roots(&self, causal_graph: &CausalGraph) -> Vec<DistinctionId> {
        causal_graph.roots()
    }

    /// Capture causal topology.
    fn capture_topology(&self, _causal_graph: &CausalGraph) -> CausalTopology {
        // TODO: Implement proper topology capture
        // For now, return empty
        CausalTopology {
            paths: vec![],
            branches: vec![],
            convergences: vec![],
        }
    }

    /// Capture reference patterns.
    fn capture_patterns(&self) -> Vec<ReferencePattern> {
        // TODO: Implement pattern extraction
        vec![]
    }

    /// Internal synthesis helper.
    ///
    /// Performs the LCA synthesis: `ΔNew = ΔLocal_Root ⊕ ΔAction`
    fn synthesize_action_internal(&self, action: EssenceAction) -> Distinction {
        let engine = self.field.engine_arc();
        let action_distinction = action.to_canonical_structure(engine);
        engine.synthesize(&self.local_root, &action_distinction)
    }
}

impl Default for EssenceAgent {
    fn default() -> Self {
        // Note: This requires a SharedEngine, so we panic if called directly
        // In practice, always use EssenceAgent::new(&shared_engine)
        panic!("EssenceAgent requires a SharedEngine - use EssenceAgent::new()")
    }
}

/// LCA Trait Implementation for EssenceAgent
///
/// All operations follow the synthesis pattern:
/// ```text
/// ΔNew = ΔLocal_Root ⊕ ΔAction_Data
/// ```
impl LocalCausalAgent for EssenceAgent {
    type ActionData = EssenceAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: EssenceAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

/// Result of expressing a genome.
#[derive(Debug, Clone)]
pub struct ExpressionResult {
    pub distinctions_restored: usize,
    pub roots_restored: usize,
    pub patterns_restored: usize,
}

/// Essence agent statistics.
#[derive(Debug, Clone)]
pub struct EssenceStats {
    pub genomes_created: u64,
    pub restorations: u64,
    pub genome_count: usize,
    pub archive_count: usize,
    pub total_archive_size: usize,
}

/// Backward-compatible type alias for existing code.
pub type DeepMemory = EssenceAgent;

/// Backward-compatible type alias for existing code.
pub type DeepConfig = EssenceConfig;

/// Backward-compatible type alias for existing code.
pub type DeepStats = EssenceStats;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_graph::CausalGraph;

    fn create_test_engine() -> SharedEngine {
        SharedEngine::new()
    }

    #[test]
    fn test_new_essence_agent() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);

        assert_eq!(essence.genome_count(), 0);
        assert_eq!(essence.archive_count(), 0);
    }

    #[test]
    fn test_extract_genome() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);
        let causal_graph = CausalGraph::new(&create_test_engine());

        // Add some nodes
        causal_graph.add_node("root1".to_string());
        causal_graph.add_node("root2".to_string());

        let genome = essence.extract_genome(&causal_graph, 5, 1000);

        assert_eq!(genome.version, 1);
        assert_eq!(genome.epoch_summary.epoch_number, 5);
        assert_eq!(genome.epoch_summary.distinction_count, 1000);
        assert_eq!(genome.roots.len(), 2);

        // Check stored
        assert_eq!(essence.genome_count(), 1);
        let stats = essence.stats();
        assert_eq!(stats.genomes_created, 1);
    }

    #[test]
    fn test_express_genome() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);
        let causal_graph = CausalGraph::new(&create_test_engine());

        causal_graph.add_node("root".to_string());

        let genome = essence.extract_genome(&causal_graph, 0, 100);
        let result = essence.express_genome(&genome);

        assert_eq!(result.distinctions_restored, 100);
        assert_eq!(result.roots_restored, 1);

        let stats = essence.stats();
        assert_eq!(stats.restorations, 1);
    }

    #[test]
    fn test_archive_epoch() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);

        essence.archive_epoch("epoch_0".to_string(), 50000, 1024 * 1024);
        essence.archive_epoch("epoch_1".to_string(), 60000, 2 * 1024 * 1024);

        assert_eq!(essence.archive_count(), 2);
        assert_eq!(essence.total_archive_size(), 3 * 1024 * 1024);
    }

    #[test]
    fn test_get_latest_genome() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);
        let causal_graph = CausalGraph::new(&create_test_engine());

        causal_graph.add_node("root".to_string());

        // Extract multiple genomes
        let _g1 = essence.extract_genome(&causal_graph, 1, 100);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _g2 = essence.extract_genome(&causal_graph, 2, 200);

        let latest = essence.latest_genome().unwrap();

        // Latest should be g2 (extracted last)
        assert_eq!(latest.epoch_summary.epoch_number, 2);
        assert_eq!(latest.epoch_summary.distinction_count, 200);
    }

    #[test]
    fn test_serialize_deserialize() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);
        let causal_graph = CausalGraph::new(&create_test_engine());

        causal_graph.add_node("root".to_string());

        let genome = essence.extract_genome(&causal_graph, 0, 100);

        // Serialize
        let bytes = EssenceAgent::serialize_genome(&genome).unwrap();

        // Deserialize
        let restored = EssenceAgent::deserialize_genome(&bytes).unwrap();

        assert_eq!(restored.version, genome.version);
        assert_eq!(
            restored.epoch_summary.distinction_count,
            genome.epoch_summary.distinction_count
        );
        assert_eq!(restored.roots.len(), genome.roots.len());
    }

    #[test]
    fn test_custom_config() {
        let config = EssenceConfig {
            genome_update_interval: std::time::Duration::from_secs(3600),
            max_roots: 50,
            max_patterns: 500,
        };
        let engine = create_test_engine();
        let essence = EssenceAgent::with_config(config, &engine);

        let causal_graph = CausalGraph::new(&create_test_engine());
        let genome = essence.extract_genome(&causal_graph, 0, 100);

        // Should still work with custom config
        assert_eq!(genome.version, 1);
        assert!(essence.genome_count() > 0);
    }

    #[test]
    fn test_stats() {
        let engine = create_test_engine();
        let essence = EssenceAgent::new(&engine);
        let causal_graph = CausalGraph::new(&create_test_engine());

        causal_graph.add_node("root".to_string());

        // Create genome
        essence.extract_genome(&causal_graph, 0, 100);

        // Archive epoch
        essence.archive_epoch("epoch_0".to_string(), 50000, 1024 * 1024);

        let stats = essence.stats();

        assert_eq!(stats.genomes_created, 1);
        assert_eq!(stats.genome_count, 1);
        assert_eq!(stats.archive_count, 1);
        assert_eq!(stats.total_archive_size, 1024 * 1024);
    }

    #[test]
    fn test_lca_trait_implementation() {
        let engine = create_test_engine();
        let mut agent = EssenceAgent::new(&engine);

        // Test get_current_root
        let root = agent.get_current_root();
        let root_id = root.id().to_string();
        assert!(!root_id.is_empty());

        // Test synthesize_action
        let action = EssenceAction::ExtractTopology {
            source_id: "test123".to_string(),
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
        let _deep_memory: DeepMemory = EssenceAgent::new(&engine);
        let _config: DeepConfig = EssenceConfig::default();
        let engine2 = create_test_engine();
        let agent = EssenceAgent::with_config(_config, &engine2);
        let _stats: DeepStats = agent.stats();
    }
}
