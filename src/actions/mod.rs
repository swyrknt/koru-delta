//! Action Types for the Koru LCA Architecture.
//!
//! This module defines all possible actions that agents can perform within the
//! unified consciousness field. Every action is canonicalizable, enabling
//! deterministic synthesis across all agents.
//!
//! # Action Philosophy
//!
//! In the LCA architecture, **everything is an action**. There are no operations,
//! no method calls, no direct mutations - only actions that are synthesized
//! with local roots to produce new distinctions.
//!
//! The formula is universal: **ΔNew = ΔLocal_Root ⊕ ΔAction_Data**
//!
//! # Action Hierarchy
//!
//! ```text
//! KoruAction
//! ├── StorageAction (memory operations)
//! ├── TemperatureAction (activity tracking)
//! ├── ChronicleAction (recent history)
//! ├── ArchiveAction (long-term storage)
//! ├── EssenceAction (causal topology)
//! ├── SleepAction (rhythmic reorganization)
//! ├── EvolutionAction (natural selection)
//! ├── LineageAction (causal ancestry)
//! ├── PerspectiveAction (derived views)
//! ├── IdentityAction (selfhood)
//! └── NetworkAction (distributed awareness)
//! ```

use chrono::{DateTime, Utc};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine};

/// The universal action type for all Koru agents.
///
/// This enum encompasses every possible action within the unified field.
/// Each variant wraps a specific agent's action type, enabling uniform
/// handling while preserving type safety.
///
/// # Example
///
/// ```ignore
/// let action = KoruAction::Storage(StorageAction::Store {
///     namespace: "users".to_string(),
///     key: "alice".to_string(),
///     value_json: json!({"name": "Alice"}),
/// });
///
/// let new_root = agent.synthesize_action(action, &engine);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum KoruAction {
    /// Storage operations - memory and retrieval.
    Storage(StorageAction),
    /// Temperature operations - activity tracking and LRU management.
    Temperature(TemperatureAction),
    /// Chronicle operations - recent history recording.
    Chronicle(ChronicleAction),
    /// Archive operations - long-term epoch-based storage.
    Archive(ArchiveAction),
    /// Essence operations - causal topology and genome.
    Essence(EssenceAction),
    /// Sleep operations - rhythmic consolidation.
    Sleep(SleepAction),
    /// Evolution operations - fitness-based selection.
    Evolution(EvolutionAction),
    /// Lineage operations - causal ancestry tracking.
    Lineage(LineageAction),
    /// Perspective operations - derived views and materialization.
    Perspective(PerspectiveAction),
    /// Identity operations - authentication and capabilities.
    Identity(IdentityAction),
    /// Network operations - distributed coordination.
    Network(NetworkAction),
    /// Pulse operations - orchestrator coordination.
    Pulse(PulseAction),
    /// Workspace operations - memory space management.
    Workspace(WorkspaceAction),
    /// Vector operations - embedding and similarity search.
    Vector(VectorAction),
    /// Lifecycle operations - memory tier transitions.
    Lifecycle(LifecycleAction),
    /// Session operations - authenticated session management.
    Session(SessionAction),
    /// Subscription operations - pub/sub change notifications.
    Subscription(SubscriptionAction),
    /// Process operations - background evolutionary processes.
    Process(ProcessAction),
    /// Reconciliation operations - distributed set synchronization.
    Reconciliation(ReconciliationAction),
}

impl From<PulseAction> for KoruAction {
    fn from(action: PulseAction) -> Self {
        KoruAction::Pulse(action)
    }
}

impl From<WorkspaceAction> for KoruAction {
    fn from(action: WorkspaceAction) -> Self {
        KoruAction::Workspace(action)
    }
}

impl From<VectorAction> for KoruAction {
    fn from(action: VectorAction) -> Self {
        KoruAction::Vector(action)
    }
}

impl KoruAction {
    /// Get the action category as a string.
    pub fn category(&self) -> &'static str {
        match self {
            KoruAction::Storage(_) => "STORAGE",
            KoruAction::Temperature(_) => "TEMPERATURE",
            KoruAction::Chronicle(_) => "CHRONICLE",
            KoruAction::Archive(_) => "ARCHIVE",
            KoruAction::Essence(_) => "ESSENCE",
            KoruAction::Sleep(_) => "SLEEP",
            KoruAction::Evolution(_) => "EVOLUTION",
            KoruAction::Lineage(_) => "LINEAGE",
            KoruAction::Perspective(_) => "PERSPECTIVE",
            KoruAction::Identity(_) => "IDENTITY",
            KoruAction::Network(_) => "NETWORK",
            KoruAction::Pulse(_) => "PULSE",
            KoruAction::Workspace(_) => "WORKSPACE",
            KoruAction::Vector(_) => "VECTOR",
            KoruAction::Lifecycle(_) => "LIFECYCLE",
            KoruAction::Session(_) => "SESSION",
            KoruAction::Subscription(_) => "SUBSCRIPTION",
            KoruAction::Process(_) => "PROCESS",
            KoruAction::Reconciliation(_) => "RECONCILIATION",
        }
    }

    /// Validate the action for correctness.
    ///
    /// Returns `Ok(())` if the action is valid, `Err(description)` otherwise.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            KoruAction::Storage(action) => action.validate(),
            KoruAction::Temperature(action) => action.validate(),
            KoruAction::Chronicle(action) => action.validate(),
            KoruAction::Archive(action) => action.validate(),
            KoruAction::Essence(action) => action.validate(),
            KoruAction::Sleep(action) => action.validate(),
            KoruAction::Evolution(action) => action.validate(),
            KoruAction::Lineage(action) => action.validate(),
            KoruAction::Perspective(action) => action.validate(),
            KoruAction::Identity(action) => action.validate(),
            KoruAction::Network(action) => action.validate(),
            KoruAction::Pulse(action) => action.validate(),
            KoruAction::Workspace(action) => action.validate(),
            KoruAction::Vector(action) => action.validate(),
            KoruAction::Lifecycle(action) => action.validate(),
            KoruAction::Session(action) => action.validate(),
            KoruAction::Subscription(action) => action.validate(),
            KoruAction::Process(action) => action.validate(),
            KoruAction::Reconciliation(action) => action.validate(),
        }
    }
}

impl Canonicalizable for KoruAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        // Delegate to the inner action type
        match self {
            KoruAction::Storage(action) => action.to_canonical_structure(engine),
            KoruAction::Temperature(action) => action.to_canonical_structure(engine),
            KoruAction::Chronicle(action) => action.to_canonical_structure(engine),
            KoruAction::Archive(action) => action.to_canonical_structure(engine),
            KoruAction::Essence(action) => action.to_canonical_structure(engine),
            KoruAction::Sleep(action) => action.to_canonical_structure(engine),
            KoruAction::Evolution(action) => action.to_canonical_structure(engine),
            KoruAction::Lineage(action) => action.to_canonical_structure(engine),
            KoruAction::Perspective(action) => action.to_canonical_structure(engine),
            KoruAction::Identity(action) => action.to_canonical_structure(engine),
            KoruAction::Network(action) => action.to_canonical_structure(engine),
            KoruAction::Pulse(action) => action.to_canonical_structure(engine),
            KoruAction::Workspace(action) => action.to_canonical_structure(engine),
            KoruAction::Vector(action) => action.to_canonical_structure(engine),
            KoruAction::Lifecycle(action) => action.to_canonical_structure(engine),
            KoruAction::Session(action) => action.to_canonical_structure(engine),
            KoruAction::Subscription(action) => action.to_canonical_structure(engine),
            KoruAction::Process(action) => action.to_canonical_structure(engine),
            KoruAction::Reconciliation(action) => action.to_canonical_structure(engine),
        }
    }
}

impl KoruAction {
    /// Serialize action to bytes for canonicalization.
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        // We use a simplified representation for serialization
        // Distinction IDs are stored as strings
        bincode::serialize(&ActionSerializable::from(self))
    }

    /// Convert bytes to distinction via synthesis.
    pub fn bytes_to_distinction(bytes: &[u8], engine: &DistinctionEngine) -> Distinction {
        bytes
            .iter()
            .map(|&byte| byte.to_canonical_structure(engine))
            .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
    }
}

/// Serializable representation of actions for canonicalization.
/// This is an internal type used for serialization - it mirrors KoruAction
/// but uses types that implement Serialize/Deserialize.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum ActionSerializable {
    Storage(StorageActionSerializable),
    Temperature(TemperatureActionSerializable),
    Chronicle(ChronicleActionSerializable),
    Archive(ArchiveActionSerializable),
    Essence(EssenceActionSerializable),
    Sleep(SleepActionSerializable),
    Evolution(EvolutionActionSerializable),
    Lineage(LineageActionSerializable),
    Perspective(PerspectiveActionSerializable),
    Identity(IdentityActionSerializable),
    Network(NetworkActionSerializable),
    Pulse(PulseActionSerializable),
    Workspace(WorkspaceActionSerializable),
    Vector(VectorActionSerializable),
    Lifecycle(LifecycleActionSerializable),
    Session(SessionActionSerializable),
    Subscription(SubscriptionActionSerializable),
    Process(ProcessActionSerializable),
    Reconciliation(ReconciliationActionSerializable),
}

impl From<&KoruAction> for ActionSerializable {
    fn from(action: &KoruAction) -> Self {
        match action {
            KoruAction::Storage(a) => ActionSerializable::Storage(a.into()),
            KoruAction::Temperature(a) => ActionSerializable::Temperature(a.into()),
            KoruAction::Chronicle(a) => ActionSerializable::Chronicle(a.into()),
            KoruAction::Archive(a) => ActionSerializable::Archive(a.into()),
            KoruAction::Essence(a) => ActionSerializable::Essence(a.into()),
            KoruAction::Sleep(a) => ActionSerializable::Sleep(a.into()),
            KoruAction::Evolution(a) => ActionSerializable::Evolution(a.into()),
            KoruAction::Lineage(a) => ActionSerializable::Lineage(a.into()),
            KoruAction::Perspective(a) => ActionSerializable::Perspective(a.into()),
            KoruAction::Identity(a) => ActionSerializable::Identity(a.into()),
            KoruAction::Network(a) => ActionSerializable::Network(a.into()),
            KoruAction::Pulse(a) => ActionSerializable::Pulse(a.into()),
            KoruAction::Workspace(a) => ActionSerializable::Workspace(a.into()),
            KoruAction::Vector(a) => ActionSerializable::Vector(a.into()),
            KoruAction::Lifecycle(a) => ActionSerializable::Lifecycle(a.into()),
            KoruAction::Session(a) => ActionSerializable::Session(a.into()),
            KoruAction::Subscription(a) => ActionSerializable::Subscription(a.into()),
            KoruAction::Process(a) => ActionSerializable::Process(a.into()),
            KoruAction::Reconciliation(a) => ActionSerializable::Reconciliation(a.into()),
        }
    }
}

