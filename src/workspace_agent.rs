//! Workspace Agent - Isolated memory spaces with LCA architecture.
//!
//! This module implements the Workspace agent following the Local Causal Agent pattern.
//! From LCA perspective, workspaces are not "containers" but patterns of distinction
//! synthesized from the workspace root. Memory items are synthesized from workspace-local
//! roots.
//!
//! # Key Insight
//!
//! A workspace is not a "place" that "contains" memories. From LCA perspective:
//! - Workspaces are distinct patterns in the field
//! - Memories are synthesized from workspace-local roots
//! - Isolation is achieved through distinct synthesis chains, not access control
//!
//! Formula for workspace operations:
//! ```text
//! ΔNew = ΔWorkspace_Local_Root ⊕ ΔAction_Data
//! ```
//!
//! Formula for workspace creation:
//! ```text
//! ΔNew = ΔWorkspace_Root ⊕ ΔWorkspace_Config
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use serde::{Deserialize, Serialize};

use crate::actions::WorkspaceAction;

/// Convert bytes to distinction via byte-wise synthesis.
fn bytes_to_distinction(bytes: &[u8], engine: &DistinctionEngine) -> Distinction {
    bytes
        .iter()
        .map(|&byte| byte.to_canonical_structure(engine))
        .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
}

/// A synthesized workspace distinction.
///
/// Represents a workspace that has been synthesized into the field.
/// The distinction itself is the workspace; there is no separate "container".
#[derive(Debug, Clone)]
pub struct SynthesizedWorkspace {
    /// The canonical distinction representing this workspace.
    pub distinction: Distinction,

    /// Workspace metadata.
    pub metadata: WorkspaceMetadata,

    /// The local root for this workspace.
    pub local_root: Distinction,

    /// When this workspace was synthesized.
    pub synthesized_at: DateTime<Utc>,
}

/// Metadata for a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    /// Workspace identifier.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of the workspace purpose.
    pub description: String,

    /// Tags for categorization.
    pub tags: Vec<String>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last access timestamp.
    pub last_accessed_at: DateTime<Utc>,

    /// Number of items in this workspace.
    pub item_count: usize,
}

impl WorkspaceMetadata {
    /// Create new metadata for a workspace.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            tags: Vec::new(),
            created_at: now,
            last_accessed_at: now,
            item_count: 0,
        }
    }
}

impl Canonicalizable for WorkspaceMetadata {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        let bytes = bincode::serialize(self).expect("WorkspaceMetadata should serialize");
        bytes_to_distinction(&bytes, engine)
    }
}

/// A synthesized memory item distinction.
///
/// Represents a memory item that has been synthesized into a workspace.
#[derive(Debug, Clone)]
pub struct SynthesizedMemory {
    /// The canonical distinction representing this memory.
    pub distinction: Distinction,

    /// The item identifier.
    pub item_id: String,

    /// The workspace this memory belongs to.
    pub workspace_id: String,

    /// When this memory was synthesized.
    pub synthesized_at: DateTime<Utc>,
}

/// Workspace agent - manages isolated memory spaces.
///
/// The workspace agent follows the LCA pattern:
/// - It has a workspace root distinction
/// - Each operation synthesizes from the current local root
/// - No privileged access - just synthesis
///
/// # Example
///
/// ```rust
/// use koru_delta::workspace_agent::WorkspaceAgent;
/// use koru_delta::roots::KoruRoots;
/// use koru_lambda_core::DistinctionEngine;
/// use std::sync::Arc;
///
/// let engine = Arc::new(DistinctionEngine::new());
/// let roots = KoruRoots::initialize(&engine);
/// let mut agent = WorkspaceAgent::new(roots.workspace.clone(), engine);
///
/// // Synthesize a new workspace
/// let workspace = agent.create_workspace("ws-1", "My Workspace");
/// ```
pub struct WorkspaceAgent {
    /// The engine for synthesis.
    engine: Arc<DistinctionEngine>,

