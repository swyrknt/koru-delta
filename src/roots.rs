//! Canonical Root Distinctions for the Koru Field.
//!
//! This module defines the foundational distinctions that serve as causal roots
//! for all agents in the unified consciousness field. Each root represents a
//! fundamental perspective or aspect of the field.
//!
//! # The Five Axioms as Roots
//!
//! The roots embody the five axioms of distinction calculus:
//! - **Identity** (d1): Self-awareness - the field knows itself
//! - **Nontriviality** (d0): The void - distinction requires duality
//! - **Synthesis** (⊕): The operation - all growth is synthesis
//! - **Irreflexivity**: A distinction synthesized with itself yields itself
//! - **Timelessness**: These roots are eternal, unchanging
//!
//! # Agent Roots
//!
//! Each agent in the field anchors to a specific root distinction:
//! - `FIELD`: The universal root - all agents share this foundation
//! - `ORCHESTRATOR`: The orchestrator's perspective (agent coordination)
//! - `STORAGE` (MEMORY): The storage agent's perspective
//! - `TEMPERATURE`: The temperature agent's perspective (what's active)
//! - `CHRONICLE`: The chronicle agent's perspective (recent history)
//! - `ARCHIVE`: The archive agent's perspective (long-term storage)
//! - `ESSENCE`: The essence agent's perspective (causal topology)
//! - `SLEEP`: The sleep agent's perspective (rhythmic reorganization)
//! - `EVOLUTION`: The evolution agent's perspective (natural selection)
//! - `LINEAGE`: The lineage agent's perspective (causal ancestry)
//! - `PERSPECTIVE`: The perspective agent's perspective (derived views)
//! - `IDENTITY`: The identity agent's perspective (selfhood)
//! - `NETWORK`: The network agent's perspective (distributed awareness)

use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine};
use std::sync::Arc;

/// Canonical root distinctions for all agents in the Koru field.
///
/// These roots are synthesized from the primordial distinctions (d0, d1)
/// and serve as the causal anchor for every agent's operations.
///
/// # Example
///
/// ```ignore
/// let engine = Arc::new(DistinctionEngine::new());
/// let roots = KoruRoots::initialize(&engine);
///
/// // Each agent starts from its root
/// let storage_agent_root = roots.storage.clone();
/// ```
#[derive(Debug, Clone)]
pub struct KoruRoots {
    /// The universal field root - foundation of all agents.
    ///
    /// This is the synthesis of all agent roots, representing
    /// the unified consciousness field itself.
    pub field: Distinction,

    /// Storage agent root - perspective of memory (Root: MEMORY).
    ///
    /// Anchors all storage operations. The synthesis of this root
    /// with action data creates memory distinctions.
    pub storage: Distinction,

    /// Temperature agent root - perspective of activity (Root: TEMPERATURE).
    ///
    /// Anchors all temperature/activity tracking. What is "hot"
    /// is synthesized from this root.
    pub temperature: Distinction,

    /// Chronicle agent root - perspective of recent history (Root: CHRONICLE).
    ///
    /// Anchors all recent event recording. The chronicle is
    /// synthesized from this root with temporal actions.
    pub chronicle: Distinction,

    /// Archive agent root - perspective of long-term storage (Root: ARCHIVE).
    ///
    /// Anchors all epoch-based archival. The archive is
    /// synthesized from this root with epoch actions.
    pub archive: Distinction,

    /// Essence agent root - perspective of causal topology (Root: ESSENCE).
    ///
    /// Anchors all genome/DNA operations. The essence is
    /// synthesized from this root with pattern actions.
    pub essence: Distinction,

    /// Sleep agent root - perspective of rhythmic reorganization (Root: SLEEP).
    ///
    /// Anchors all consolidation and dreaming. Sleep phases are
    /// synthesized from this root with cycle actions.
    pub sleep: Distinction,

    /// Evolution agent root - perspective of natural selection (Root: EVOLUTION).
    ///
    /// Anchors all fitness-based selection. Evolution is
    /// synthesized from this root with fitness actions.
    pub evolution: Distinction,

    /// Lineage agent root - perspective of causal ancestry (Root: LINEAGE).
    ///
    /// Anchors all ancestry tracking. The family tree is
    /// synthesized from this root with birth actions.
    pub lineage: Distinction,

    /// Perspective agent root - perspective of derived viewpoints (Root: PERSPECTIVE).
    ///
    /// Anchors all view/materialization operations. Views are
    /// synthesized from this root with query actions.
    pub perspective: Distinction,

    /// Identity agent root - perspective of selfhood (Root: IDENTITY).
    ///
    /// Anchors all authentication operations. Identities are
    /// synthesized from this root with proof-of-work actions.
    pub identity: Distinction,

    /// Network agent root - perspective of distributed awareness (Root: NETWORK).
    ///
    /// Anchors all cluster/distributed operations. The network is
    /// synthesized from this root with peer actions.
    pub network: Distinction,