/// Actions for the storage agent.
///
/// These actions represent all memory operations within the field.
/// Every store, retrieve, or query is an action that gets synthesized
/// with the storage agent's local root.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageAction {
    /// Store a value in the field.
    Store {
        /// Namespace (the "where").
        namespace: String,
        /// Key (the "what").
        key: String,
        /// Value JSON content.
        value_json: serde_json::Value,
    },
    /// Retrieve a value from the field.
    Retrieve {
        /// Namespace to search.
        namespace: String,
        /// Key to retrieve.
        key: String,
    },
    /// Get history for a key.
    History {
        /// Namespace of the key.
        namespace: String,
        /// Key to get history for.
        key: String,
    },
    /// Query with a pattern.
    Query {
        /// Query pattern as JSON.
        pattern_json: serde_json::Value,
    },
    /// Delete a key (tombstone).
    Delete {
        /// Namespace of the key.
        namespace: String,
        /// Key to delete.
        key: String,
    },
}

impl Canonicalizable for StorageAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        // Convert to serializable form, then to bytes, then synthesize
        let serializable = StorageActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

/// Convert bytes to distinction via synthesis.
fn bytes_to_distinction(bytes: &[u8], engine: &DistinctionEngine) -> Distinction {
    bytes
        .iter()
        .map(|&byte| byte.to_canonical_structure(engine))
        .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
}

/// Serializable version of StorageAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum StorageActionSerializable {
    Store { namespace: String, key: String, value_json: serde_json::Value },
    Retrieve { namespace: String, key: String },
    History { namespace: String, key: String },
    Query { pattern_json: serde_json::Value },
    Delete { namespace: String, key: String },
}

impl From<&StorageAction> for StorageActionSerializable {
    fn from(action: &StorageAction) -> Self {
        match action {
            StorageAction::Store { namespace, key, value_json } => {
                StorageActionSerializable::Store {
                    namespace: namespace.clone(),
                    key: key.clone(),
                    value_json: value_json.clone(),
                }
            }
            StorageAction::Retrieve { namespace, key } => {
                StorageActionSerializable::Retrieve {
                    namespace: namespace.clone(),
                    key: key.clone(),
                }
            }
            StorageAction::History { namespace, key } => {
                StorageActionSerializable::History {
                    namespace: namespace.clone(),
                    key: key.clone(),
                }
            }
            StorageAction::Query { pattern_json } => {
                StorageActionSerializable::Query {
                    pattern_json: pattern_json.clone(),
                }
            }
            StorageAction::Delete { namespace, key } => {
                StorageActionSerializable::Delete {
                    namespace: namespace.clone(),
                    key: key.clone(),
                }
            }
        }
    }
}

impl StorageAction {
    /// Validate the storage action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            StorageAction::Store { namespace, key, .. } => {
                if namespace.is_empty() {
                    return Err("StorageAction::Store: namespace is empty".to_string());
                }
                if key.is_empty() {
                    return Err("StorageAction::Store: key is empty".to_string());
                }
                Ok(())
            }
            StorageAction::Retrieve { namespace, key } => {
                if namespace.is_empty() {
                    return Err("StorageAction::Retrieve: namespace is empty".to_string());
                }
                if key.is_empty() {
                    return Err("StorageAction::Retrieve: key is empty".to_string());
                }
                Ok(())
            }
            StorageAction::History { namespace, key } => {
                if namespace.is_empty() {
                    return Err("StorageAction::History: namespace is empty".to_string());
                }
                if key.is_empty() {
                    return Err("StorageAction::History: key is empty".to_string());
                }
                Ok(())
            }
            StorageAction::Query { .. } => Ok(()),
            StorageAction::Delete { namespace, key } => {
                if namespace.is_empty() {
                    return Err("StorageAction::Delete: namespace is empty".to_string());
                }
                if key.is_empty() {
                    return Err("StorageAction::Delete: key is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the temperature agent.
///
/// Temperature represents "what's active" in the field. Hot distinctions
/// are those currently in focus, like the prefrontal cortex.
#[derive(Debug, Clone, PartialEq)]
pub enum TemperatureAction {
    /// Heat up a distinction (promote to hot).
    Heat {
        /// Distinction ID to heat.
        distinction_id: String,
        /// Current temperature level.
        level: TemperatureLevel,
    },
    /// Cool down a distinction (demote from hot).
    Cool {
        /// Distinction ID to cool.
        distinction_id: String,
    },
    /// Evict from hot to chronicle.
    Evict {
        /// Distinction ID to evict.
        distinction_id: String,
    },
    /// Record access (affects temperature).
    Access {
        /// Distinction ID that was accessed.
        distinction_id: String,
    },
}

/// Serializable version of TemperatureAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum TemperatureActionSerializable {
    Heat { distinction_id: String, level: TemperatureLevel },
    Cool { distinction_id: String },
    Evict { distinction_id: String },
    Access { distinction_id: String },
}

impl From<&TemperatureAction> for TemperatureActionSerializable {
    fn from(action: &TemperatureAction) -> Self {
        match action {
            TemperatureAction::Heat { distinction_id, level } => {
                TemperatureActionSerializable::Heat {
                    distinction_id: distinction_id.clone(),
                    level: *level,
                }
            }
            TemperatureAction::Cool { distinction_id } => {
                TemperatureActionSerializable::Cool {
                    distinction_id: distinction_id.clone(),
                }
            }
            TemperatureAction::Evict { distinction_id } => {
                TemperatureActionSerializable::Evict {
                    distinction_id: distinction_id.clone(),
                }
            }
            TemperatureAction::Access { distinction_id } => {
                TemperatureActionSerializable::Access {
                    distinction_id: distinction_id.clone(),
                }
            }
        }
    }
}

impl TemperatureAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            TemperatureAction::Heat { distinction_id, .. }
            | TemperatureAction::Cool { distinction_id }
            | TemperatureAction::Evict { distinction_id }
            | TemperatureAction::Access { distinction_id } => {
                if distinction_id.is_empty() {
                    return Err(format!("{:?}: distinction_id is empty", self));
                }
                Ok(())
            }
        }
    }
}

/// Temperature levels for distinctions.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TemperatureLevel {
    /// Cold - rarely accessed, archived.
    Cold,
    /// Cool - occasionally accessed, in chronicle.
    Cool,
    /// Warm - recently accessed, cooling down.
    Warm,
    /// Hot - currently active, in working memory.
    Hot,
}

/// Actions for the chronicle agent.
///
/// The chronicle maintains recent history, like the hippocampus.
/// It records what happened recently for temporal context.
#[derive(Debug, Clone, PartialEq)]
pub enum ChronicleAction {
    /// Record an event in the chronicle.
    Record {
        /// Event distinction ID to record.
        event_id: String,
        /// Timestamp of the event.
        timestamp: DateTime<Utc>,
    },
    /// Recall from the chronicle.
    Recall {
        /// Query pattern.
        query: String,
    },
    /// Promote from chronicle to hot.
    Promote {
        /// Distinction ID to promote.
        distinction_id: String,
    },
    /// Demote from chronicle to archive.
    Demote {
        /// Distinction ID to demote.
        distinction_id: String,
    },
}

/// Serializable version of ChronicleAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum ChronicleActionSerializable {
    Record { event_id: String, timestamp: DateTime<Utc> },
    Recall { query: String },
    Promote { distinction_id: String },
    Demote { distinction_id: String },
}

impl From<&ChronicleAction> for ChronicleActionSerializable {
    fn from(action: &ChronicleAction) -> Self {
        match action {
            ChronicleAction::Record { event_id, timestamp } => {
                ChronicleActionSerializable::Record {
                    event_id: event_id.clone(),
                    timestamp: *timestamp,
                }
            }
            ChronicleAction::Recall { query } => {
                ChronicleActionSerializable::Recall { query: query.clone() }
            }
            ChronicleAction::Promote { distinction_id } => {
                ChronicleActionSerializable::Promote {
                    distinction_id: distinction_id.clone(),
                }
            }
            ChronicleAction::Demote { distinction_id } => {
                ChronicleActionSerializable::Demote {
                    distinction_id: distinction_id.clone(),
                }
            }
        }
    }
}

impl ChronicleAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            ChronicleAction::Record { event_id, .. } => {
                if event_id.is_empty() {
                    return Err("ChronicleAction::Record: event_id is empty".to_string());
                }
                Ok(())
            }
            ChronicleAction::Recall { query } => {
                if query.is_empty() {
                    return Err("ChronicleAction::Recall: query is empty".to_string());
                }
                Ok(())
            }
            ChronicleAction::Promote { distinction_id } | ChronicleAction::Demote { distinction_id } => {
                if distinction_id.is_empty() {
                    return Err(format!("{:?}: distinction_id is empty", self));
                }
                Ok(())
            }
        }
    }
}

/// Actions for the archive agent.
///
/// The archive maintains long-term storage organized into epochs.
/// Like the cerebral cortex, it stores vast amounts of compressed history.
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveAction {
    /// Start a new epoch.
    EpochStart {
        /// Timestamp when the epoch starts.
        timestamp: DateTime<Utc>,
    },
    /// Seal an epoch as a distinction.
    EpochSeal {
        /// Epoch number.
        epoch_number: u64,
        /// Content distinction ID.
        content_id: String,
    },
    /// Compress an epoch.
    Compress {
        /// Epoch distinction ID to compress.
        epoch_id: String,
    },
    /// Retrieve from archive.
    Retrieve {
        /// Pattern to search for.
        pattern: String,
    },
    /// Archive distinctions to cold storage.
    Archive {
        /// Distinction IDs to archive.
        distinction_ids: Vec<String>,
    },
}