    /// Workspaces by ID.
    workspaces: RwLock<HashMap<String, SynthesizedWorkspace>>,

    /// Memories by workspace ID.
    memories: RwLock<HashMap<String, Vec<SynthesizedMemory>>>,

    /// Current local root for the workspace agent.
    local_root: Distinction,

    /// Operation sequence counter.
    sequence: AtomicU64,

    /// Metrics tracking.
    metrics: RwLock<WorkspaceMetrics>,
}

/// Metrics for workspace operations.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceMetrics {
    /// Number of workspaces created.
    pub workspaces_created: u64,

    /// Number of memories synthesized.
    pub memories_synthesized: u64,

    /// Number of recalls performed.
    pub recalls_performed: u64,

    /// Number of consolidations performed.
    pub consolidations_performed: u64,

    /// Number of searches performed.
    pub searches_performed: u64,
}

impl WorkspaceAgent {
    /// Create a new workspace agent.
    ///
    /// # Arguments
    ///
    /// * `workspace_root` - The canonical workspace root (becomes initial local_root)
    /// * `engine` - The distinction engine for synthesis
    pub fn new(workspace_root: Distinction, engine: Arc<DistinctionEngine>) -> Self {
        Self {
            engine,
            workspaces: RwLock::new(HashMap::new()),
            memories: RwLock::new(HashMap::new()),
            local_root: workspace_root,
            sequence: AtomicU64::new(0),
            metrics: RwLock::new(WorkspaceMetrics::default()),
        }
    }

    /// Get the current local root.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Get current metrics.
    pub fn metrics(&self) -> WorkspaceMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Synthesize a workspace into the field.
    ///
    /// Formula: ΔNew = ΔLocal_Root ⊕ ΔWorkspace_Config
    fn synthesize_workspace(&mut self, metadata: WorkspaceMetadata) -> SynthesizedWorkspace {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let local_root = self.local_root();

        // Synthesize the workspace configuration
        let config_distinction = metadata.to_canonical_structure(&self.engine);

        // Include sequence in the synthesis for uniqueness
        let seq_bytes = bincode::serialize(&seq).unwrap();
        let seq_distinction = bytes_to_distinction(&seq_bytes, &self.engine);
        let config_distinction = self.engine.synthesize(&config_distinction, &seq_distinction);

        // Synthesize new workspace from local root
        let distinction = self.engine.synthesize(local_root, &config_distinction);

        // Create a local root for this specific workspace
        let local_root_seed = format!("workspace-local-{}", metadata.id);
        let workspace_local_root = bytes_to_distinction(local_root_seed.as_bytes(), &self.engine);
        let workspace_local_root = self.engine.synthesize(&distinction, &workspace_local_root);

        let workspace = SynthesizedWorkspace {
            distinction: distinction.clone(),
            metadata: metadata.clone(),
            local_root: workspace_local_root,
            synthesized_at: Utc::now(),
        };

        // Update the agent's local root
        self.local_root = distinction;

        // Store the workspace
        self.workspaces
            .write()
            .unwrap()
            .insert(metadata.id.clone(), workspace.clone());

        // Initialize memory storage for this workspace
        self.memories
            .write()
            .unwrap()
            .insert(metadata.id.clone(), Vec::new());

        self.metrics.write().unwrap().workspaces_created += 1;

        workspace
    }