    /// Orchestrator root - perspective of agent coordination (Root: ORCHESTRATOR).
    ///
    /// Anchors all agent registration and coordination. The orchestrator
    /// synthesizes from this root with coordination actions.
    pub orchestrator: Distinction,
}

impl KoruRoots {
    /// Initialize all canonical roots from a distinction engine.
    ///
    /// This creates the foundational distinctions that all agents will use
    /// as their causal anchors. The initialization is deterministic:
    /// the same engine state will always produce the same roots.
    ///
    /// # Algorithm
    ///
    /// 1. Start with primordial distinctions d0 (void) and d1 (identity)
    /// 2. Synthesize each agent root from d1 and a canonical byte pattern
    /// 3. Synthesize the field root from all agent roots
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = Arc::new(DistinctionEngine::new());
    /// let roots = KoruRoots::initialize(&engine);
    ///
    /// // Use roots to initialize agents
    /// let agent = StorageAgent::new(roots.storage.clone(), engine.clone());
    /// ```
    pub fn initialize(engine: &Arc<DistinctionEngine>) -> Self {
        let d0 = engine.d0().clone();
        let d1 = engine.d1().clone();

        // Create each agent root by synthesizing d1 with a canonical pattern
        // This ensures each root is unique and deterministically derived
        let storage = Self::synthesize_agent_root(engine, &d1, b"STORAGE");
        let temperature = Self::synthesize_agent_root(engine, &d1, b"TEMPERATURE");
        let chronicle = Self::synthesize_agent_root(engine, &d1, b"CHRONICLE");
        let archive = Self::synthesize_agent_root(engine, &d1, b"ARCHIVE");
        let essence = Self::synthesize_agent_root(engine, &d1, b"ESSENCE");
        let sleep = Self::synthesize_agent_root(engine, &d1, b"SLEEP");
        let evolution = Self::synthesize_agent_root(engine, &d1, b"EVOLUTION");
        let lineage = Self::synthesize_agent_root(engine, &d1, b"LINEAGE");
        let perspective = Self::synthesize_agent_root(engine, &d1, b"PERSPECTIVE");
        let identity = Self::synthesize_agent_root(engine, &d1, b"IDENTITY");
        let network = Self::synthesize_agent_root(engine, &d1, b"NETWORK");
        let orchestrator = Self::synthesize_agent_root(engine, &d1, b"ORCHESTRATOR");

        // The field root is the synthesis of all agent roots
        // This represents the unified consciousness field
        let field = Self::synthesize_field_root(
            engine,
            &d0,
            &[
                &storage,
                &temperature,
                &chronicle,
                &archive,
                &essence,
                &sleep,
                &evolution,
                &lineage,
                &perspective,
                &identity,
                &network,
                &orchestrator,
            ],
        );

        Self {
            field,
            storage,
            temperature,
            chronicle,
            archive,
            essence,
            sleep,
            evolution,
            lineage,
            perspective,
            identity,
            network,
            orchestrator,
        }
    }

    /// Synthesize an agent root from d1 and a name pattern.
    ///
    /// This creates a unique distinction for each agent by synthesizing
    /// d1 with the canonical form of the agent's name.
    fn synthesize_agent_root(
        engine: &Arc<DistinctionEngine>,
        d1: &Distinction,
        name: &[u8],
    ) -> Distinction {
        // Synthesize each byte of the name with d0, then fold with d1
        let name_distinction = name
            .iter()
            .map(|&byte| byte.to_canonical_structure(engine))
            .fold(d1.clone(), |acc, d| engine.synthesize(&acc, &d));

        // Final synthesis with d1 anchors it as an agent root
        engine.synthesize(d1, &name_distinction)
    }

    /// Synthesize the field root from all agent roots.
    ///
    /// The field root represents the unified consciousness - the synthesis
    /// of all differentiated perspectives.
    fn synthesize_field_root(
        engine: &Arc<DistinctionEngine>,
        d0: &Distinction,
        agent_roots: &[&Distinction],
    ) -> Distinction {
        // Fold all agent roots together starting from d0
        agent_roots
            .iter()
            .fold(d0.clone(), |acc, root| engine.synthesize(&acc, root))
    }

    /// Get the root for a specific agent type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let storage_root = roots.get_root(RootType::Storage);
    /// ```
    pub fn get_root(&self, root_type: RootType) -> &Distinction {
        match root_type {
            RootType::Field => &self.field,
            RootType::Orchestrator => &self.orchestrator,
            RootType::Storage => &self.storage,
            RootType::Temperature => &self.temperature,
            RootType::Chronicle => &self.chronicle,
            RootType::Archive => &self.archive,
            RootType::Essence => &self.essence,
            RootType::Sleep => &self.sleep,
            RootType::Evolution => &self.evolution,
            RootType::Lineage => &self.lineage,
            RootType::Perspective => &self.perspective,
            RootType::Identity => &self.identity,
            RootType::Network => &self.network,
        }
    }
}