/// Serializable version of ArchiveAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum ArchiveActionSerializable {
    EpochStart { timestamp: DateTime<Utc> },
    EpochSeal { epoch_number: u64, content_id: String },
    Compress { epoch_id: String },
    Retrieve { pattern: String },
    Archive { distinction_ids: Vec<String> },
}

impl From<&ArchiveAction> for ArchiveActionSerializable {
    fn from(action: &ArchiveAction) -> Self {
        match action {
            ArchiveAction::EpochStart { timestamp } => {
                ArchiveActionSerializable::EpochStart { timestamp: *timestamp }
            }
            ArchiveAction::EpochSeal { epoch_number, content_id } => {
                ArchiveActionSerializable::EpochSeal {
                    epoch_number: *epoch_number,
                    content_id: content_id.clone(),
                }
            }
            ArchiveAction::Compress { epoch_id } => {
                ArchiveActionSerializable::Compress { epoch_id: epoch_id.clone() }
            }
            ArchiveAction::Retrieve { pattern } => {
                ArchiveActionSerializable::Retrieve { pattern: pattern.clone() }
            }
            ArchiveAction::Archive { distinction_ids } => {
                ArchiveActionSerializable::Archive {
                    distinction_ids: distinction_ids.clone(),
                }
            }
        }
    }
}

