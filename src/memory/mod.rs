pub mod cold;
pub mod deep;
/// Memory tiering subsystem and workspaces.
///
/// This module provides:
/// - **Workspaces**: Isolated, versioned storage containers with natural lifecycle
/// - **Memory Tiers**: Hot, Warm, Cold, Deep for automatic data lifecycle
///
/// ## Workspaces
///
/// Workspaces are the primary interface for causal storage:
///
/// ```ignore
/// let workspace = db.workspace("project-alpha");
/// workspace.store("config", data, MemoryPattern::Reference).await?;
/// let history = workspace.history("config").await?;
/// ```
///
/// ## Memory Patterns
///
/// Patterns are conventions for organizing data:
/// - **Event**: Things that happened (logs, audit trail, episodes)
/// - **Reference**: Facts and knowledge (config, taxonomy)
/// - **Procedure**: Computable knowledge (workflows, rules)
///
/// ## Memory Tiers
///
/// Data automatically moves through tiers based on access patterns:
///
/// ```text
/// Put: Data → Hot (immediate access)
///          ↓
///     Access stops → Warm (chronicle)
///          ↓
///     Time passes → Cold (consolidated)
///          ↓
///     Epoch ends → Deep (genomic)
/// ```
pub mod hot;
pub mod warm;
pub mod workspace;

pub use cold::{ArchiveAgent, ArchiveConfig, ArchiveStats, ConsolidationResult, Pattern};
pub use deep::{
    CausalTopology, EssenceAgent, EssenceConfig, EssenceStats, EpochSummary, ExpressionResult,
    Genome, ReferencePattern,
};
pub use hot::{Evicted, TemperatureAgent, TemperatureConfig, TemperatureStats};
pub use warm::{ChronicleAgent, ChronicleConfig, ChronicleStats};
pub use workspace::{
    AgentContext, ConsolidationSummary, MemoryPattern, SearchOptions, Workspace, WorkspaceItem,
    WorkspaceSearchResult, WorkspaceStats,
};
