//! Workspace - General-Purpose Causal Storage Container
//!
//! A Workspace provides isolated, versioned storage with natural memory lifecycle.
//! Think of it as a "Git repository for data" with automatic lifecycle management.
//!
//! ## Use Cases
//!
//! - **AI Agents**: Memory spaces for different agents (episodic, semantic, procedural)
//! - **Audit Logs**: Immutable record of all changes with full provenance
//! - **Scientific Data**: Reproducible experiments with complete history
//! - **Config Management**: Versioned configuration with rollback capability
//! - **Edge Caching**: Smart data lifecycle for resource-constrained environments
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                 Workspace API                           │
//! │     (put, get, history, recall, consolidate)            │
//! ├─────────────────────────────────────────────────────────┤
//! │              Memory Patterns                            │
//! │   (Events, Facts, Procedures - conventions)             │
//! ├─────────────────────────────────────────────────────────┤
//! │              Vector Search                              │
//! │   (semantic similarity for any data)                    │
//! ├─────────────────────────────────────────────────────────┤
//! │              Causal Storage                             │
//! │   (versioned, time-travel capable)                      │
//! ├─────────────────────────────────────────────────────────┤
//! │              Memory Tiers                               │
//! │   (Hot → Warm → Cold → Deep)                            │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Natural Lifecycle
//!
//! Data in a workspace automatically moves through tiers:
//! - **Hot**: Recently accessed, fastest retrieval
//! - **Warm**: Less recent, still readily available
//! - **Cold**: Old data, compressed but queryable
//! - **Deep**: Archived, genomic storage for recovery
//!
//! This lifecycle is use-case agnostic:
//! - Old audit logs compress
//! - Stale configs archive
//! - Sensor data consolidates
//! - Agent memories summarize

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace};

use crate::core::KoruDeltaGeneric;
use crate::error::{DeltaError, DeltaResult};
use crate::runtime::Runtime;
use crate::types::VersionedValue;
use crate::vector::Vector;

/// Memory patterns for organizing workspace data.
///
/// These are CONVENTIONS, not hardcoded types. Use them as guidelines
/// for structuring data in your workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryPattern {
    /// Event log: Things that happened at specific times.
    ///
    /// Examples:
    /// - Audit trail: "Transaction 12345 processed at 10:00"
    /// - Metrics: "CPU usage peaked at 95%"
    /// - Agent: "User asked about Python bindings"
    ///
    /// Lifecycle: Events consolidate into summaries over time.
    Event,

    /// Reference data: Facts, knowledge, configuration.
    ///
    /// Examples:
    /// - Config: "Max connections = 100"
    /// - Knowledge: "KoruDelta has Python bindings"
    /// - Taxonomy: "Category A includes items X, Y, Z"
    ///
    /// Lifecycle: Facts preserved, updated when contradicted.
    Reference,

    /// Computable knowledge: Procedures, rules, workflows.
    ///
    /// Examples:
    /// - Workflow: "When alert fires, notify team then escalate"
    /// - Formula: "Revenue = Price × Quantity"
    /// - Agent skill: "To debug: check logs, then metrics, then traces"
    ///
    /// Lifecycle: Procedures refined based on success/failure.
    Procedure,
}

impl std::fmt::Display for MemoryPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryPattern::Event => write!(f, "event"),
            MemoryPattern::Reference => write!(f, "reference"),
            MemoryPattern::Procedure => write!(f, "procedure"),
        }
    }
}