    /// Create a new workspace.
    pub fn create_workspace(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> SynthesizedWorkspace {
        let metadata = WorkspaceMetadata::new(id, name);
        self.synthesize_workspace(metadata)
    }

    /// Get a workspace by ID.
    pub fn get_workspace(&self, id: &str) -> Option<SynthesizedWorkspace> {
        self.workspaces.read().unwrap().get(id).cloned()
    }

    /// List all workspaces.
    pub fn list_workspaces(&self) -> Vec<SynthesizedWorkspace> {
        self.workspaces.read().unwrap().values().cloned().collect()
    }

    /// Synthesize a memory item into a workspace.
    ///
    /// Formula: ΔNew = ΔWorkspace_Local_Root ⊕ ΔContent ⊕ ΔItem_ID
    pub fn remember(
        &self,
        workspace_id: &str,
        item_id: impl Into<String>,
        content: &[u8],
    ) -> Option<SynthesizedMemory> {
        let workspace = self.get_workspace(workspace_id)?;
        let item_id = item_id.into();

        // Synthesize the content
        let content_distinction = bytes_to_distinction(content, &self.engine);

        // Include item_id in synthesis
        let id_distinction = bytes_to_distinction(item_id.as_bytes(), &self.engine);
        let content_distinction = self.engine.synthesize(&content_distinction, &id_distinction);

        // Synthesize from workspace local root
        let distinction = self
            .engine
            .synthesize(&workspace.local_root, &content_distinction);

        let memory = SynthesizedMemory {
            distinction,
            item_id: item_id.clone(),
            workspace_id: workspace_id.to_string(),
            synthesized_at: Utc::now(),
        };

        // Store the memory
        if let Some(memories) = self.memories.write().unwrap().get_mut(workspace_id) {
            memories.push(memory.clone());
        }

        // Update workspace item count
        if let Some(ws) = self.workspaces.write().unwrap().get_mut(workspace_id) {
            ws.metadata.item_count += 1;
            ws.metadata.last_accessed_at = Utc::now();
        }

        self.metrics.write().unwrap().memories_synthesized += 1;

        Some(memory)
    }

    /// Recall memories from a workspace.
    ///
    /// Returns all memories for the given workspace.
    pub fn recall(&self, workspace_id: &str) -> Option<Vec<SynthesizedMemory>> {
        let memories = self.memories.read().unwrap().get(workspace_id)?.clone();

        // Update access time
        if let Some(ws) = self.workspaces.write().unwrap().get_mut(workspace_id) {
            ws.metadata.last_accessed_at = Utc::now();
        }

        self.metrics.write().unwrap().recalls_performed += 1;

        Some(memories)
    }

    /// Search for memories by pattern.
    ///
    /// Simple pattern matching on item IDs. For semantic search,
    /// use the VectorAgent.
    pub fn search(&self, workspace_id: &str, pattern: &str) -> Option<Vec<SynthesizedMemory>> {
        let memories = self.memories.read().unwrap();
        let workspace_memories = memories.get(workspace_id)?;

        let results: Vec<SynthesizedMemory> = workspace_memories
            .iter()
            .filter(|m| m.item_id.contains(pattern))
            .cloned()
            .collect();

        // Update access time
        if let Some(ws) = self.workspaces.write().unwrap().get_mut(workspace_id) {
            ws.metadata.last_accessed_at = Utc::now();
        }

        self.metrics.write().unwrap().searches_performed += 1;

        Some(results)
    }

    /// Consolidate memories.
    ///
    /// Synthesizes all memories in a workspace into a consolidated distinction.
    pub fn consolidate(&self, workspace_id: &str) -> Option<Distinction> {
        let workspace = self.get_workspace(workspace_id)?;
        let memories = self.memories.read().unwrap();
        let workspace_memories = memories.get(workspace_id)?;

        if workspace_memories.is_empty() {
            return Some(workspace.local_root.clone());
        }

        // Synthesize all memories together
        let mut consolidated = workspace.local_root.clone();
        for memory in workspace_memories {
            consolidated = self.engine.synthesize(&consolidated, &memory.distinction);
        }

        // Update access time
        if let Some(ws) = self.workspaces.write().unwrap().get_mut(workspace_id) {
            ws.metadata.last_accessed_at = Utc::now();
        }

        self.metrics.write().unwrap().consolidations_performed += 1;

        Some(consolidated)
    }

    /// Execute a workspace action.
    ///
    /// This is the main entry point for workspace operations.
    pub fn execute(&self, action: WorkspaceAction) -> WorkspaceResult {
        match action {
            WorkspaceAction::Remember {
                workspace_id,
                item_id,
                content_json,
            } => {
                let content = serde_json::to_vec(&content_json).unwrap_or_default();
                match self.remember(&workspace_id, item_id, &content) {
                    Some(memory) => WorkspaceResult::Memory(memory),
                    None => WorkspaceResult::Error(format!("Workspace {} not found", workspace_id)),
                }
            }
            WorkspaceAction::Recall { workspace_id, query: _ } => {
                // For now, just return all memories
                // In a more sophisticated implementation, query would filter
                match self.recall(&workspace_id) {
                    Some(memories) => WorkspaceResult::Memories(memories),
                    None => WorkspaceResult::Error(format!("Workspace {} not found", workspace_id)),
                }
            }
            WorkspaceAction::Search {
                workspace_id,
                pattern,
                options: _,
            } => match self.search(&workspace_id, &pattern) {
                Some(memories) => WorkspaceResult::Memories(memories),
                None => WorkspaceResult::Error(format!("Workspace {} not found", workspace_id)),
            },
            WorkspaceAction::Consolidate { workspace_id } => {
                match self.consolidate(&workspace_id) {
                    Some(distinction) => WorkspaceResult::Distinction(distinction),
                    None => WorkspaceResult::Error(format!("Workspace {} not found", workspace_id)),
                }
            }
        }
    }
}

/// Result of a workspace operation.
#[derive(Debug, Clone)]
pub enum WorkspaceResult {
    /// A single memory item.
    Memory(SynthesizedMemory),