impl ArchiveAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            ArchiveAction::EpochStart { .. } => Ok(()),
            ArchiveAction::EpochSeal { content_id, .. } => {
                if content_id.is_empty() {
                    return Err("ArchiveAction::EpochSeal: content_id is empty".to_string());
                }
                Ok(())
            }
            ArchiveAction::Compress { epoch_id } => {
                if epoch_id.is_empty() {
                    return Err("ArchiveAction::Compress: epoch_id is empty".to_string());
                }
                Ok(())
            }
            ArchiveAction::Retrieve { pattern } => {
                if pattern.is_empty() {
                    return Err("ArchiveAction::Retrieve: pattern is empty".to_string());
                }
                Ok(())
            }
            ArchiveAction::Archive { distinction_ids } => {
                if distinction_ids.is_empty() {
                    return Err("ArchiveAction::Archive: distinction_ids is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the essence agent.
///
/// The essence agent extracts and maintains the causal topology of the field.
/// It creates the "DNA" or genome of the system.
#[derive(Debug, Clone, PartialEq)]
pub enum EssenceAction {
    /// Extract topology from a source.
    ExtractTopology {
        /// Source distinction ID to extract from.
        source_id: String,
    },
    /// Synthesize DNA from topology.
    SynthesizeDNA {
        /// Topology JSON structure.
        topology_json: serde_json::Value,
    },
    /// Regenerate from DNA.
    Regenerate {
        /// DNA distinction ID to regenerate from.
        from_dna_id: String,
    },
    /// Store a genome.
    StoreGenome {
        /// Name of the genome.
        name: String,
        /// Genome distinction ID.
        genome_id: String,
    },
}

/// Serializable version of EssenceAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum EssenceActionSerializable {
    ExtractTopology { source_id: String },
    SynthesizeDNA { topology_json: serde_json::Value },
    Regenerate { from_dna_id: String },
    StoreGenome { name: String, genome_id: String },
}

impl From<&EssenceAction> for EssenceActionSerializable {
    fn from(action: &EssenceAction) -> Self {
        match action {
            EssenceAction::ExtractTopology { source_id } => {
                EssenceActionSerializable::ExtractTopology { source_id: source_id.clone() }
            }
            EssenceAction::SynthesizeDNA { topology_json } => {
                EssenceActionSerializable::SynthesizeDNA {
                    topology_json: topology_json.clone(),
                }
            }
            EssenceAction::Regenerate { from_dna_id } => {
                EssenceActionSerializable::Regenerate { from_dna_id: from_dna_id.clone() }
            }
            EssenceAction::StoreGenome { name, genome_id } => {
                EssenceActionSerializable::StoreGenome {
                    name: name.clone(),
                    genome_id: genome_id.clone(),
                }
            }
        }
    }
}

impl EssenceAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            EssenceAction::ExtractTopology { source_id } => {
                if source_id.is_empty() {
                    return Err("EssenceAction::ExtractTopology: source_id is empty".to_string());
                }
                Ok(())
            }
            EssenceAction::SynthesizeDNA { .. } => Ok(()),
            EssenceAction::Regenerate { from_dna_id } => {
                if from_dna_id.is_empty() {
                    return Err("EssenceAction::Regenerate: from_dna_id is empty".to_string());
                }
                Ok(())
            }
            EssenceAction::StoreGenome { name, genome_id } => {
                if name.is_empty() {
                    return Err("EssenceAction::StoreGenome: name is empty".to_string());
                }
                if genome_id.is_empty() {
                    return Err("EssenceAction::StoreGenome: genome_id is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the sleep agent.
///
/// The sleep agent performs rhythmic reorganization of the field,
/// like sleep consolidating memories in biological systems.
#[derive(Debug, Clone, PartialEq)]
pub enum SleepAction {
    /// Enter a sleep phase.
    EnterPhase {
        /// Phase to enter.
        phase: SleepPhase,
    },
    /// Consolidate from one tier to another.
    Consolidate {
        /// Source tier name.
        from_tier: String,
        /// Destination tier name.
        to_tier: String,
    },
    /// Dream - random synthesis exploration.
    Dream,
    /// Wake from sleep.
    Wake,
}

/// Serializable version of SleepAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum SleepActionSerializable {
    EnterPhase { phase: SleepPhase },
    Consolidate { from_tier: String, to_tier: String },
    Dream,
    Wake,
}

impl From<&SleepAction> for SleepActionSerializable {
    fn from(action: &SleepAction) -> Self {
        match action {
            SleepAction::EnterPhase { phase } => {
                SleepActionSerializable::EnterPhase { phase: *phase }
            }
            SleepAction::Consolidate { from_tier, to_tier } => {
                SleepActionSerializable::Consolidate {
                    from_tier: from_tier.clone(),
                    to_tier: to_tier.clone(),
                }
            }
            SleepAction::Dream => SleepActionSerializable::Dream,
            SleepAction::Wake => SleepActionSerializable::Wake,
        }
    }
}

impl SleepAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            SleepAction::EnterPhase { .. } | SleepAction::Dream | SleepAction::Wake => Ok(()),
            SleepAction::Consolidate { from_tier, to_tier } => {
                if from_tier.is_empty() {
                    return Err("SleepAction::Consolidate: from_tier is empty".to_string());
                }
                if to_tier.is_empty() {
                    return Err("SleepAction::Consolidate: to_tier is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Sleep phases for rhythmic consolidation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SleepPhase {
    /// Awake - normal operation.
    Awake,
    /// Light sleep - hot to warm consolidation.
    LightSleep,
    /// Deep sleep - warm to cold consolidation.
    DeepSleep,
    /// REM - pattern extraction and dreaming.
    Rem,
}

/// Actions for the evolution agent.
///
/// The evolution agent performs natural selection on distinctions,
/// preserving fit distinctions and archiving unfit ones.
#[derive(Debug, Clone, PartialEq)]
pub enum EvolutionAction {
    /// Evaluate fitness of a distinction.
    EvaluateFitness {
        /// Candidate ID to evaluate.
        candidate_id: String,
    },
    /// Select fit distinctions from a population.
    Select {
        /// Population IDs to select from.
        population_ids: Vec<String>,
    },
    /// Preserve fit distinctions.
    Preserve {
        /// Fit distinction IDs to preserve.
        fit_ids: Vec<String>,
    },
    /// Archive unfit distinctions.
    Archive {
        /// Unfit distinction IDs to archive.
        unfit_ids: Vec<String>,
    },
}

/// Serializable version of EvolutionAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum EvolutionActionSerializable {
    EvaluateFitness { candidate_id: String },
    Select { population_ids: Vec<String> },
    Preserve { fit_ids: Vec<String> },
    Archive { unfit_ids: Vec<String> },
}

impl From<&EvolutionAction> for EvolutionActionSerializable {
    fn from(action: &EvolutionAction) -> Self {
        match action {
            EvolutionAction::EvaluateFitness { candidate_id } => {
                EvolutionActionSerializable::EvaluateFitness {
                    candidate_id: candidate_id.clone(),
                }
            }
            EvolutionAction::Select { population_ids } => {
                EvolutionActionSerializable::Select {
                    population_ids: population_ids.clone(),
                }
            }
            EvolutionAction::Preserve { fit_ids } => {
                EvolutionActionSerializable::Preserve { fit_ids: fit_ids.clone() }
            }
            EvolutionAction::Archive { unfit_ids } => {
                EvolutionActionSerializable::Archive { unfit_ids: unfit_ids.clone() }
            }
        }
    }
}

impl EvolutionAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            EvolutionAction::EvaluateFitness { candidate_id } => {
                if candidate_id.is_empty() {
                    return Err("EvolutionAction::EvaluateFitness: candidate_id is empty".to_string());
                }
                Ok(())
            }
            EvolutionAction::Select { population_ids } => {
                if population_ids.is_empty() {
                    return Err("EvolutionAction::Select: population_ids is empty".to_string());
                }
                Ok(())
            }
            EvolutionAction::Preserve { fit_ids } => {
                if fit_ids.is_empty() {
                    return Err("EvolutionAction::Preserve: fit_ids is empty".to_string());
                }
                Ok(())
            }
            EvolutionAction::Archive { unfit_ids } => {
                if unfit_ids.is_empty() {
                    return Err("EvolutionAction::Archive: unfit_ids is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the lineage agent.
///
/// The lineage agent tracks causal ancestry, maintaining the family
/// tree of how distinctions emerged from one another.
#[derive(Debug, Clone, PartialEq)]
pub enum LineageAction {
    /// Record the birth of a distinction.
    RecordBirth {
        /// Child distinction ID born.
        child_id: String,
        /// Parent distinction IDs.
        parent_ids: Vec<String>,
    },
    /// Trace ancestors of a distinction.
    TraceAncestors {
        /// Starting distinction ID.
        from_id: String,
    },
    /// Trace descendants of a distinction.
    TraceDescendants {
        /// Starting distinction ID.
        from_id: String,
    },
    /// Find common ancestor of two distinctions.
    FindCommonAncestor {
        /// First distinction ID.
        a_id: String,
        /// Second distinction ID.
        b_id: String,
    },
}

/// Serializable version of LineageAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum LineageActionSerializable {
    RecordBirth { child_id: String, parent_ids: Vec<String> },
    TraceAncestors { from_id: String },
    TraceDescendants { from_id: String },
    FindCommonAncestor { a_id: String, b_id: String },
}

impl From<&LineageAction> for LineageActionSerializable {
    fn from(action: &LineageAction) -> Self {
        match action {
            LineageAction::RecordBirth { child_id, parent_ids } => {
                LineageActionSerializable::RecordBirth {
                    child_id: child_id.clone(),
                    parent_ids: parent_ids.clone(),
                }
            }
            LineageAction::TraceAncestors { from_id } => {
                LineageActionSerializable::TraceAncestors { from_id: from_id.clone() }
            }
            LineageAction::TraceDescendants { from_id } => {
                LineageActionSerializable::TraceDescendants { from_id: from_id.clone() }
            }
            LineageAction::FindCommonAncestor { a_id, b_id } => {
                LineageActionSerializable::FindCommonAncestor {
                    a_id: a_id.clone(),
                    b_id: b_id.clone(),
                }
            }
        }
    }
}

impl LineageAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            LineageAction::RecordBirth { child_id, parent_ids } => {
                if child_id.is_empty() {
                    return Err("LineageAction::RecordBirth: child_id is empty".to_string());
                }
                if parent_ids.is_empty() {
                    return Err("LineageAction::RecordBirth: parent_ids is empty".to_string());
                }
                for (i, parent_id) in parent_ids.iter().enumerate() {
                    if parent_id.is_empty() {
                        return Err(format!(
                            "LineageAction::RecordBirth: parent_ids[{}] is empty",
                            i
                        ));
                    }
                }
                Ok(())
            }
            LineageAction::TraceAncestors { from_id }
            | LineageAction::TraceDescendants { from_id } => {
                if from_id.is_empty() {
                    return Err(format!("{:?}: from_id is empty", self));
                }
                Ok(())
            }
            LineageAction::FindCommonAncestor { a_id, b_id } => {
                if a_id.is_empty() {
                    return Err("LineageAction::FindCommonAncestor: a_id is empty".to_string());
                }
                if b_id.is_empty() {
                    return Err("LineageAction::FindCommonAncestor: b_id is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the perspective agent.
///
/// The perspective agent maintains derived views on the field,
/// like materialized views in a database but as synthesized distinctions.
#[derive(Debug, Clone, PartialEq)]
pub enum PerspectiveAction {
    /// Form a new view from a query.
    FormView {
        /// Query JSON.
        query_json: serde_json::Value,
        /// Name for the view.
        name: String,
    },
    /// Refresh a view.
    Refresh {
        /// View distinction ID to refresh.
        view_id: String,
    },
    /// Compose two views into one.
    Compose {
        /// First view distinction ID.
        view_a_id: String,
        /// Second view distinction ID.
        view_b_id: String,
    },
    /// Project from one view onto another.
    Project {
        /// Source view distinction ID.
        from_view_id: String,
        /// Target projection.
        onto_query: String,
    },
}

/// Serializable version of PerspectiveAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum PerspectiveActionSerializable {
    FormView { query_json: serde_json::Value, name: String },
    Refresh { view_id: String },
    Compose { view_a_id: String, view_b_id: String },
    Project { from_view_id: String, onto_query: String },
}

impl From<&PerspectiveAction> for PerspectiveActionSerializable {
    fn from(action: &PerspectiveAction) -> Self {
        match action {
            PerspectiveAction::FormView { query_json, name } => {
                PerspectiveActionSerializable::FormView {
                    query_json: query_json.clone(),
                    name: name.clone(),
                }
            }
            PerspectiveAction::Refresh { view_id } => {
                PerspectiveActionSerializable::Refresh { view_id: view_id.clone() }
            }
            PerspectiveAction::Compose { view_a_id, view_b_id } => {
                PerspectiveActionSerializable::Compose {
                    view_a_id: view_a_id.clone(),
                    view_b_id: view_b_id.clone(),
                }
            }
            PerspectiveAction::Project { from_view_id, onto_query } => {
                PerspectiveActionSerializable::Project {
                    from_view_id: from_view_id.clone(),
                    onto_query: onto_query.clone(),
                }
            }
        }
    }
}

impl PerspectiveAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            PerspectiveAction::FormView { name, .. } => {
                if name.is_empty() {
                    return Err("PerspectiveAction::FormView: name is empty".to_string());
                }
                Ok(())
            }
            PerspectiveAction::Refresh { view_id } => {
                if view_id.is_empty() {
                    return Err("PerspectiveAction::Refresh: view_id is empty".to_string());
                }
                Ok(())
            }
            PerspectiveAction::Compose { view_a_id, view_b_id } => {
                if view_a_id.is_empty() {
                    return Err("PerspectiveAction::Compose: view_a_id is empty".to_string());
                }
                if view_b_id.is_empty() {
                    return Err("PerspectiveAction::Compose: view_b_id is empty".to_string());
                }
                Ok(())
            }
            PerspectiveAction::Project { from_view_id, onto_query } => {
                if from_view_id.is_empty() {
                    return Err("PerspectiveAction::Project: from_view_id is empty".to_string());
                }
                if onto_query.is_empty() {
                    return Err("PerspectiveAction::Project: onto_query is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the identity agent.
///
/// The identity agent manages selfhood within the field - authentication,
/// capabilities, and proof-of-work identity.
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityAction {
    /// Mine a new identity.
    MineIdentity {
        /// Proof-of-work as JSON.
        proof_of_work_json: serde_json::Value,
    },
    /// Authenticate an identity.
    Authenticate {
        /// Identity ID to authenticate.
        identity_id: String,
        /// Challenge to verify.
        challenge: String,
    },
    /// Grant a capability.
    GrantCapability {
        /// Granter identity ID.
        from_id: String,
        /// Grantee identity ID.
        to_id: String,
        /// Permission being granted.
        permission: String,
    },
    /// Verify access to a resource.
    VerifyAccess {
        /// Identity ID requesting access.
        identity_id: String,
        /// Resource being accessed.
        resource: String,
    },
}

/// Serializable version of IdentityAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum IdentityActionSerializable {
    MineIdentity { proof_of_work_json: serde_json::Value },
    Authenticate { identity_id: String, challenge: String },
    GrantCapability { from_id: String, to_id: String, permission: String },
    VerifyAccess { identity_id: String, resource: String },
}

impl From<&IdentityAction> for IdentityActionSerializable {
    fn from(action: &IdentityAction) -> Self {
        match action {
            IdentityAction::MineIdentity { proof_of_work_json } => {
                IdentityActionSerializable::MineIdentity {
                    proof_of_work_json: proof_of_work_json.clone(),
                }
            }
            IdentityAction::Authenticate { identity_id, challenge } => {
                IdentityActionSerializable::Authenticate {
                    identity_id: identity_id.clone(),
                    challenge: challenge.clone(),
                }
            }
            IdentityAction::GrantCapability { from_id, to_id, permission } => {
                IdentityActionSerializable::GrantCapability {
                    from_id: from_id.clone(),
                    to_id: to_id.clone(),
                    permission: permission.clone(),
                }
            }
            IdentityAction::VerifyAccess { identity_id, resource } => {
                IdentityActionSerializable::VerifyAccess {
                    identity_id: identity_id.clone(),
                    resource: resource.clone(),
                }
            }
        }
    }
}

impl IdentityAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            IdentityAction::MineIdentity { .. } => Ok(()),
            IdentityAction::Authenticate { identity_id, challenge } => {
                if identity_id.is_empty() {
                    return Err("IdentityAction::Authenticate: identity_id is empty".to_string());
                }
                if challenge.is_empty() {
                    return Err("IdentityAction::Authenticate: challenge is empty".to_string());
                }
                Ok(())
            }
            IdentityAction::GrantCapability { from_id, to_id, permission } => {
                if from_id.is_empty() {
                    return Err("IdentityAction::GrantCapability: from_id is empty".to_string());
                }
                if to_id.is_empty() {
                    return Err("IdentityAction::GrantCapability: to_id is empty".to_string());
                }
                if permission.is_empty() {
                    return Err("IdentityAction::GrantCapability: permission is empty".to_string());
                }
                Ok(())
            }
            IdentityAction::VerifyAccess { identity_id, resource } => {
                if identity_id.is_empty() {
                    return Err("IdentityAction::VerifyAccess: identity_id is empty".to_string());
                }
                if resource.is_empty() {
                    return Err("IdentityAction::VerifyAccess: resource is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Actions for the network agent.
///
/// The network agent manages distributed awareness across multiple
/// nodes in the Koru field.
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkAction {
    /// Join the network with a peer.
    Join {
        /// Peer address to join.
        peer_address: String,
    },
    /// Synchronize with a peer.
    Synchronize {
        /// Peer ID to synchronize with.
        peer_id: String,
    },
    /// Reconcile differences with peers.
    Reconcile {
        /// Difference IDs to reconcile.
        difference_ids: Vec<String>,
    },
    /// Broadcast a message to all peers.
    Broadcast {
        /// Message JSON to broadcast.
        message_json: serde_json::Value,
    },
    /// Gossip state to peers.
    Gossip {
        /// State JSON to gossip.
        state_json: serde_json::Value,
    },
}

/// Serializable version of NetworkAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum NetworkActionSerializable {
    Join { peer_address: String },
    Synchronize { peer_id: String },
    Reconcile { difference_ids: Vec<String> },
    Broadcast { message_json: serde_json::Value },
    Gossip { state_json: serde_json::Value },
}

impl From<&NetworkAction> for NetworkActionSerializable {
    fn from(action: &NetworkAction) -> Self {
        match action {
            NetworkAction::Join { peer_address } => {
                NetworkActionSerializable::Join { peer_address: peer_address.clone() }
            }
            NetworkAction::Synchronize { peer_id } => {
                NetworkActionSerializable::Synchronize { peer_id: peer_id.clone() }
            }
            NetworkAction::Reconcile { difference_ids } => {
                NetworkActionSerializable::Reconcile {
                    difference_ids: difference_ids.clone(),
                }
            }
            NetworkAction::Broadcast { message_json } => {
                NetworkActionSerializable::Broadcast {
                    message_json: message_json.clone(),
                }
            }
            NetworkAction::Gossip { state_json } => {
                NetworkActionSerializable::Gossip { state_json: state_json.clone() }
            }
        }
    }
}

impl NetworkAction {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            NetworkAction::Join { peer_address } => {
                if peer_address.is_empty() {
                    return Err("NetworkAction::Join: peer_address is empty".to_string());
                }
                Ok(())
            }
            NetworkAction::Synchronize { peer_id } => {
                if peer_id.is_empty() {
                    return Err("NetworkAction::Synchronize: peer_id is empty".to_string());
                }
                Ok(())
            }
            NetworkAction::Reconcile { difference_ids } => {
                if difference_ids.is_empty() {
                    return Err("NetworkAction::Reconcile: difference_ids is empty".to_string());
                }
                Ok(())
            }
            NetworkAction::Broadcast { .. } | NetworkAction::Gossip { .. } => Ok(()),
        }
    }
}

/// Actions for the orchestrator.
///
/// These actions represent coordination operations for the agent orchestrator,
/// enabling agent registration, pulse coordination, and system-wide synthesis.
#[derive(Debug, Clone, PartialEq)]
pub enum PulseAction {
    /// Register a new agent.
    RegisterAgent {
        /// Agent ID.
        agent_id: String,
        /// Agent type.
        agent_type: String,
    },
    /// Unregister an agent.
    UnregisterAgent {
        /// Agent ID.
        agent_id: String,
    },
    /// Trigger a coordination pulse.
    TriggerPulse {
        /// Phase name.
        phase: String,
    },
}

/// Serializable version of PulseAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum PulseActionSerializable {
    RegisterAgent { agent_id: String, agent_type: String },
    UnregisterAgent { agent_id: String },
    TriggerPulse { phase: String },
}

impl From<&PulseAction> for PulseActionSerializable {
    fn from(action: &PulseAction) -> Self {
        match action {
            PulseAction::RegisterAgent { agent_id, agent_type } => {
                PulseActionSerializable::RegisterAgent {
                    agent_id: agent_id.clone(),
                    agent_type: agent_type.clone(),
                }
            }
            PulseAction::UnregisterAgent { agent_id } => {
                PulseActionSerializable::UnregisterAgent {
                    agent_id: agent_id.clone(),
                }
            }
            PulseAction::TriggerPulse { phase } => {
                PulseActionSerializable::TriggerPulse {
                    phase: phase.clone(),
                }
            }
        }
    }
}

impl PulseAction {
    /// Validate the pulse action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            PulseAction::RegisterAgent { agent_id, agent_type } => {
                if agent_id.is_empty() {
                    return Err("PulseAction::RegisterAgent: agent_id is empty".to_string());
                }
                if agent_type.is_empty() {
                    return Err("PulseAction::RegisterAgent: agent_type is empty".to_string());
                }
                Ok(())
            }
            PulseAction::UnregisterAgent { agent_id } => {
                if agent_id.is_empty() {
                    return Err("PulseAction::UnregisterAgent: agent_id is empty".to_string());
                }
                Ok(())
            }
            PulseAction::TriggerPulse { phase } => {
                if phase.is_empty() {
                    return Err("PulseAction::TriggerPulse: phase is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

impl Canonicalizable for PulseAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = PulseActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

/// Actions for the workspace agent.
///
/// The workspace agent manages isolated, versioned memory spaces with
/// natural lifecycle management. All workspace operations are synthesized
/// into the unified field. Each action targets a specific workspace
/// identified by `workspace_id`.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceAction {
    /// Remember an item in the workspace.
    Remember {
        /// Workspace to store in.
        workspace_id: String,
        /// Item identifier.
        item_id: String,
        /// Item content as JSON.
        content_json: serde_json::Value,
    },
    /// Recall items matching a query.
    Recall {
        /// Workspace to recall from.
        workspace_id: String,
        /// Query string.
        query: String,
    },
    /// Consolidate items in the workspace.
    Consolidate {
        /// Workspace to consolidate.
        workspace_id: String,
    },
    /// Search the workspace with options.
    Search {
        /// Workspace to search.
        workspace_id: String,
        /// Search pattern.
        pattern: String,
        /// Search options.
        options: WorkspaceSearchOptions,
    },
}

/// Serializable version of WorkspaceAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum WorkspaceActionSerializable {
    Remember { workspace_id: String, item_id: String, content_json: serde_json::Value },
    Recall { workspace_id: String, query: String },
    Consolidate { workspace_id: String },
    Search { workspace_id: String, pattern: String, options: WorkspaceSearchOptionsSerializable },
}

impl From<&WorkspaceAction> for WorkspaceActionSerializable {
    fn from(action: &WorkspaceAction) -> Self {
        match action {
            WorkspaceAction::Remember { workspace_id, item_id, content_json } => {
                WorkspaceActionSerializable::Remember {
                    workspace_id: workspace_id.clone(),
                    item_id: item_id.clone(),
                    content_json: content_json.clone(),
                }
            }
            WorkspaceAction::Recall { workspace_id, query } => {
                WorkspaceActionSerializable::Recall {
                    workspace_id: workspace_id.clone(),
                    query: query.clone(),
                }
            }
            WorkspaceAction::Consolidate { workspace_id } => {
                WorkspaceActionSerializable::Consolidate {
                    workspace_id: workspace_id.clone(),
                }
            }
            WorkspaceAction::Search { workspace_id, pattern, options } => {
                WorkspaceActionSerializable::Search {
                    workspace_id: workspace_id.clone(),
                    pattern: pattern.clone(),
                    options: options.into(),
                }
            }
        }
    }
}

impl WorkspaceAction {
    /// Validate the workspace action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            WorkspaceAction::Remember { workspace_id, item_id, content_json } => {
                if workspace_id.is_empty() {
                    return Err("WorkspaceAction::Remember: workspace_id is empty".to_string());
                }
                if item_id.is_empty() {
                    return Err("WorkspaceAction::Remember: item_id is empty".to_string());
                }
                if content_json.is_null() {
                    return Err("WorkspaceAction::Remember: content_json is null".to_string());
                }
                Ok(())
            }
            WorkspaceAction::Recall { workspace_id, query } => {
                if workspace_id.is_empty() {
                    return Err("WorkspaceAction::Recall: workspace_id is empty".to_string());
                }
                if query.is_empty() {
                    return Err("WorkspaceAction::Recall: query is empty".to_string());
                }
                Ok(())
            }
            WorkspaceAction::Consolidate { workspace_id } => {
                if workspace_id.is_empty() {
                    return Err("WorkspaceAction::Consolidate: workspace_id is empty".to_string());
                }
                Ok(())
            }
            WorkspaceAction::Search { workspace_id, pattern, .. } => {
                if workspace_id.is_empty() {
                    return Err("WorkspaceAction::Search: workspace_id is empty".to_string());
                }
                if pattern.is_empty() {
                    return Err("WorkspaceAction::Search: pattern is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

impl Canonicalizable for WorkspaceAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = WorkspaceActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

/// Actions for the vector agent.
///
/// The vector agent manages embedding vectors and similarity search.
/// All vector operations are synthesized into the unified field.
#[derive(Debug, Clone, PartialEq)]
pub enum VectorAction {
    /// Embed data into a vector.
    Embed {
        /// Data to embed (as JSON).
        data_json: serde_json::Value,
        /// Embedding model identifier.
        model: String,
        /// Vector dimensions.
        dimensions: usize,
    },
    /// Search for similar vectors.
    Search {
        /// Query vector (as array of floats).
        query_vector: Vec<f32>,
        /// Top-k results.
        top_k: usize,
        /// Similarity threshold.
        threshold: f32,
    },
    /// Index a vector with a key.
    Index {
        /// Vector to index (as array of floats).
        vector: Vec<f32>,
        /// Key to associate with the vector.
        key: String,
        /// Model identifier.
        model: String,
    },
}

/// Serializable version of VectorAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum VectorActionSerializable {
    Embed { data_json: serde_json::Value, model: String, dimensions: usize },
    Search { query_vector: Vec<f32>, top_k: usize, threshold: f32 },
    Index { vector: Vec<f32>, key: String, model: String },
}

impl From<&VectorAction> for VectorActionSerializable {
    fn from(action: &VectorAction) -> Self {
        match action {
            VectorAction::Embed { data_json, model, dimensions } => {
                VectorActionSerializable::Embed {
                    data_json: data_json.clone(),
                    model: model.clone(),
                    dimensions: *dimensions,
                }
            }
            VectorAction::Search { query_vector, top_k, threshold } => {
                VectorActionSerializable::Search {
                    query_vector: query_vector.clone(),
                    top_k: *top_k,
                    threshold: *threshold,
                }
            }
            VectorAction::Index { vector, key, model } => {
                VectorActionSerializable::Index {
                    vector: vector.clone(),
                    key: key.clone(),
                    model: model.clone(),
                }
            }
        }
    }
}

impl VectorAction {
    /// Validate the vector action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            VectorAction::Embed { data_json, model, dimensions } => {
                if data_json.is_null() {
                    return Err("VectorAction::Embed: data_json is null".to_string());
                }
                if model.is_empty() {
                    return Err("VectorAction::Embed: model is empty".to_string());
                }
                if *dimensions == 0 {
                    return Err("VectorAction::Embed: dimensions is zero".to_string());
                }
                Ok(())
            }
            VectorAction::Search { query_vector, top_k, threshold } => {
                if query_vector.is_empty() {
                    return Err("VectorAction::Search: query_vector is empty".to_string());
                }
                if *top_k == 0 {
                    return Err("VectorAction::Search: top_k is zero".to_string());
                }
                if *threshold < 0.0 || *threshold > 1.0 {
                    return Err("VectorAction::Search: threshold must be in [0.0, 1.0]".to_string());
                }
                Ok(())
            }
            VectorAction::Index { vector, key, model } => {
                if vector.is_empty() {
                    return Err("VectorAction::Index: vector is empty".to_string());
                }
                if key.is_empty() {
                    return Err("VectorAction::Index: key is empty".to_string());
                }
                if model.is_empty() {
                    return Err("VectorAction::Index: model is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

impl Canonicalizable for VectorAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = VectorActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

/// Actions for the lifecycle agent.
///
/// The lifecycle agent manages memory tier transitions with LCA architecture.
/// All lifecycle operations are synthesized into the unified field.
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleAction {
    /// Evaluate access patterns for a distinction.
    EvaluateAccess {
        /// Distinction ID to evaluate.
        distinction_id: String,
        /// Full key for the distinction.
        full_key: crate::types::FullKey,
    },
    /// Promote a distinction to a higher tier.
    Promote {
        /// Distinction ID to promote.
        distinction_id: String,
        /// Source tier.
        from_tier: crate::lifecycle::MemoryTier,
        /// Target tier.
        to_tier: crate::lifecycle::MemoryTier,
    },
    /// Demote a distinction to a lower tier.
    Demote {
        /// Distinction ID to demote.
        distinction_id: String,
        /// Source tier.
        from_tier: crate::lifecycle::MemoryTier,
        /// Target tier.
        to_tier: crate::lifecycle::MemoryTier,
    },
    /// Execute a batch of transitions.
    Transition {
        /// Transitions to execute.
        transitions: Vec<crate::lifecycle::Transition>,
    },
    /// Update lifecycle thresholds.
    UpdateThresholds {
        /// New threshold configuration.
        thresholds: serde_json::Value,
    },
    /// Run memory consolidation.
    Consolidate,
    /// Extract genome for deep storage.
    ExtractGenome,
}

/// Serializable version of LifecycleAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum LifecycleActionSerializable {
    EvaluateAccess { distinction_id: String, full_key: crate::types::FullKey },
    Promote { distinction_id: String, from_tier: String, to_tier: String },
    Demote { distinction_id: String, from_tier: String, to_tier: String },
    Transition { transitions: Vec<crate::lifecycle::Transition> },
    UpdateThresholds { thresholds: serde_json::Value },
    Consolidate,
    ExtractGenome,
}

impl From<&LifecycleAction> for LifecycleActionSerializable {
    fn from(action: &LifecycleAction) -> Self {
        match action {
            LifecycleAction::EvaluateAccess { distinction_id, full_key } => {
                LifecycleActionSerializable::EvaluateAccess {
                    distinction_id: distinction_id.clone(),
                    full_key: full_key.clone(),
                }
            }
            LifecycleAction::Promote { distinction_id, from_tier, to_tier } => {
                LifecycleActionSerializable::Promote {
                    distinction_id: distinction_id.clone(),
                    from_tier: from_tier.to_string(),
                    to_tier: to_tier.to_string(),
                }
            }
            LifecycleAction::Demote { distinction_id, from_tier, to_tier } => {
                LifecycleActionSerializable::Demote {
                    distinction_id: distinction_id.clone(),
                    from_tier: from_tier.to_string(),
                    to_tier: to_tier.to_string(),
                }
            }
            LifecycleAction::Transition { transitions } => {
                LifecycleActionSerializable::Transition {
                    transitions: transitions.clone(),
                }
            }
            LifecycleAction::UpdateThresholds { thresholds } => {
                LifecycleActionSerializable::UpdateThresholds {
                    thresholds: thresholds.clone(),
                }
            }
            LifecycleAction::Consolidate => LifecycleActionSerializable::Consolidate,
            LifecycleAction::ExtractGenome => LifecycleActionSerializable::ExtractGenome,
        }
    }
}

impl LifecycleAction {
    /// Validate the lifecycle action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            LifecycleAction::EvaluateAccess { distinction_id, .. } => {
                if distinction_id.is_empty() {
                    return Err("LifecycleAction::EvaluateAccess: distinction_id is empty".to_string());
                }
                Ok(())
            }
            LifecycleAction::Promote { distinction_id, from_tier, to_tier } => {
                if distinction_id.is_empty() {
                    return Err("LifecycleAction::Promote: distinction_id is empty".to_string());
                }
                if from_tier == to_tier {
                    return Err("LifecycleAction::Promote: from_tier equals to_tier".to_string());
                }
                Ok(())
            }
            LifecycleAction::Demote { distinction_id, from_tier, to_tier } => {
                if distinction_id.is_empty() {
                    return Err("LifecycleAction::Demote: distinction_id is empty".to_string());
                }
                if from_tier == to_tier {
                    return Err("LifecycleAction::Demote: from_tier equals to_tier".to_string());
                }
                Ok(())
            }
            LifecycleAction::Transition { transitions } => {
                if transitions.is_empty() {
                    return Err("LifecycleAction::Transition: transitions is empty".to_string());
                }
                Ok(())
            }
            LifecycleAction::UpdateThresholds { thresholds } => {
                if thresholds.is_null() {
                    return Err("LifecycleAction::UpdateThresholds: thresholds is null".to_string());
                }
                Ok(())
            }
            LifecycleAction::Consolidate => Ok(()),
            LifecycleAction::ExtractGenome => Ok(()),
        }
    }
}

impl Canonicalizable for LifecycleAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = LifecycleActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// SESSION ACTIONS
// ============================================================================

/// Actions for session management agent.
///
/// Session operations follow the LCA pattern:
/// - Each session operation synthesizes a new distinction
/// - Sessions are content-addressed by their action history
/// - All session state changes are causal distinctions
#[derive(Debug, Clone, PartialEq)]
pub enum SessionAction {
    /// Create a new session after successful authentication.
    CreateSession {
        /// Identity public key.
        identity_key: String,
        /// Challenge used for authentication.
        challenge: String,
        /// Capabilities granted to this session.
        capabilities: Vec<crate::auth::types::CapabilityRef>,
    },
    /// Validate a session (check if it exists and is not expired).
    ValidateSession {
        /// Session ID to validate.
        session_id: String,
    },
    /// Refresh a session to extend its expiry.
    RefreshSession {
        /// Session ID to refresh.
        session_id: String,
    },
    /// Invalidate (revoke) a session.
    InvalidateSession {
        /// Session ID to invalidate.
        session_id: String,
    },
    /// Rotate session keys.
    RotateKeys {
        /// Session ID to rotate keys for.
        session_id: String,
    },
    /// Clean up expired sessions.
    CleanupExpired,
    /// Revoke all sessions for an identity.
    RevokeAllForIdentity {
        /// Identity public key.
        identity_key: String,
    },
}

/// Serializable version of SessionAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum SessionActionSerializable {
    CreateSession { identity_key: String, challenge: String, capabilities: Vec<crate::auth::types::CapabilityRef> },
    ValidateSession { session_id: String },
    RefreshSession { session_id: String },
    InvalidateSession { session_id: String },
    RotateKeys { session_id: String },
    CleanupExpired,
    RevokeAllForIdentity { identity_key: String },
}

impl From<&SessionAction> for SessionActionSerializable {
    fn from(action: &SessionAction) -> Self {
        match action {
            SessionAction::CreateSession { identity_key, challenge, capabilities } => {
                SessionActionSerializable::CreateSession {
                    identity_key: identity_key.clone(),
                    challenge: challenge.clone(),
                    capabilities: capabilities.clone(),
                }
            }
            SessionAction::ValidateSession { session_id } => {
                SessionActionSerializable::ValidateSession {
                    session_id: session_id.clone(),
                }
            }
            SessionAction::RefreshSession { session_id } => {
                SessionActionSerializable::RefreshSession {
                    session_id: session_id.clone(),
                }
            }
            SessionAction::InvalidateSession { session_id } => {
                SessionActionSerializable::InvalidateSession {
                    session_id: session_id.clone(),
                }
            }
            SessionAction::RotateKeys { session_id } => {
                SessionActionSerializable::RotateKeys {
                    session_id: session_id.clone(),
                }
            }
            SessionAction::CleanupExpired => SessionActionSerializable::CleanupExpired,
            SessionAction::RevokeAllForIdentity { identity_key } => {
                SessionActionSerializable::RevokeAllForIdentity {
                    identity_key: identity_key.clone(),
                }
            }
        }
    }
}

impl SessionAction {
    /// Validate the session action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            SessionAction::CreateSession { identity_key, challenge, .. } => {
                if identity_key.is_empty() {
                    return Err("SessionAction::CreateSession: identity_key is empty".to_string());
                }
                if challenge.is_empty() {
                    return Err("SessionAction::CreateSession: challenge is empty".to_string());
                }
                Ok(())
            }
            SessionAction::ValidateSession { session_id } => {
                if session_id.is_empty() {
                    return Err("SessionAction::ValidateSession: session_id is empty".to_string());
                }
                Ok(())
            }
            SessionAction::RefreshSession { session_id } => {
                if session_id.is_empty() {
                    return Err("SessionAction::RefreshSession: session_id is empty".to_string());
                }
                Ok(())
            }
            SessionAction::InvalidateSession { session_id } => {
                if session_id.is_empty() {
                    return Err("SessionAction::InvalidateSession: session_id is empty".to_string());
                }
                Ok(())
            }
            SessionAction::RotateKeys { session_id } => {
                if session_id.is_empty() {
                    return Err("SessionAction::RotateKeys: session_id is empty".to_string());
                }
                Ok(())
            }
            SessionAction::CleanupExpired => Ok(()),
            SessionAction::RevokeAllForIdentity { identity_key } => {
                if identity_key.is_empty() {
                    return Err("SessionAction::RevokeAllForIdentity: identity_key is empty".to_string());
                }
                Ok(())
            }
        }
    }
}

impl Canonicalizable for SessionAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = SessionActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// SUBSCRIPTION ACTIONS
// ============================================================================

/// Actions for subscription management agent.
///
/// Subscription operations follow the LCA pattern:
/// - Each subscription operation synthesizes a new distinction
/// - Subscriptions are content-addressed by their action history
/// - All subscription state changes are causal distinctions
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionAction {
    /// Subscribe to changes.
    Subscribe {
        /// Subscription definition.
        subscription: crate::subscriptions::Subscription,
    },
    /// Unsubscribe from changes.
    Unsubscribe {
        /// Subscription ID to unsubscribe.
        subscription_id: crate::subscriptions::SubscriptionId,
    },
    /// Notify subscribers of a change event.
    Notify {
        /// Change event to broadcast.
        event: crate::subscriptions::ChangeEvent,
    },
    /// Update an existing subscription's query.
    UpdateSubscription {
        /// Subscription ID to update.
        subscription_id: crate::subscriptions::SubscriptionId,
        /// New subscription definition.
        new_subscription: crate::subscriptions::Subscription,
    },
    /// List all active subscriptions.
    ListSubscriptions,
    /// Get subscription info.
    GetSubscription {
        /// Subscription ID to query.
        subscription_id: crate::subscriptions::SubscriptionId,
    },
}

/// Serializable version of SubscriptionAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum SubscriptionActionSerializable {
    Subscribe { subscription: crate::subscriptions::Subscription },
    Unsubscribe { subscription_id: u64 },
    Notify { event: crate::subscriptions::ChangeEvent },
    UpdateSubscription { subscription_id: u64, new_subscription: crate::subscriptions::Subscription },
    ListSubscriptions,
    GetSubscription { subscription_id: u64 },
}

impl From<&SubscriptionAction> for SubscriptionActionSerializable {
    fn from(action: &SubscriptionAction) -> Self {
        match action {
            SubscriptionAction::Subscribe { subscription } => {
                SubscriptionActionSerializable::Subscribe {
                    subscription: subscription.clone(),
                }
            }
            SubscriptionAction::Unsubscribe { subscription_id } => {
                SubscriptionActionSerializable::Unsubscribe {
                    subscription_id: subscription_id.0,
                }
            }
            SubscriptionAction::Notify { event } => {
                SubscriptionActionSerializable::Notify {
                    event: event.clone(),
                }
            }
            SubscriptionAction::UpdateSubscription { subscription_id, new_subscription } => {
                SubscriptionActionSerializable::UpdateSubscription {
                    subscription_id: subscription_id.0,
                    new_subscription: new_subscription.clone(),
                }
            }
            SubscriptionAction::ListSubscriptions => SubscriptionActionSerializable::ListSubscriptions,
            SubscriptionAction::GetSubscription { subscription_id } => {
                SubscriptionActionSerializable::GetSubscription {
                    subscription_id: subscription_id.0,
                }
            }
        }
    }
}

impl SubscriptionAction {
    /// Validate the subscription action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            SubscriptionAction::Subscribe { subscription } => {
                if subscription.change_types.is_empty() {
                    return Err("SubscriptionAction::Subscribe: change_types is empty".to_string());
                }
                Ok(())
            }
            SubscriptionAction::Unsubscribe { subscription_id } => {
                if subscription_id.0 == 0 {
                    return Err("SubscriptionAction::Unsubscribe: subscription_id is 0".to_string());
                }
                Ok(())
            }
            SubscriptionAction::Notify { .. } => Ok(()),
            SubscriptionAction::UpdateSubscription { subscription_id, .. } => {
                if subscription_id.0 == 0 {
                    return Err("SubscriptionAction::UpdateSubscription: subscription_id is 0".to_string());
                }
                Ok(())
            }
            SubscriptionAction::ListSubscriptions => Ok(()),
            SubscriptionAction::GetSubscription { subscription_id } => {
                if subscription_id.0 == 0 {
                    return Err("SubscriptionAction::GetSubscription: subscription_id is 0".to_string());
                }
                Ok(())
            }
        }
    }
}

impl Canonicalizable for SubscriptionAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = SubscriptionActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// PROCESS ACTIONS
// ============================================================================

/// Actions for process management agent.
///
/// Process operations follow the LCA pattern:
/// - Each process operation synthesizes a new distinction
/// - Process lifecycle is content-addressed by action history
/// - All process state changes are causal distinctions
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessAction {
    /// Spawn a new background process.
    SpawnProcess {
        /// Type of process to spawn.
        process_type: ProcessType,
        /// Process configuration.
        config: ProcessConfig,
    },
    /// Pause a running process.
    PauseProcess {
        /// Process ID to pause.
        process_id: String,
    },
    /// Resume a paused process.
    ResumeProcess {
        /// Process ID to resume.
        process_id: String,
    },
    /// Terminate a process.
    TerminateProcess {
        /// Process ID to terminate.
        process_id: String,
    },
    /// Send heartbeat to keep process alive.
    Heartbeat {
        /// Process ID to heartbeat.
        process_id: String,
    },
    /// Get process status.
    GetStatus {
        /// Process ID to query.
        process_id: String,
    },
    /// List all running processes.
    ListProcesses,
}

/// Type of evolutionary process.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProcessType {
    /// Consolidation process - rhythmic memory movement.
    Consolidation,
    /// Distillation process - fitness-based selection.
    Distillation,
    /// Genome update process - DNA maintenance.
    GenomeUpdate,
}

/// Configuration for process spawning.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProcessConfig {
    /// Interval in seconds between process runs.
    pub interval_secs: u64,
    /// Whether process should auto-start.
    pub auto_start: bool,
    /// Process-specific configuration as JSON.
    pub config_json: serde_json::Value,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            interval_secs: 3600,
            auto_start: true,
            config_json: serde_json::json!({}),
        }
    }
}