/// A stored item in a workspace.
///
/// Items are content-addressed and versioned, enabling:
/// - Time travel (retrieve any historical state)
/// - Causal tracking (understand how data evolved)
/// - Automatic deduplication (identical content shares storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceItem {
    /// Unique item ID (content hash)
    pub id: String,

    /// The item content
    pub content: String,

    /// Memory pattern (convention, not enforced)
    pub pattern: MemoryPattern,

    /// Optional vector embedding for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vector>,

    /// Importance score (0.0 - 1.0)
    ///
    /// High importance items:
    /// - Less likely to be forgotten
    /// - Prioritized in search
    /// - Preserved during consolidation
    pub importance: f32,

    /// When the item was created
    pub created_at: DateTime<Utc>,

    /// When the item was last accessed
    pub last_accessed: DateTime<Utc>,

    /// Number of times this item has been retrieved
    pub access_count: u32,

    /// Tags for categorization and filtering
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,

    /// Causal context: what led to this item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_context: Option<String>,

    /// Source of the item (e.g., "user_input", "system_event", "derived")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl WorkspaceItem {
    /// Create a new workspace item.
    pub fn new(content: impl Into<String>, pattern: MemoryPattern) -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), // Will be set by Workspace
            content: content.into(),
            pattern,
            embedding: None,
            importance: 0.5, // Default medium importance
            created_at: now,
            last_accessed: now,
            access_count: 0,
            tags: Vec::new(),
            causal_context: None,
            source: None,
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set the vector embedding.
    pub fn with_embedding(mut self, embedding: Vector) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set causal context.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.causal_context = Some(context.into());
        self
    }

    /// Set source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Calculate relevance score for a query.
    ///
    /// Combines multiple factors:
    /// - Semantic similarity (if embeddings available)
    /// - Importance
    /// - Recency
    /// - Access frequency
    pub fn relevance_score(&self, query_embedding: Option<&Vector>) -> f32 {
        let mut score = 0.0;

        // Semantic similarity (if embeddings available)
        if let Some(query_vec) = query_embedding {
            if let Some(ref item_vec) = self.embedding {
                if let Some(sim) = query_vec.cosine_similarity(item_vec) {
                    // Normalize to 0-1 range (cosine is -1 to 1)
                    score += (sim + 1.0) / 2.0 * 0.4; // 40% weight
                }
            }
        }

        // Importance (0-1)
        score += self.importance * 0.3; // 30% weight

        // Recency (exponential decay)
        let age = Utc::now() - self.created_at;
        let days_old = age.num_days() as f32;
        let recency = (-days_old / 30.0).exp(); // 30-day half-life
        score += recency * 0.2; // 20% weight

        // Access frequency (normalized)
        let access_factor = (self.access_count as f32 / 10.0).min(1.0);
        score += access_factor * 0.1; // 10% weight

        score
    }

    /// Mark as accessed (updates last_accessed and access_count).
    pub fn mark_accessed(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }

    /// Check if this item should be consolidated.
    ///
    /// Old, low-importance, rarely accessed items are candidates.
    pub fn should_consolidate(&self, threshold_days: i64) -> bool {
        let age = Utc::now() - self.created_at;
        if age.num_days() < threshold_days {
            return false;
        }

        // High importance items are preserved
        if self.importance > 0.8 {
            return false;
        }

        // Frequently accessed items are preserved
        if self.access_count > 5 {
            return false;
        }

        true
    }
}

/// Result of a workspace search operation.
#[derive(Debug, Clone)]
pub struct WorkspaceSearchResult {
    /// The matched item
    pub item: WorkspaceItem,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
    /// How the item was found ("exact", "semantic", "association")
    pub match_type: String,
}

/// Options for workspace search.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results to return
    pub limit: usize,
    /// Filter by memory pattern
    pub pattern: Option<MemoryPattern>,
    /// Filter by tags (all must match)
    pub tags: Vec<String>,
    /// Minimum relevance threshold
    pub min_relevance: f32,
    /// Time range filter
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
}

impl SearchOptions {
    /// Create default search options.
    pub fn new() -> Self {
        Self {
            limit: 10,
            pattern: None,
            tags: Vec::new(),
            min_relevance: 0.0,
            from_time: None,
            to_time: None,
        }
    }

    /// Set the limit.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }

    /// Filter by memory pattern.
    pub fn pattern(mut self, p: MemoryPattern) -> Self {
        self.pattern = Some(p);
        self
    }

