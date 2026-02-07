//! Agent Memory System - AI Agent Memory Management
//!
//! This module provides human-like memory for AI agents, including:
//! - **Episodic Memory**: Specific events and experiences
//! - **Semantic Memory**: Facts, concepts, and knowledge
//! - **Procedural Memory**: How-to knowledge and skills
//!
//! ## Architecture
//!
//! The agent memory system leverages KoruDelta's existing infrastructure:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              Agent Memory API                           │
//! │   (remember, recall, consolidate, forget)               │
//! ├─────────────────────────────────────────────────────────┤
//! │              Memory Types                               │
//! │   (Episodic, Semantic, Procedural)                      │
//! ├─────────────────────────────────────────────────────────┤
//! │              Vector Search                              │
//! │   (semantic similarity via embeddings)                  │
//! ├─────────────────────────────────────────────────────────┤
//! │              Causal Storage                             │
//! │   (versioned, time-travel capable)                      │
//! ├─────────────────────────────────────────────────────────┤
//! │              Memory Tiers                               │
//! │   (Hot → Warm → Cold → Deep)                            │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Natural Forgetting
//!
//! Memories automatically move through tiers based on:
//! - **Importance**: High importance memories stay accessible longer
//! - **Recency**: Recent memories stay in Hot/Warm
//! - **Access Frequency**: Frequently accessed memories are promoted
//! - **Consolidation**: Old memories are compressed into summaries
//!
//! ## Distinction Integration
//!
//! The system uses `koru-lambda-core` to:
//! - Track causal relationships between memories
//! - Content-address memories for deduplication
//! - Build memory graphs (what led to what)

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

use crate::error::{DeltaError, DeltaResult};
use crate::vector::{Vector, VectorSearchOptions};
use crate::KoruDelta;

/// Types of agent memory.
///
/// Different memory types have different consolidation patterns
/// and retrieval priorities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// Episodic memory: Specific events and experiences.
    ///
    /// Examples:
    /// - "User asked about Python bindings at 2026-02-06T10:00:00Z"
    /// - "Agent processed 50 documents today"
    ///
    /// Consolidation: Old episodes are summarized into "periods"
    Episodic,

    /// Semantic memory: Facts, concepts, and knowledge.
    ///
    /// Examples:
    /// - "KoruDelta has Python bindings via PyO3"
    /// - "Vector similarity uses cosine distance"
    ///
    /// Consolidation: Facts are preserved, updated when contradicted
    Semantic,

    /// Procedural memory: How-to knowledge and skills.
    ///
    /// Examples:
    /// - "To install KoruDelta: pip install koru-delta"
    /// - "When user asks about X, do Y"
    ///
    /// Consolidation: Procedures are refined based on success/failure
    Procedural,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Episodic => write!(f, "episodic"),
            MemoryType::Semantic => write!(f, "semantic"),
            MemoryType::Procedural => write!(f, "procedural"),
        }
    }
}