/// Serializable version of ProcessAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum ProcessActionSerializable {
    SpawnProcess { process_type: ProcessType, config: ProcessConfig },
    PauseProcess { process_id: String },
    ResumeProcess { process_id: String },
    TerminateProcess { process_id: String },
    Heartbeat { process_id: String },
    GetStatus { process_id: String },
    ListProcesses,
}

impl From<&ProcessAction> for ProcessActionSerializable {
    fn from(action: &ProcessAction) -> Self {
        match action {
            ProcessAction::SpawnProcess { process_type, config } => {
                ProcessActionSerializable::SpawnProcess {
                    process_type: process_type.clone(),
                    config: config.clone(),
                }
            }
            ProcessAction::PauseProcess { process_id } => {
                ProcessActionSerializable::PauseProcess {
                    process_id: process_id.clone(),
                }
            }
            ProcessAction::ResumeProcess { process_id } => {
                ProcessActionSerializable::ResumeProcess {
                    process_id: process_id.clone(),
                }
            }
            ProcessAction::TerminateProcess { process_id } => {
                ProcessActionSerializable::TerminateProcess {
                    process_id: process_id.clone(),
                }
            }
            ProcessAction::Heartbeat { process_id } => {
                ProcessActionSerializable::Heartbeat {
                    process_id: process_id.clone(),
                }
            }
            ProcessAction::GetStatus { process_id } => {
                ProcessActionSerializable::GetStatus {
                    process_id: process_id.clone(),
                }
            }
            ProcessAction::ListProcesses => ProcessActionSerializable::ListProcesses,
        }
    }
}