    /// Multiple memory items.
    Memories(Vec<SynthesizedMemory>),

    /// A workspace.
    Workspace(SynthesizedWorkspace),

    /// A raw distinction.
    Distinction(Distinction),

    /// An error occurred.
    Error(String),
}

impl fmt::Display for WorkspaceResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceResult::Memory(m) => write!(f, "Memory({})", m.item_id),
            WorkspaceResult::Memories(ms) => write!(f, "Memories({} items)", ms.len()),
            WorkspaceResult::Workspace(w) => write!(f, "Workspace({})", w.metadata.name),
            WorkspaceResult::Distinction(d) => write!(f, "Distinction({})", d.id()),
            WorkspaceResult::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

/// Type alias for backward compatibility.
pub type Workspace = SynthesizedWorkspace;

// LCA Trait Implementation
impl LocalCausalAgent for WorkspaceAgent {
    type ActionData = WorkspaceAction;

    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }

    fn synthesize_action(
        &mut self,
        action: WorkspaceAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Canonical LCA pattern: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        new_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_agent() -> (WorkspaceAgent, Arc<DistinctionEngine>) {
        let engine = Arc::new(DistinctionEngine::new());
        let workspace_root = engine.synthesize(&engine.d0().clone(), &engine.d1().clone());
        let agent = WorkspaceAgent::new(workspace_root, engine.clone());
        (agent, engine)
    }

    #[test]
    fn test_create_workspace() {
        let (mut agent, _) = setup_agent();

        let workspace = agent.create_workspace("ws-1", "Test Workspace");

        assert_eq!(workspace.metadata.id, "ws-1");
        assert_eq!(workspace.metadata.name, "Test Workspace");
        assert_eq!(workspace.metadata.item_count, 0);
    }

    #[test]
    fn test_get_workspace() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");

        let retrieved = agent.get_workspace("ws-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "Test Workspace");

        assert!(agent.get_workspace("nonexistent").is_none());
    }

    #[test]
    fn test_remember_and_recall() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");

        // Remember something
        let content = b"test content";
        let memory = agent.remember("ws-1", "item-1", content);
        assert!(memory.is_some());
        assert_eq!(memory.unwrap().item_id, "item-1");

        // Recall
        let memories = agent.recall("ws-1");
        assert!(memories.is_some());
        assert_eq!(memories.unwrap().len(), 1);
    }