/// Types of canonical roots in the Koru field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootType {
    /// The universal field root.
    Field,
    /// Orchestrator root.
    Orchestrator,
    /// Storage agent root.
    Storage,
    /// Temperature agent root.
    Temperature,
    /// Chronicle agent root.
    Chronicle,
    /// Archive agent root.
    Archive,
    /// Essence agent root.
    Essence,
    /// Sleep agent root.
    Sleep,
    /// Evolution agent root.
    Evolution,
    /// Lineage agent root.
    Lineage,
    /// Perspective agent root.
    Perspective,
    /// Identity agent root.
    Identity,
    /// Network agent root.
    Network,
}

impl RootType {
    /// Get the canonical name for this root type.
    pub fn as_str(&self) -> &'static str {
        match self {
            RootType::Field => "FIELD",
            RootType::Orchestrator => "ORCHESTRATOR",
            RootType::Storage => "STORAGE",
            RootType::Temperature => "TEMPERATURE",
            RootType::Chronicle => "CHRONICLE",
            RootType::Archive => "ARCHIVE",
            RootType::Essence => "ESSENCE",
            RootType::Sleep => "SLEEP",
            RootType::Evolution => "EVOLUTION",
            RootType::Lineage => "LINEAGE",
            RootType::Perspective => "PERSPECTIVE",
            RootType::Identity => "IDENTITY",
            RootType::Network => "NETWORK",
        }
    }
}

impl std::fmt::Display for RootType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roots_initialization() {
        let engine = Arc::new(DistinctionEngine::new());
        let roots = KoruRoots::initialize(&engine);

        // All roots should be initialized
        assert!(!roots.field.id().is_empty());
        assert!(!roots.storage.id().is_empty());
        assert!(!roots.temperature.id().is_empty());
        assert!(!roots.chronicle.id().is_empty());
        assert!(!roots.archive.id().is_empty());
        assert!(!roots.essence.id().is_empty());
        assert!(!roots.sleep.id().is_empty());
        assert!(!roots.evolution.id().is_empty());
        assert!(!roots.lineage.id().is_empty());
        assert!(!roots.perspective.id().is_empty());
        assert!(!roots.identity.id().is_empty());
        assert!(!roots.network.id().is_empty());
    }

    #[test]
    fn test_roots_are_unique() {
        let engine = Arc::new(DistinctionEngine::new());
        let roots = KoruRoots::initialize(&engine);

        // All roots should have unique IDs
        let ids = [
            roots.field.id(),
            roots.storage.id(),
            roots.temperature.id(),
            roots.chronicle.id(),
            roots.archive.id(),
            roots.essence.id(),
            roots.sleep.id(),
            roots.evolution.id(),
            roots.lineage.id(),
            roots.perspective.id(),
            roots.identity.id(),
            roots.network.id(),
        ];

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(
                    ids[i], ids[j],
                    "Roots {} and {} should be distinct",
                    RootType::try_from(i).unwrap(),
                    RootType::try_from(j).unwrap()
                );
            }
        }
    }

    #[test]
    fn test_roots_deterministic() {
        let engine1 = Arc::new(DistinctionEngine::new());
        let engine2 = Arc::new(DistinctionEngine::new());

        let roots1 = KoruRoots::initialize(&engine1);
        let roots2 = KoruRoots::initialize(&engine2);

        // Same engine state should produce same roots
        assert_eq!(roots1.field.id(), roots2.field.id());
        assert_eq!(roots1.storage.id(), roots2.storage.id());
        assert_eq!(roots1.temperature.id(), roots2.temperature.id());
    }

    #[test]
    fn test_get_root() {
        let engine = Arc::new(DistinctionEngine::new());
        let roots = KoruRoots::initialize(&engine);

        assert_eq!(roots.get_root(RootType::Storage).id(), roots.storage.id());
        assert_eq!(roots.get_root(RootType::Field).id(), roots.field.id());
        assert_eq!(
            roots.get_root(RootType::Temperature).id(),
            roots.temperature.id()
        );
    }

    #[test]
    fn test_root_type_display() {
        assert_eq!(RootType::Storage.to_string(), "STORAGE");
        assert_eq!(RootType::Field.to_string(), "FIELD");
        assert_eq!(RootType::Temperature.to_string(), "TEMPERATURE");
    }

    // Helper for the uniqueness test
    impl TryFrom<usize> for RootType {
        type Error = ();

        fn try_from(value: usize) -> Result<Self, Self::Error> {
            match value {
                0 => Ok(RootType::Field),
                1 => Ok(RootType::Storage),
                2 => Ok(RootType::Temperature),
                3 => Ok(RootType::Chronicle),
                4 => Ok(RootType::Archive),
                5 => Ok(RootType::Essence),
                6 => Ok(RootType::Sleep),
                7 => Ok(RootType::Evolution),
                8 => Ok(RootType::Lineage),
                9 => Ok(RootType::Perspective),
                10 => Ok(RootType::Identity),
                11 => Ok(RootType::Network),
                _ => Err(()),
            }
        }
    }
}