impl ProcessAction {
    /// Validate the process action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            ProcessAction::SpawnProcess { process_type, config } => {
                if config.interval_secs == 0 {
                    return Err("ProcessAction::SpawnProcess: interval_secs cannot be 0".to_string());
                }
                match process_type {
                    ProcessType::Consolidation | ProcessType::Distillation | ProcessType::GenomeUpdate => Ok(()),
                }
            }
            ProcessAction::PauseProcess { process_id } => {
                if process_id.is_empty() {
                    return Err("ProcessAction::PauseProcess: process_id is empty".to_string());
                }
                Ok(())
            }
            ProcessAction::ResumeProcess { process_id } => {
                if process_id.is_empty() {
                    return Err("ProcessAction::ResumeProcess: process_id is empty".to_string());
                }
                Ok(())
            }
            ProcessAction::TerminateProcess { process_id } => {
                if process_id.is_empty() {
                    return Err("ProcessAction::TerminateProcess: process_id is empty".to_string());
                }
                Ok(())
            }
            ProcessAction::Heartbeat { process_id } => {
                if process_id.is_empty() {
                    return Err("ProcessAction::Heartbeat: process_id is empty".to_string());
                }
                Ok(())
            }
            ProcessAction::GetStatus { process_id } => {
                if process_id.is_empty() {
                    return Err("ProcessAction::GetStatus: process_id is empty".to_string());
                }
                Ok(())
            }
            ProcessAction::ListProcesses => Ok(()),
        }
    }
}