/// A memory entry in the agent's memory system.
///
/// Memories are content-addressed and versioned, enabling:
/// - Time travel (recall what agent knew at specific time)
/// - Causal tracking (what led to this memory)
/// - Automatic deduplication (identical memories share storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique memory ID (content hash from distinction engine)
    pub id: String,

    /// The memory content
    pub content: String,

    /// Memory type
    pub memory_type: MemoryType,

    /// Vector embedding for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vector>,

    /// Importance score (0.0 - 1.0)
    ///
    /// High importance memories are:
    /// - Less likely to be forgotten
    /// - Prioritized in recall
    /// - Preserved during consolidation
    pub importance: f32,

    /// When the memory was created
    pub created_at: DateTime<Utc>,

    /// When the memory was last accessed
    pub last_accessed: DateTime<Utc>,

    /// Number of times this memory has been recalled
    pub access_count: u32,

    /// Tags for categorization and filtering
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,

    /// Causal context: what led to this memory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_context: Option<String>,

    /// Source of the memory (e.g., "user_input", "agent_thought", "tool_result")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Memory {
    /// Create a new memory entry.
    pub fn new(content: impl Into<String>, memory_type: MemoryType) -> Self {
        let now = Utc::now();
        Self {
            id: String::new(), // Will be set by AgentMemory
            content: content.into(),
            memory_type,
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
            if let Some(ref memory_vec) = self.embedding {
                if let Some(sim) = query_vec.cosine_similarity(memory_vec) {
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

    /// Check if this memory should be consolidated.
    ///
    /// Old, low-importance, rarely accessed memories are candidates.
    pub fn should_consolidate(&self, threshold_days: i64) -> bool {
        let age = Utc::now() - self.created_at;
        if age.num_days() < threshold_days {
            return false;
        }

        // High importance memories are preserved
        if self.importance > 0.8 {
            return false;
        }

        // Frequently accessed memories are preserved
        if self.access_count > 5 {
            return false;
        }

        true
    }
}

/// Result of a memory recall operation.
#[derive(Debug, Clone)]
pub struct MemoryRecall {
    /// The recalled memory
    pub memory: Memory,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
    /// How the memory was found ("exact", "semantic", "association")
    pub match_type: String,
}

/// Options for memory recall.
#[derive(Debug, Clone)]
pub struct RecallOptions {
    /// Maximum number of memories to return
    pub limit: usize,
    /// Filter by memory type
    pub memory_type: Option<MemoryType>,
    /// Filter by tags (all must match)
    pub tags: Vec<String>,
    /// Minimum relevance threshold
    pub min_relevance: f32,
    /// Time range filter
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    /// Whether to include context
    pub include_context: bool,
}

impl RecallOptions {
    /// Create default recall options.
    pub fn new() -> Self {
        Self {
            limit: 10,
            memory_type: None,
            tags: Vec::new(),
            min_relevance: 0.0,
            from_time: None,
            to_time: None,
            include_context: true,
        }
    }

    /// Set the limit.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }

    /// Filter by memory type.
    pub fn memory_type(mut self, t: MemoryType) -> Self {
        self.memory_type = Some(t);
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

    /// Set time range.
    pub fn time_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.from_time = Some(from);
        self.to_time = Some(to);
        self
    }
}

impl Default for RecallOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent memory manager.
///
/// Provides a high-level API for AI agents to store and retrieve memories.
/// Integrates with KoruDelta's storage, vector search, and memory tiers.
pub struct AgentMemory {
    /// Reference to the KoruDelta database
    db: KoruDelta,
    /// Unique agent identifier
    agent_id: String,
    /// In-memory cache of recent memories
    cache: RwLock<HashMap<String, Memory>>,
}

impl AgentMemory {
    /// Create a new agent memory manager.
    pub fn new(db: KoruDelta, agent_id: impl Into<String>) -> Self {
        Self {
            db,
            agent_id: agent_id.into(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get the namespace for this agent's memories.
    fn memory_namespace(&self) -> String {
        format!("agent_memory:{}", self.agent_id)
    }

    /// Get the namespace for this agent's embeddings.
    fn embedding_namespace(&self) -> String {
        format!("agent_embeddings:{}", self.agent_id)
    }

    /// Generate a content-based ID for a memory.
    fn generate_memory_id(&self, content: &str) -> String {
        // Generate content-based ID using SHA256
        use sha2::{Digest, Sha256};
        let content_key = format!("agent:{}/{}", self.agent_id, content);
        let hash = Sha256::digest(content_key.as_bytes());
        hex::encode(hash)
    }

    /// Store an episodic memory (specific event).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let memory = agent.remember_episode(
    ///     "User asked about Python bindings",
    ///     0.8,  // High importance
    /// ).await?;
    /// ```
    pub async fn remember_episode(
        &self,
        content: impl Into<String>,
        importance: f32,
    ) -> DeltaResult<Memory> {
        let memory = Memory::new(content, MemoryType::Episodic)
            .with_importance(importance)
            .with_source("agent_experience");

        self.store_memory(memory).await
    }

    /// Store a semantic memory (fact/knowledge).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let memory = agent.remember_fact(
    ///     "koru_python_bindings",
    ///     "KoruDelta has Python bindings via PyO3",
    ///     vec!["bindings".to_string(), "python".to_string()],
    /// ).await?;
    /// ```
    pub async fn remember_fact(
        &self,
        key: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) -> DeltaResult<Memory> {
        let memory = Memory::new(content, MemoryType::Semantic)
            .with_importance(0.9) // Facts are important
            .with_tags(tags)
            .with_source("knowledge_acquisition");

        self.store_memory_with_key(key, memory).await
    }

    /// Store a procedural memory (how-to).
    pub async fn remember_procedure(
        &self,
        name: impl Into<String>,
        steps: impl Into<String>,
        success_rate: Option<f32>,
    ) -> DeltaResult<Memory> {
        let content = steps.into();
        let mut memory = Memory::new(content, MemoryType::Procedural)
            .with_importance(success_rate.unwrap_or(0.5))
            .with_tags(vec!["procedure".to_string()])
            .with_source("skill_learning");

        // Procedural memories get their name as causal context
        memory.causal_context = Some(name.into());

        self.store_memory(memory).await
    }

    /// Store a memory with auto-generated key.
    async fn store_memory(&self, mut memory: Memory) -> DeltaResult<Memory> {
        let id = self.generate_memory_id(&memory.content);
        memory.id = id.clone();

        // Serialize memory
        let value = serde_json::to_value(&memory)
            .map_err(DeltaError::SerializationError)?;

        // Store in database
        self.db.put(&self.memory_namespace(), &id, value).await?;

        // If memory has embedding, store in vector index
        if let Some(ref embedding) = memory.embedding {
            self.db
                .embed(&self.embedding_namespace(), &id, embedding.clone(), None)
                .await?;
        }

        // Add to cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id, memory.clone());
        }

        debug!(
            agent_id = %self.agent_id,
            memory_id = %memory.id,
            memory_type = %memory.memory_type,
            "Memory stored"
        );

        Ok(memory)
    }

    /// Store a memory with specific key.
    async fn store_memory_with_key(
        &self,
        key: impl Into<String>,
        mut memory: Memory,
    ) -> DeltaResult<Memory> {
        let key = key.into();
        memory.id = key.clone();

        let value = serde_json::to_value(&memory)
            .map_err(DeltaError::SerializationError)?;

        self.db.put(&self.memory_namespace(), &key, value).await?;

        if let Some(ref embedding) = memory.embedding {
            self.db
                .embed(&self.embedding_namespace(), &key, embedding.clone(), None)
                .await?;
        }

        {
            let mut cache = self.cache.write().await;
            cache.insert(key, memory.clone());
        }

        Ok(memory)
    }

    /// Recall memories relevant to a query.
    ///
    /// Searches through all memories and returns the most relevant ones.
    /// Uses semantic search if embeddings are available, otherwise falls back
    /// to keyword matching and metadata filtering.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let memories = agent
    ///     .recall("What did user ask about Python?", RecallOptions::new().limit(5))
    ///     .await?;
    ///
    /// for recall in memories {
    ///     println!("{}", recall.memory.content);
    /// }
    /// ```
    pub async fn recall(
        &self,
        query: impl Into<String>,
        opts: RecallOptions,
    ) -> DeltaResult<Vec<MemoryRecall>> {
        let query_str = query.into();
        trace!(query = %query_str, "Recalling memories");

        // Try semantic search first (if we have embeddings)
        let query_embedding = self.generate_query_embedding(&query_str).await?;
        let mut results = Vec::new();

        if let Some(ref embedding) = query_embedding {
            // Vector search for semantic similarity
            let search_opts = VectorSearchOptions::new()
                .top_k(opts.limit * 2) // Get more for filtering
                .threshold(0.0);

            let search_results = self
                .db
                .embed_search(Some(&self.embedding_namespace()), embedding, search_opts)
                .await?;

            for result in search_results {
                if let Ok(Some(memory)) = self.get_memory(&result.key).await {
                    results.push((memory, result.score, "semantic".to_string()));
                }
            }
        }

        // If no vector results, scan all memories
        if results.is_empty() {
            results = self.scan_memories(&query_str).await?;
        }

        // Apply filters
        let mut filtered: Vec<MemoryRecall> = results
            .into_iter()
            .filter_map(|(memory, _score, match_type)| {
                // Filter by memory type
                if let Some(ref target_type) = opts.memory_type {
                    if memory.memory_type != *target_type {
                        return None;
                    }
                }

                // Filter by tags
                if !opts.tags.is_empty() {
                    let has_all_tags = opts.tags.iter().all(|tag| memory.tags.contains(tag));
                    if !has_all_tags {
                        return None;
                    }
                }

                // Filter by time range
                if let Some(from) = opts.from_time {
                    if memory.created_at < from {
                        return None;
                    }
                }
                if let Some(to) = opts.to_time {
                    if memory.created_at > to {
                        return None;
                    }
                }

                // Calculate final relevance
                let relevance = memory.relevance_score(query_embedding.as_ref());

                // Filter by minimum relevance
                if relevance < opts.min_relevance {
                    return None;
                }

                Some(MemoryRecall {
                    memory,
                    relevance,
                    match_type,
                })
            })
            .collect();

        // Sort by relevance (highest first)
        filtered.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        filtered.truncate(opts.limit);

        // Mark as accessed and update cache
        for recall in &filtered {
            if let Ok(Some(mut memory)) = self.get_memory(&recall.memory.id).await {
                memory.mark_accessed();
                let _ = self.update_memory(&memory).await;
            }
        }

        debug!(
            agent_id = %self.agent_id,
            query = %query_str,
            results = filtered.len(),
            "Recall completed"
        );

        Ok(filtered)
    }

    /// Generate embedding for a query string.
    ///
    /// In production, this would call an embedding API (OpenAI, etc.)
    /// For now, returns None (fallback to keyword search).
    async fn generate_query_embedding(&self, _query: &str) -> DeltaResult<Option<Vector>> {
        // TODO: Integrate with embedding API
        // For now, semantic search only works if memories have embeddings
        Ok(None)
    }

    /// Scan all memories for keyword matches.
    async fn scan_memories(&self, query: &str) -> DeltaResult<Vec<(Memory, f32, String)>> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // List all memory keys
        let keys = self.db.list_keys(&self.memory_namespace()).await;

        for key in keys {
            if let Ok(Some(memory)) = self.get_memory(&key).await {
                // Simple keyword matching
                let content_lower = memory.content.to_lowercase();
                if content_lower.contains(&query_lower) {
                    results.push((memory, 0.5, "keyword".to_string()));
                }
            }
        }

        Ok(results)
    }

    /// Get a specific memory by ID.
    async fn get_memory(&self, id: &str) -> DeltaResult<Option<Memory>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(memory) = cache.get(id) {
                return Ok(Some(memory.clone()));
            }
        }

        // Load from database
        match self.db.get(&self.memory_namespace(), id).await {
            Ok(versioned) => {
                let memory: Memory = serde_json::from_value(versioned.value().clone())
                    .map_err(DeltaError::SerializationError)?;

                // Add to cache
                let mut cache = self.cache.write().await;
                cache.insert(id.to_string(), memory.clone());

                Ok(Some(memory))
            }
            Err(_) => Ok(None),
        }
    }

    /// Update an existing memory.
    async fn update_memory(&self, memory: &Memory) -> DeltaResult<()> {
        let value = serde_json::to_value(memory)
            .map_err(DeltaError::SerializationError)?;

        self.db
            .put(&self.memory_namespace(), &memory.id, value)
            .await?;

        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(memory.id.clone(), memory.clone());

        Ok(())
    }

    /// Consolidate old memories.
    ///
    /// This process:
    /// 1. Finds old, low-importance memories
    /// 2. Groups related memories
    /// 3. Creates summaries
    /// 4. Archives original memories to cold storage
    ///
    /// Should be called periodically (e.g., nightly).
    pub async fn consolidate(&self) -> DeltaResult<ConsolidationSummary> {
        info!(agent_id = %self.agent_id, "Starting memory consolidation");

        let mut summary = ConsolidationSummary::default();

        // Get all memories
        let keys = self.db.list_keys(&self.memory_namespace()).await;
        summary.total_memories = keys.len();

        for key in keys {
            if let Ok(Some(memory)) = self.get_memory(&key).await {
                if memory.should_consolidate(30) {
                    // 30-day threshold
                    summary.consolidated_count += 1;

                    // In full implementation, would:
                    // 1. Group related memories
                    // 2. Generate summary via LLM
                    // 3. Store summary
                    // 4. Archive originals

                    debug!(memory_id = %key, "Memory marked for consolidation");
                }
            }
        }

        info!(
            agent_id = %self.agent_id,
            consolidated = summary.consolidated_count,
            "Consolidation completed"
        );

        Ok(summary)
    }

    /// Get memory statistics.
    pub async fn stats(&self) -> DeltaResult<MemoryStats> {
        let keys = self.db.list_keys(&self.memory_namespace()).await;

        let mut stats = MemoryStats {
            total_memories: keys.len(),
            episodic_count: 0,
            semantic_count: 0,
            procedural_count: 0,
            by_tag: HashMap::new(),
        };

        for key in keys {
            if let Ok(Some(memory)) = self.get_memory(&key).await {
                match memory.memory_type {
                    MemoryType::Episodic => stats.episodic_count += 1,
                    MemoryType::Semantic => stats.semantic_count += 1,
                    MemoryType::Procedural => stats.procedural_count += 1,
                }

                for tag in &memory.tags {
                    *stats.by_tag.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Clear all memories for this agent.
    ///
    /// Warning: This is permanent (though history is preserved).
    pub async fn clear_all(&self) -> DeltaResult<()> {
        warn!(agent_id = %self.agent_id, "Clearing all memories");

        let keys = self.db.list_keys(&self.memory_namespace()).await;

        for key in keys {
            self.db.delete_embed(&self.memory_namespace(), &key).await.ok();
        }

        // Clear cache
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }

        info!(agent_id = %self.agent_id, "All memories cleared");
        Ok(())
    }
}

/// Summary of a consolidation operation.
#[derive(Debug, Clone, Default)]
pub struct ConsolidationSummary {
    /// Total memories scanned
    pub total_memories: usize,
    /// Memories consolidated
    pub consolidated_count: usize,
    /// New summaries created
    pub summaries_created: usize,
    /// Errors encountered
    pub errors: usize,
}

/// Memory statistics.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total number of memories
    pub total_memories: usize,
    /// Episodic memory count
    pub episodic_count: usize,
    /// Semantic memory count
    pub semantic_count: usize,
    /// Procedural memory count
    pub procedural_count: usize,
    /// Count by tag
    pub by_tag: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_memory_creation() {
        let memory = Memory::new("Test content", MemoryType::Episodic)
            .with_importance(0.8)
            .with_tags(vec!["test".to_string()]);

        assert_eq!(memory.content, "Test content");
        assert_eq!(memory.memory_type, MemoryType::Episodic);
        assert!((memory.importance - 0.8).abs() < 1e-6);
        assert_eq!(memory.tags, vec!["test"]);
    }

    #[tokio::test]
    async fn test_memory_relevance_score() {
        let memory = Memory::new("Test", MemoryType::Semantic)
            .with_importance(1.0);

        // Without embedding, relevance is based on importance + recency
        let score = memory.relevance_score(None);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[tokio::test]
    async fn test_memory_mark_accessed() {
        let mut memory = Memory::new("Test", MemoryType::Episodic);
        let initial_access = memory.access_count;

        memory.mark_accessed();

        assert_eq!(memory.access_count, initial_access + 1);
        assert!(memory.last_accessed >= memory.created_at);
    }

    #[tokio::test]
    async fn test_memory_should_consolidate() {
        // Old, low importance memory should consolidate
        let old_memory = Memory {
            id: "test".to_string(),
            content: "Old memory".to_string(),
            memory_type: MemoryType::Episodic,
            embedding: None,
            importance: 0.3,
            created_at: Utc::now() - Duration::days(60),
            last_accessed: Utc::now(),
            access_count: 0,
            tags: vec![],
            causal_context: None,
            source: None,
        };

        assert!(old_memory.should_consolidate(30));

        // High importance memory should not consolidate
        let important_memory = Memory {
            id: "test2".to_string(),
            content: "Important".to_string(),
            memory_type: MemoryType::Semantic,
            embedding: None,
            importance: 0.9,
            created_at: Utc::now() - Duration::days(60),
            last_accessed: Utc::now(),
            access_count: 0,
            tags: vec![],
            causal_context: None,
            source: None,
        };

        assert!(!important_memory.should_consolidate(30));
    }

    #[tokio::test]
    async fn test_recall_options() {
        let opts = RecallOptions::new()
            .limit(5)
            .memory_type(MemoryType::Semantic)
            .min_relevance(0.5);

        assert_eq!(opts.limit, 5);
        assert_eq!(opts.memory_type, Some(MemoryType::Semantic));
        assert!((opts.min_relevance - 0.5).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_agent_memory_new() {
        let db = KoruDelta::start().await.unwrap();
        let agent = AgentMemory::new(db, "agent-1");

        assert_eq!(agent.agent_id, "agent-1");
        assert_eq!(agent.memory_namespace(), "agent_memory:agent-1");
        assert_eq!(agent.embedding_namespace(), "agent_embeddings:agent-1");
    }

    #[tokio::test]
    async fn test_remember_and_recall_episode() {
        let db = KoruDelta::start().await.unwrap();
        let agent = AgentMemory::new(db, "agent-1");

        // Store an episode
        let memory = agent
            .remember_episode("User asked about Python bindings", 0.8)
            .await
            .unwrap();

        assert_eq!(memory.memory_type, MemoryType::Episodic);
        assert!((memory.importance - 0.8).abs() < 1e-6);

        // Recall it
        let recalls = agent
            .recall("Python bindings", RecallOptions::new().limit(5))
            .await
            .unwrap();

        assert!(!recalls.is_empty());
        assert!(recalls[0].memory.content.contains("Python bindings"));
    }

    #[tokio::test]
    async fn test_remember_fact() {
        let db = KoruDelta::start().await.unwrap();
        let agent = AgentMemory::new(db, "agent-1");

        let memory = agent
            .remember_fact(
                "python_bindings",
                "KoruDelta has Python bindings via PyO3",
                vec!["bindings".to_string(), "python".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(memory.memory_type, MemoryType::Semantic);
        assert_eq!(memory.tags, vec!["bindings", "python"]);
    }

    #[tokio::test]
    async fn test_recall_filter_by_type() {
        let db = KoruDelta::start().await.unwrap();
        let agent = AgentMemory::new(db, "agent-1");

        // Store different types
        agent.remember_episode("Episode", 0.5).await.unwrap();
        agent.remember_fact("key", "Fact", vec![]).await.unwrap();

        // Recall only semantic
        let recalls = agent
            .recall("", RecallOptions::new().memory_type(MemoryType::Semantic))
            .await
            .unwrap();

        for recall in &recalls {
            assert_eq!(recall.memory.memory_type, MemoryType::Semantic);
        }
    }

    #[tokio::test]
    async fn test_memory_stats() {
        let db = KoruDelta::start().await.unwrap();
        let agent = AgentMemory::new(db, "agent-1");

        agent.remember_episode("Episode", 0.5).await.unwrap();
        agent.remember_fact("key", "Fact", vec!["tag1".to_string()]).await.unwrap();
        agent.remember_fact("key2", "Fact2", vec!["tag1".to_string()]).await.unwrap();

        let stats = agent.stats().await.unwrap();

        assert!(stats.total_memories >= 3);
        assert!(stats.episodic_count >= 1);
        assert!(stats.semantic_count >= 2);
        assert!(stats.by_tag.get("tag1").copied().unwrap_or(0) >= 2);
    }

    #[tokio::test]
    async fn test_consolidation_summary() {
        let summary = ConsolidationSummary {
            total_memories: 100,
            consolidated_count: 20,
            summaries_created: 5,
            errors: 0,
        };

        assert_eq!(summary.total_memories, 100);
        assert_eq!(summary.consolidated_count, 20);
    }
}