    /// Filter by tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set minimum relevance.
    pub fn min_relevance(mut self, threshold: f32) -> Self {
        self.min_relevance = threshold;
        self
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// A workspace - isolated causal storage with natural lifecycle.
///
/// Workspaces provide:
/// - Isolation: Data in one workspace doesn't affect others
/// - Versioning: Complete history of all changes
/// - Search: Keyword and semantic search
/// - Lifecycle: Automatic tiering and consolidation
///
/// ## Examples
///
/// **AI Agent Memory:**
/// ```ignore
/// let workspace = db.workspace("agent-42");
/// workspace.store("episode:001", "User asked about Python", MemoryPattern::Event).await?;
/// let memories = workspace.recall("Python", SearchOptions::new().limit(5)).await?;
/// ```
///
/// **Audit Trail:**
/// ```ignore
/// let audit = db.workspace("audit-2026");
/// audit.store("tx:12345", transaction_data, MemoryPattern::Event).await?;
/// let history = audit.history("tx:12345").await?;
/// ```
///
/// **Configuration:**
/// ```ignore
/// let config = db.workspace("app-config");
/// config.store("api.timeout", "30s", MemoryPattern::Reference).await?;
/// let timeout = config.get_at("api.timeout", yesterday).await?; // Time travel
/// ```
pub struct Workspace<R: Runtime> {
    db: KoruDeltaGeneric<R>,
    name: String,
}

impl<R: Runtime> Workspace<R> {
    /// Create a new workspace.
    pub fn new(db: KoruDeltaGeneric<R>, name: impl Into<String>) -> Self {
        Self {
            db,
            name: name.into(),
        }
    }

    /// Get the workspace name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Store an item in the workspace.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique identifier for this item
    /// * `content` - The item content (any Serialize type)
    /// * `pattern` - Memory pattern (convention, not enforced)
    ///
    /// # Example
    ///
    /// ```ignore
    /// workspace.store("config:timeout", "30s", MemoryPattern::Reference).await?;
    /// ```
    pub async fn store(
        &self,
        key: impl Into<String>,
        content: impl Serialize,
        pattern: MemoryPattern,
    ) -> DeltaResult<VersionedValue> {
        let key = key.into();

        // Serialize content
        let value = serde_json::to_value(content).map_err(DeltaError::SerializationError)?;

        // Store in database
        let versioned = self.db.put(&self.name, &key, value).await?;

        debug!(workspace = %self.name, key = %key, pattern = %pattern, "Item stored");
        Ok(versioned)
    }

    /// Retrieve an item by key.
    ///
    /// Returns the current value. For historical values, use `history()` or `get_at()`.
    pub async fn get(&self, key: impl Into<String>) -> DeltaResult<serde_json::Value> {
        let key = key.into();
        let versioned = self.db.get(&self.name, &key).await?;
        Ok(versioned.value().clone())
    }

    /// Get item at a specific point in time.
    ///
    /// Enables time-travel queries - see what the value was at any historical timestamp.
    pub async fn get_at(
        &self,
        key: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> DeltaResult<serde_json::Value> {
        let key = key.into();
        let versioned = self.db.get_at(&self.name, &key, timestamp).await?;
        Ok(versioned.value().clone())
    }

    /// Get complete history for an item.
    ///
    /// Returns all versions from oldest to newest.
    pub async fn history(
        &self,
        key: impl Into<String>,
    ) -> DeltaResult<Vec<crate::types::HistoryEntry>> {
        let key = key.into();
        self.db.history(&self.name, &key).await
    }

    /// Search for items in the workspace.
    ///
    /// Performs keyword search (and semantic search if embeddings available).
    pub async fn search(
        &self,
        query: impl Into<String>,
        opts: SearchOptions,
    ) -> DeltaResult<Vec<WorkspaceSearchResult>> {
        let query_str = query.into();
        trace!(query = %query_str, "Searching workspace");

        // For now, simple keyword search on keys
        // Full implementation would scan and rank items
        let keys = self.db.list_keys(&self.name).await;
        let mut results = Vec::new();

        let query_lower = query_str.to_lowercase();

        for key in keys {
            if let Ok(value) = self.db.get(&self.name, &key).await {
                let content = value.value().to_string().to_lowercase();
                if content.contains(&query_lower) || key.to_lowercase().contains(&query_lower) {
                    // Create a synthetic WorkspaceItem for the result
                    let item = WorkspaceItem {
                        id: key.clone(),
                        content: value.value().to_string(),
                        pattern: MemoryPattern::Event, // Default, would be stored in real impl
                        embedding: None,
                        importance: 0.5,
                        created_at: value.timestamp(),
                        last_accessed: Utc::now(),
                        access_count: 0,
                        tags: vec![],
                        causal_context: None,
                        source: None,
                    };

                    results.push(WorkspaceSearchResult {
                        item,
                        relevance: 0.5, // Would be calculated properly
                        match_type: "keyword".to_string(),
                    });
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        results.truncate(opts.limit);

        debug!(workspace = %self.name, results = results.len(), "Search completed");
        Ok(results)
    }

    /// List all keys in the workspace.
    pub async fn list_keys(&self) -> Vec<String> {
        self.db.list_keys(&self.name).await
    }

    /// Check if a key exists.
    pub async fn contains(&self, key: impl Into<String>) -> bool {
        let key = key.into();
        self.db.contains(&self.name, &key).await
    }

    /// Delete an item (stores tombstone, history preserved).
    pub async fn delete(&self, key: impl Into<String>) -> DeltaResult<VersionedValue> {
        let key = key.into();
        self.db.delete_embed(&self.name, &key).await
    }

    /// Consolidate old items.
    ///
    /// Compresses old, low-importance items to save space.
    /// Should be called periodically (e.g., nightly).
    pub async fn consolidate(&self) -> ConsolidationSummary {
        info!(workspace = %self.name, "Starting consolidation");

        let keys = self.db.list_keys(&self.name).await;
        let total = keys.len();

        // Placeholder: In full implementation, would:
        // 1. Find old items
        // 2. Group related items
        // 3. Create summaries
        // 4. Archive originals

        ConsolidationSummary {
            total_items: total,
            consolidated_count: 0,
            summaries_created: 0,
            errors: 0,
        }
    }

    /// Get workspace statistics.
    pub async fn stats(&self) -> WorkspaceStats {
        let keys = self.db.list_keys(&self.name).await;

        WorkspaceStats {
            total_items: keys.len(),
            workspace_name: self.name.clone(),
        }
    }
}

impl<R: Runtime> Clone for Workspace<R> {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            name: self.name.clone(),
        }
    }
}

/// Summary of a consolidation operation.
#[derive(Debug, Clone, Default)]
pub struct ConsolidationSummary {
    /// Total items scanned
    pub total_items: usize,
    /// Items consolidated
    pub consolidated_count: usize,
    /// New summaries created
    pub summaries_created: usize,
    /// Errors encountered
    pub errors: usize,
}

/// Workspace statistics.
#[derive(Debug, Clone)]
pub struct WorkspaceStats {
    /// Total number of items
    pub total_items: usize,
    /// Workspace name
    pub workspace_name: String,
}

// ============================================================================
// AI Agent Context (Thin wrapper for backward compatibility)
// ============================================================================

/// AI Agent memory context.
///
/// A thin wrapper around Workspace that provides AI-specific convenience methods.
/// This is optional - agents can use Workspace directly if preferred.
///
/// # Example
///
/// ```ignore
/// let agent = db.workspace("agent-42").ai_context();
/// agent.remember_episode("User asked about Python").await?;
/// agent.remember_fact("python_bindings", "KoruDelta has Python bindings").await?;
/// ```
pub struct AgentContext<R: Runtime> {
    workspace: Workspace<R>,
}

impl<R: Runtime> AgentContext<R> {
    /// Create an AI agent context from a workspace.
    pub fn new(workspace: Workspace<R>) -> Self {
        Self { workspace }
    }

    /// Remember an episodic memory (specific event).
    pub async fn remember_episode(
        &self,
        content: impl Into<String>,
        importance: f32,
    ) -> DeltaResult<VersionedValue> {
        let content = content.into();
        let key = format!("episode:{}", generate_id(&content));

        self.workspace
            .store(
                &key,
                serde_json::json!({
                    "type": "episodic",
                    "content": content,
                    "importance": importance,
                }),
                MemoryPattern::Event,
            )
            .await
    }

    /// Remember a fact (semantic memory).
    pub async fn remember_fact(
        &self,
        key: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) -> DeltaResult<VersionedValue> {
        let key = format!("fact:{}", key.into());
        let content = content.into();

        self.workspace
            .store(
                &key,
                serde_json::json!({
                    "type": "semantic",
                    "content": content,
                    "tags": tags,
                }),
                MemoryPattern::Reference,
            )
            .await
    }

    /// Remember a procedure (procedural memory).
    pub async fn remember_procedure(
        &self,
        name: impl Into<String>,
        steps: impl Into<String>,
        success_rate: Option<f32>,
    ) -> DeltaResult<VersionedValue> {
        let name = name.into();
        let key = format!("procedure:{}", name);
        let steps = steps.into();

        self.workspace
            .store(
                &key,
                serde_json::json!({
                    "type": "procedural",
                    "name": name,
                    "steps": steps,
                    "success_rate": success_rate.unwrap_or(0.5),
                }),
                MemoryPattern::Procedure,
            )
            .await
    }

    /// Recall relevant memories.
    pub async fn recall(
        &self,
        query: impl Into<String>,
        limit: usize,
    ) -> DeltaResult<Vec<WorkspaceSearchResult>> {
        self.workspace
            .search(query, SearchOptions::new().limit(limit))
            .await
    }
}

/// Generate a short ID from content.
fn generate_id(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(content.as_bytes());
    hex::encode(&hash[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workspace_item_creation() {
        let item = WorkspaceItem::new("Test content", MemoryPattern::Event)
            .with_importance(0.8)
            .with_tags(vec!["test".to_string()]);

        assert_eq!(item.content, "Test content");
        assert_eq!(item.pattern, MemoryPattern::Event);
        assert!((item.importance - 0.8).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_workspace_item_should_consolidate() {
        let old_item = WorkspaceItem {
            id: "test".to_string(),
            content: "Old item".to_string(),
            pattern: MemoryPattern::Event,
            embedding: None,
            importance: 0.3,
            created_at: Utc::now() - chrono::Duration::days(60),
            last_accessed: Utc::now(),
            access_count: 0,
            tags: vec![],
            causal_context: None,
            source: None,
        };

        assert!(old_item.should_consolidate(30));
    }

    #[tokio::test]
    async fn test_search_options() {
        let opts = SearchOptions::new()
            .limit(5)
            .pattern(MemoryPattern::Reference)
            .min_relevance(0.7);

        assert_eq!(opts.limit, 5);
        assert_eq!(opts.pattern, Some(MemoryPattern::Reference));
        assert!((opts.min_relevance - 0.7).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_memory_pattern_display() {
        assert_eq!(format!("{}", MemoryPattern::Event), "event");
        assert_eq!(format!("{}", MemoryPattern::Reference), "reference");
        assert_eq!(format!("{}", MemoryPattern::Procedure), "procedure");
    }
}