impl Canonicalizable for ProcessAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = ProcessActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// RECONCILIATION ACTIONS
// ============================================================================

/// Actions for distributed reconciliation agent.
///
/// Reconciliation operations follow the LCA pattern:
/// - Each sync operation synthesizes a new distinction
/// - Set reconciliation is content-addressed by action history
/// - All sync state changes are causal distinctions
#[derive(Debug, Clone, PartialEq)]
pub enum ReconciliationAction {
    /// Start synchronization with a peer.
    StartSync {
        /// Peer ID to sync with.
        peer_id: String,
    },
    /// Exchange Merkle roots with peer.
    ExchangeRoots {
        /// Remote peer's frontier/root hash.
        peer_frontier: [u8; 32],
    },
    /// Request differences at divergence point.
    RequestDifferences {
        /// Point where trees diverge.
        divergence_point: String,
    },
    /// Apply received changes.
    ApplyDelta {
        /// Changes to apply.
        changes: Vec<String>,
    },
    /// Resolve a conflict.
    ResolveConflict {
        /// Conflict ID.
        conflict_id: String,
        /// Resolution strategy.
        resolution: ConflictResolution,
    },
    /// Complete synchronization.
    CompleteSync {
        /// Peer ID that sync completed with.
        peer_id: String,
    },
    /// Get sync status.
    GetSyncStatus,
}

/// Conflict resolution strategies.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ConflictResolution {
    /// Prefer local version.
    PreferLocal,
    /// Prefer remote version.
    PreferRemote,
    /// Merge both versions.
    Merge,
    /// Manual resolution required.
    Manual,
}

/// Serializable version of ReconciliationAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum ReconciliationActionSerializable {
    StartSync { peer_id: String },
    ExchangeRoots { peer_frontier: [u8; 32] },
    RequestDifferences { divergence_point: String },
    ApplyDelta { changes: Vec<String> },
    ResolveConflict { conflict_id: String, resolution: ConflictResolution },
    CompleteSync { peer_id: String },
    GetSyncStatus,
}