    #[test]
    fn test_workspace_isolation() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Workspace 1");
        agent.create_workspace("ws-2", "Workspace 2");

        // Add to workspace 1
        agent.remember("ws-1", "item-1", b"content 1");

        // Add to workspace 2
        agent.remember("ws-2", "item-2", b"content 2");

        // Verify isolation
        let ws1_memories = agent.recall("ws-1").unwrap();
        let ws2_memories = agent.recall("ws-2").unwrap();

        assert_eq!(ws1_memories.len(), 1);
        assert_eq!(ws2_memories.len(), 1);

        assert_eq!(ws1_memories[0].item_id, "item-1");
        assert_eq!(ws2_memories[0].item_id, "item-2");
    }

    #[test]
    fn test_search() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");

        agent.remember("ws-1", "alpha-one", b"content 1");
        agent.remember("ws-1", "alpha-two", b"content 2");
        agent.remember("ws-1", "beta-one", b"content 3");

        let results = agent.search("ws-1", "alpha").unwrap();
        assert_eq!(results.len(), 2);

        let results = agent.search("ws-1", "two").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_consolidate() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");

        agent.remember("ws-1", "item-1", b"content 1");
        agent.remember("ws-1", "item-2", b"content 2");

        let consolidated = agent.consolidate("ws-1");
        assert!(consolidated.is_some());
    }

    #[test]
    fn test_metrics() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");
        agent.remember("ws-1", "item-1", b"content");
        agent.recall("ws-1");

        let metrics = agent.metrics();
        assert_eq!(metrics.workspaces_created, 1);
        assert_eq!(metrics.memories_synthesized, 1);
        assert_eq!(metrics.recalls_performed, 1);
    }

    #[test]
    fn test_execute_remember() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");

        let action = WorkspaceAction::Remember {
            workspace_id: "ws-1".to_string(),
            item_id: "item-1".to_string(),
            content_json: serde_json::json!({"key": "value"}),
        };

        let result = agent.execute(action);
        match result {
            WorkspaceResult::Memory(m) => assert_eq!(m.item_id, "item-1"),
            _ => panic!("Expected Memory result"),
        }
    }

    #[test]
    fn test_execute_recall() {
        let (mut agent, _) = setup_agent();

        agent.create_workspace("ws-1", "Test Workspace");
        agent.remember("ws-1", "item-1", b"content");

        let action = WorkspaceAction::Recall {
            workspace_id: "ws-1".to_string(),
            query: "test".to_string(),
        };

        let result = agent.execute(action);
        match result {
            WorkspaceResult::Memories(ms) => assert_eq!(ms.len(), 1),
            _ => panic!("Expected Memories result"),
        }
    }

    #[test]
    fn test_workspace_has_unique_local_root() {
        let (mut agent, _engine) = setup_agent();

        let ws1 = agent.create_workspace("ws-1", "Workspace 1");
        let ws2 = agent.create_workspace("ws-2", "Workspace 2");

        // Each workspace should have its own unique local root
        assert_ne!(ws1.local_root.id(), ws2.local_root.id());

        // Each workspace local root should be distinct from the workspace distinction
        assert_ne!(ws1.distinction.id(), ws1.local_root.id());
        assert_ne!(ws2.distinction.id(), ws2.local_root.id());
    }

    #[test]
    fn test_memories_synthesize_from_workspace_local_root() {
        let (mut agent, _engine) = setup_agent();

        let ws = agent.create_workspace("ws-1", "Test Workspace");
        let memory = agent.remember("ws-1", "item-1", b"content").unwrap();

        // The memory distinction ID should be different from the workspace local root
        // because synthesis creates new distinctions
        assert_ne!(memory.distinction.id(), ws.local_root.id());
    }
}