impl From<&ReconciliationAction> for ReconciliationActionSerializable {
    fn from(action: &ReconciliationAction) -> Self {
        match action {
            ReconciliationAction::StartSync { peer_id } => {
                ReconciliationActionSerializable::StartSync {
                    peer_id: peer_id.clone(),
                }
            }
            ReconciliationAction::ExchangeRoots { peer_frontier } => {
                ReconciliationActionSerializable::ExchangeRoots {
                    peer_frontier: *peer_frontier,
                }
            }
            ReconciliationAction::RequestDifferences { divergence_point } => {
                ReconciliationActionSerializable::RequestDifferences {
                    divergence_point: divergence_point.clone(),
                }
            }
            ReconciliationAction::ApplyDelta { changes } => {
                ReconciliationActionSerializable::ApplyDelta {
                    changes: changes.clone(),
                }
            }
            ReconciliationAction::ResolveConflict { conflict_id, resolution } => {
                ReconciliationActionSerializable::ResolveConflict {
                    conflict_id: conflict_id.clone(),
                    resolution: *resolution,
                }
            }
            ReconciliationAction::CompleteSync { peer_id } => {
                ReconciliationActionSerializable::CompleteSync {
                    peer_id: peer_id.clone(),
                }
            }
            ReconciliationAction::GetSyncStatus => ReconciliationActionSerializable::GetSyncStatus,
        }
    }
}

impl ReconciliationAction {
    /// Validate the reconciliation action.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            ReconciliationAction::StartSync { peer_id } => {
                if peer_id.is_empty() {
                    return Err("ReconciliationAction::StartSync: peer_id is empty".to_string());
                }
                Ok(())
            }
            ReconciliationAction::ExchangeRoots { .. } => Ok(()),
            ReconciliationAction::RequestDifferences { divergence_point } => {
                if divergence_point.is_empty() {
                    return Err("ReconciliationAction::RequestDifferences: divergence_point is empty".to_string());
                }
                Ok(())
            }
            ReconciliationAction::ApplyDelta { changes } => {
                if changes.is_empty() {
                    return Err("ReconciliationAction::ApplyDelta: changes is empty".to_string());
                }
                Ok(())
            }
            ReconciliationAction::ResolveConflict { conflict_id, .. } => {
                if conflict_id.is_empty() {
                    return Err("ReconciliationAction::ResolveConflict: conflict_id is empty".to_string());
                }
                Ok(())
            }
            ReconciliationAction::CompleteSync { peer_id } => {
                if peer_id.is_empty() {
                    return Err("ReconciliationAction::CompleteSync: peer_id is empty".to_string());
                }
                Ok(())
            }
            ReconciliationAction::GetSyncStatus => Ok(()),
        }
    }
}

impl Canonicalizable for ReconciliationAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = ReconciliationActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// WORKSPACE SEARCH OPTIONS
// ============================================================================

/// Options for workspace search operations.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSearchOptions {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Tag filters to apply.
    pub tag_filters: Vec<String>,
    /// Whether to include deleted items.
    pub include_deleted: bool,
}

impl Default for WorkspaceSearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            tag_filters: Vec::new(),
            include_deleted: false,
        }
    }
}

/// Serializable version of WorkspaceSearchOptions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceSearchOptionsSerializable {
    pub limit: usize,
    pub tag_filters: Vec<String>,
    pub include_deleted: bool,
}

impl Default for WorkspaceSearchOptionsSerializable {
    fn default() -> Self {
        Self {
            limit: 10,
            tag_filters: Vec::new(),
            include_deleted: false,
        }
    }
}

impl From<&WorkspaceSearchOptions> for WorkspaceSearchOptionsSerializable {
    fn from(options: &WorkspaceSearchOptions) -> Self {
        Self {
            limit: options.limit,
            tag_filters: options.tag_filters.clone(),
            include_deleted: options.include_deleted,
        }
    }
}

// ============================================================================
// CANONICALIZABLE IMPLEMENTATIONS
// ============================================================================
// Each action type must be convertible to a canonical distinction structure.
// This follows the LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction_Data
// ============================================================================

impl Canonicalizable for TemperatureAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = TemperatureActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for ChronicleAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = ChronicleActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for ArchiveAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = ArchiveActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for EssenceAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = EssenceActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for SleepAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = SleepActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for EvolutionAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = EvolutionActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for LineageAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = LineageActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for PerspectiveAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = PerspectiveActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for IdentityAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = IdentityActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl Canonicalizable for NetworkAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = NetworkActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// ALIS AI Integration Actions - Phase 1: TTL Support
// ============================================================================

/// Consolidation actions for proactive synthesis and TTL management.
///
/// These actions are performed by the ConsolidationAgent during background
/// processing to maintain the health and coherence of the distinction field.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsolidationAction {
    /// Clean up expired TTL values.
    ///
    /// Removes all values that have exceeded their time-to-live,
    /// returning the count of items removed.
    CleanupExpired,
    /// Find similar distinctions that are not causally connected.
    ///
    /// These pairs are candidates for proactive synthesis.
    FindSimilarUnconnectedPairs {
        /// Maximum number of pairs to return.
        k: usize,
        /// Minimum similarity threshold (0.0 - 1.0).
        threshold: f32,
    },
}

/// Serializable version of ConsolidationAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum ConsolidationActionSerializable {
    CleanupExpired,
    FindSimilarUnconnectedPairs { k: usize, threshold: f32 },
}

impl From<&ConsolidationAction> for ConsolidationActionSerializable {
    fn from(action: &ConsolidationAction) -> Self {
        match action {
            ConsolidationAction::CleanupExpired => {
                ConsolidationActionSerializable::CleanupExpired
            }
            ConsolidationAction::FindSimilarUnconnectedPairs { k, threshold } => {
                ConsolidationActionSerializable::FindSimilarUnconnectedPairs {
                    k: *k,
                    threshold: *threshold,
                }
            }
        }
    }
}

impl Canonicalizable for ConsolidationAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = ConsolidationActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

impl From<ConsolidationAction> for KoruAction {
    fn from(action: ConsolidationAction) -> Self {
        // For now, map to Storage action as a placeholder
        // In full implementation, would add KoruAction::Consolidation variant
        KoruAction::Storage(StorageAction::Query {
            pattern_json: serde_json::json!({
                "consolidation_action": format!("{:?}", action)
            }),
        })
    }
}

// ============================================================================
// ALIS AI Integration - Extended Lineage Actions
// ============================================================================

/// Extended lineage actions for graph connectivity queries.
///
/// These actions query the causal graph structure to understand
/// relationships between distinctions.
#[derive(Debug, Clone, PartialEq)]
pub enum LineageQueryAction {
    /// Check if two distinctions are causally connected.
    QueryConnected {
        /// First distinction key.
        key_a: String,
        /// Second distinction key.
        key_b: String,
    },
    /// Get the causal connection path between two distinctions.
    GetConnectionPath {
        /// First distinction key.
        key_a: String,
        /// Second distinction key.
        key_b: String,
    },
    /// Get the most highly-connected distinctions.
    GetHighlyConnected {
        /// Maximum number of results.
        k: usize,
    },
}

/// Serializable version of LineageQueryAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum LineageQueryActionSerializable {
    QueryConnected { key_a: String, key_b: String },
    GetConnectionPath { key_a: String, key_b: String },
    GetHighlyConnected { k: usize },
}

impl From<&LineageQueryAction> for LineageQueryActionSerializable {
    fn from(action: &LineageQueryAction) -> Self {
        match action {
            LineageQueryAction::QueryConnected { key_a, key_b } => {
                LineageQueryActionSerializable::QueryConnected {
                    key_a: key_a.clone(),
                    key_b: key_b.clone(),
                }
            }
            LineageQueryAction::GetConnectionPath { key_a, key_b } => {
                LineageQueryActionSerializable::GetConnectionPath {
                    key_a: key_a.clone(),
                    key_b: key_b.clone(),
                }
            }
            LineageQueryAction::GetHighlyConnected { k } => {
                LineageQueryActionSerializable::GetHighlyConnected { k: *k }
            }
        }
    }
}

impl Canonicalizable for LineageQueryAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = LineageQueryActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}

// ============================================================================
// ALIS AI Integration - Extended Sleep Actions
// ============================================================================

/// Extended sleep actions for creative synthesis.
///
/// These actions are performed during the dream phase to create
/// novel combinations of distant distinctions.
#[derive(Debug, Clone, PartialEq)]
pub enum SleepCreativeAction {
    /// Perform random walk combinations for creative synthesis.
    RandomWalkCombinations {
        /// Number of combinations to generate.
        n: usize,
        /// Number of steps per random walk.
        steps: usize,
    },
}

/// Serializable version of SleepCreativeAction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum SleepCreativeActionSerializable {
    RandomWalkCombinations { n: usize, steps: usize },
}

impl From<&SleepCreativeAction> for SleepCreativeActionSerializable {
    fn from(action: &SleepCreativeAction) -> Self {
        match action {
            SleepCreativeAction::RandomWalkCombinations { n, steps } => {
                SleepCreativeActionSerializable::RandomWalkCombinations {
                    n: *n,
                    steps: *steps,
                }
            }
        }
    }
}

impl Canonicalizable for SleepCreativeAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let serializable = SleepCreativeActionSerializable::from(self);
        match bincode::serialize(&serializable) {
            Ok(bytes) => bytes_to_distinction(&bytes, engine),
            Err(_) => engine.d0().clone(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_temperature_levels() {
        assert_ne!(TemperatureLevel::Hot, TemperatureLevel::Cold);
        assert_ne!(TemperatureLevel::Warm, TemperatureLevel::Cool);
    }

    #[test]
    fn test_sleep_phases() {
        assert_ne!(SleepPhase::Awake, SleepPhase::DeepSleep);
        assert_ne!(SleepPhase::LightSleep, SleepPhase::Rem);
    }

    #[test]
    fn test_action_serialization() {
        let original = KoruAction::Storage(StorageAction::Store {
            namespace: "users".to_string(),
            key: "alice".to_string(),
            value_json: serde_json::json!({"name": "Alice"}),
        });

        // Convert to serializable
        let serializable = ActionSerializable::from(&original);
        
        // Serialize to bytes - should succeed
        let bytes = bincode::serialize(&serializable);
        assert!(bytes.is_ok());
        assert!(!bytes.unwrap().is_empty());
        
        // Verify it's the right variant
        match (&original, &serializable) {
            (KoruAction::Storage(_), ActionSerializable::Storage(_)) => {}
            _ => panic!("Serialization failed"),
        }
    }

    #[test]
    fn test_action_to_bytes() {
        let action = KoruAction::Storage(StorageAction::Store {
            namespace: "users".to_string(),
            key: "alice".to_string(),
            value_json: serde_json::json!({"name": "Alice"}),
        });

        let bytes = action.to_bytes();
        assert!(bytes.is_ok());
        assert!(!bytes.unwrap().is_empty());
    }
}
