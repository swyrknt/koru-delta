//! Unified Core v2 - Integrated KoruDelta System
//!
//! This module provides `KoruDeltaCore`, which wires together all v2 components:
//! - CausalStorage (foundation)
//! - Memory tiering (Hot/Warm/Cold/Deep)
//! - Evolutionary processes (Consolidation, Distillation, Genome)
//! - Reconciliation (sync)
//! - Auth (self-sovereign identity)
//!
//! # Example
//!
//! ```rust,ignore
//! use koru_delta::core_v2::{KoruDeltaCore, CoreConfig};
//!
//! let config = CoreConfig::default();
//! let core = KoruDeltaCore::new(config).await?;
//!
//! // Automatic memory tiering, background processes, auth
//! core.put("users", "alice", json!({"name": "Alice"})).await?;
//! let user = core.get("users", "alice").await?;
//! ```

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use tokio::sync::RwLock;

use crate::auth::{AuthConfig, AuthManager, AuthStats};

use crate::memory::{ColdMemory, DeepMemory, HotConfig, HotMemory, WarmMemory};
use crate::processes::ProcessRunner;
use crate::query::{Filter, Query};
use crate::reconciliation::ReconciliationManager;
use crate::storage::CausalStorage;
use crate::types::{FullKey, HistoryEntry, VersionedValue};

/// Configuration for KoruDeltaCore.
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// Memory tier configuration
    pub memory: MemoryConfig,
    /// Process configuration
    pub processes: ProcessConfig,
    /// Auth configuration
    pub auth: AuthConfig,
    /// Reconciliation configuration
    pub reconciliation: ReconciliationConfig,
}

/// Memory tier configuration.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Hot memory capacity
    pub hot_capacity: usize,
    /// Warm memory capacity
    pub warm_capacity: usize,
    /// Number of cold epochs
    pub cold_epochs: usize,
}

/// Process configuration.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Enable background processes
    pub enabled: bool,
    /// Consolidation interval
    pub consolidation_interval: Duration,
    /// Distillation interval
    pub distillation_interval: Duration,
    /// Genome update interval
    pub genome_interval: Duration,
}

/// Reconciliation configuration.
#[derive(Debug, Clone)]
pub struct ReconciliationConfig {
    /// Enable sync
    pub enabled: bool,
    /// Sync interval
    pub sync_interval: Duration,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            memory: MemoryConfig::default(),
            processes: ProcessConfig::default(),
            auth: AuthConfig::default(),
            reconciliation: ReconciliationConfig::default(),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            hot_capacity: 1000,
            warm_capacity: 10000,
            cold_epochs: 7,
        }
    }
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            consolidation_interval: Duration::from_secs(300), // 5 minutes
            distillation_interval: Duration::from_secs(3600), // 1 hour
            genome_interval: Duration::from_secs(86400),      // 24 hours
        }
    }
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Off by default until peers configured
            sync_interval: Duration::from_secs(30),
        }
    }
}

/// Unified KoruDelta Core that integrates all v2 components.
pub struct KoruDeltaCore {
    /// Configuration
    config: CoreConfig,

    /// Layer 2: Storage (foundation)
    storage: Arc<CausalStorage>,

    /// Layer 3: Memory Tiers
    hot: Arc<RwLock<HotMemory>>,
    #[allow(dead_code)]
    warm: Arc<RwLock<WarmMemory>>,
    #[allow(dead_code)]
    cold: Arc<RwLock<ColdMemory>>,
    #[allow(dead_code)]
    deep: Arc<RwLock<DeepMemory>>,

    /// Layer 4: Process Runner
    #[allow(dead_code)]
    process_runner: Option<Arc<ProcessRunner>>,

    /// Layer 5: Reconciliation
    #[allow(dead_code)]
    reconciliation: Arc<RwLock<ReconciliationManager>>,

    /// Layer 6: Auth
    auth: Arc<AuthManager>,

    /// Shutdown signal
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    #[allow(dead_code)]
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl KoruDeltaCore {
    /// Create a new KoruDeltaCore with the given configuration.
    pub async fn new(config: CoreConfig) -> crate::error::DeltaResult<Self> {
        let engine = Arc::new(koru_lambda_core::DistinctionEngine::new());
        let storage = Arc::new(CausalStorage::new(engine));

        // Initialize memory tiers
        let hot = Arc::new(RwLock::new(HotMemory::with_config(HotConfig {
            capacity: config.memory.hot_capacity,
            promote_threshold: 2,
        })));

        let warm = Arc::new(RwLock::new(WarmMemory::new()));
        let cold = Arc::new(RwLock::new(ColdMemory::new()));
        let deep = Arc::new(RwLock::new(DeepMemory::new()));

        // Initialize reconciliation
        let reconciliation = Arc::new(RwLock::new(ReconciliationManager::new()));

        // Initialize auth
        let auth = Arc::new(AuthManager::with_config(storage.clone(), config.auth.clone()));

        // Shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let core = Self {
            config,
            storage,
            hot,
            warm,
            cold,
            deep,
            process_runner: None,
            reconciliation,
            auth,
            shutdown_tx,
            shutdown_rx,
        };

        // Start background processes if enabled
        if core.config.processes.enabled {
            // TODO: Start process runner
        }

        Ok(core)
    }

    /// Store a value with automatic memory tiering.
    pub async fn put(
        &self,
        namespace: &str,
        key: &str,
        value: JsonValue,
    ) -> crate::error::DeltaResult<VersionedValue> {
        // Store in CausalStorage
        let versioned = self.storage.put(namespace, key, value)?;

        // Add to hot memory (fast path)
        let full_key = FullKey::new(namespace, key);
        {
            let hot = self.hot.write().await;
            hot.put(full_key.clone(), versioned.clone());
        }

        // TODO: Notify reconciliation of change

        Ok(versioned)
    }

    /// Get a value with automatic tier promotion.
    pub async fn get(
        &self,
        namespace: &str,
        key: &str,
    ) -> crate::error::DeltaResult<VersionedValue> {
        let full_key = FullKey::new(namespace, key);

        // Try hot memory first (fast path)
        {
            let hot = self.hot.read().await;
            if let Some(value) = hot.get(&full_key) {
                return Ok(value);
            }
        }

        // TODO: Try warm/cold/deep memory with promotion
        // For now, fallback to storage

        // Fallback to storage (source of truth)
        let value = self.storage.get(namespace, key)?;
        
        // Add to hot memory for next access
        {
            let hot = self.hot.write().await;
            hot.put(full_key, value.clone());
        }
        
        Ok(value)
    }

    /// Get the current value for a key (sync version for compatibility).
    pub fn get_sync(
        &self,
        namespace: &str,
        key: &str,
    ) -> crate::error::DeltaResult<VersionedValue> {
        // Note: This doesn't check memory tiers since it needs async
        self.storage.get(namespace, key)
    }

    /// Get value at a specific point in time (time travel).
    pub async fn get_at(
        &self,
        namespace: &str,
        key: &str,
        timestamp: DateTime<Utc>,
    ) -> crate::error::DeltaResult<VersionedValue> {
        // Time travel uses causal graph, bypasses memory tiers
        self.storage.get_at(namespace, key, timestamp)
    }

    /// Get complete history for a key.
    pub async fn history(
        &self,
        namespace: &str,
        key: &str,
    ) -> crate::error::DeltaResult<Vec<HistoryEntry>> {
        self.storage.history(namespace, key)
    }

    /// Query with filter and sort.
    pub async fn query(
        &self,
        namespace: &str,
        query: Query,
    ) -> crate::error::DeltaResult<Vec<(String, VersionedValue)>> {
        // Start with storage scan
        let all = self.storage.scan_collection(namespace);

        // Apply filters
        let filtered: Vec<_> = all
            .into_iter()
            .filter(|(_, v)| Self::matches_filters(&v.value, &query.filters))
            .collect();

        // Apply sort
        // TODO: Implement sorting

        // Apply limit
        let limit = query.limit.unwrap_or(filtered.len());
        Ok(filtered.into_iter().take(limit).collect())
    }

    /// Check if a value matches all filters.
    fn matches_filters(value: &JsonValue, filters: &[Filter]) -> bool {
        if filters.is_empty() {
            return true;
        }
        filters.iter().all(|f| Self::matches_filter(value, f))
    }

    /// Check if a value matches a filter.
    fn matches_filter(value: &JsonValue, filter: &Filter) -> bool {
        match filter {
            Filter::Eq { field, value: expected } => {
                value.get(field).map_or(false, |actual| actual == expected)
            }
            Filter::Gt { field, value: threshold } => {
                if let Some(actual) = value.get(field) {
                    if let (Some(a), Some(t)) = (actual.as_f64(), threshold.as_f64()) {
                        a > t
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Filter::Lt { field, value: threshold } => {
                if let Some(actual) = value.get(field) {
                    if let (Some(a), Some(t)) = (actual.as_f64(), threshold.as_f64()) {
                        a < t
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Filter::And(filters) => filters.iter().all(|f| Self::matches_filter(value, f)),
            Filter::Or(filters) => filters.iter().any(|f| Self::matches_filter(value, f)),
            Filter::Not(filter) => !Self::matches_filter(value, filter),
            _ => true, // Other filters not yet implemented
        }
    }

    /// Check if a key exists.
    pub async fn contains_key(&self, namespace: &str, key: &str) -> bool {
        let full_key = FullKey::new(namespace, key);

        // Check hot first
        {
            let hot = self.hot.read().await;
            if hot.contains_key(&full_key) {
                return true;
            }
        }

        // Fallback to storage
        self.storage.contains_key(namespace, key)
    }

    /// Delete a key (creates tombstone).
    pub async fn delete(
        &self,
        namespace: &str,
        key: &str,
    ) -> crate::error::DeltaResult<Option<VersionedValue>> {
        // Store null as tombstone
        self.put(namespace, key, JsonValue::Null).await.map(Some)
    }

    /// List all keys in a namespace.
    pub async fn list_keys(&self, namespace: &str) -> Vec<String> {
        self.storage.list_keys(namespace)
    }

    /// List all namespaces.
    pub async fn list_namespaces(&self) -> Vec<String> {
        self.storage.list_namespaces()
    }

    /// Get database statistics.
    pub async fn stats(&self) -> CoreStats {
        let storage_stats = crate::core::DatabaseStats {
            key_count: self.storage.key_count(),
            total_versions: self.storage.total_version_count(),
            namespace_count: self.storage.list_namespaces().len(),
        };

        let hot_stats = {
            let hot = self.hot.read().await;
            hot.stats()
        };

        let auth_stats = self.auth.stats();

        CoreStats {
            storage: storage_stats,
            hot_memory: hot_stats,
            auth: auth_stats,
        }
    }

    /// Get access to the auth manager.
    pub fn auth(&self) -> &AuthManager {
        &self.auth
    }

    /// Shutdown the core gracefully.
    pub async fn shutdown(self) -> crate::error::DeltaResult<()> {
        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // TODO: Wait for background processes to finish

        Ok(())
    }
}

/// Statistics for KoruDeltaCore.
#[derive(Debug, Clone)]
pub struct CoreStats {
    /// Storage statistics
    pub storage: crate::core::DatabaseStats,
    /// Hot memory statistics
    pub hot_memory: crate::memory::HotStats,
    /// Auth statistics
    pub auth: AuthStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn create_test_core() -> KoruDeltaCore {
        let config = CoreConfig::default();
        KoruDeltaCore::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_core_creation() {
        let core = create_test_core().await;
        let stats = core.stats().await;
        assert_eq!(stats.storage.key_count, 0);
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let core = create_test_core().await;

        // Put a value
        let value = json!({"name": "Alice", "age": 30});
        core.put("users", "alice", value.clone()).await.unwrap();

        // Get it back
        let retrieved = core.get("users", "alice").await.unwrap();
        assert_eq!(*retrieved.value, value);
    }

    #[tokio::test]
    async fn test_contains_key() {
        let core = create_test_core().await;

        assert!(!core.contains_key("users", "alice").await);

        core.put("users", "alice", json!({"name": "Alice"}))
            .await
            .unwrap();

        assert!(core.contains_key("users", "alice").await);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let core = create_test_core().await;

        core.put("users", "alice", json!({"name": "Alice"}))
            .await
            .unwrap();
        core.put("users", "bob", json!({"name": "Bob"}))
            .await
            .unwrap();

        let keys = core.list_keys("users").await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"alice".to_string()));
        assert!(keys.contains(&"bob".to_string()));
    }

    #[tokio::test]
    async fn test_query_with_filter() {
        let core = create_test_core().await;

        // Insert test data
        core.put("users", "alice", json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();
        core.put("users", "bob", json!({"name": "Bob", "age": 25}))
            .await
            .unwrap();
        core.put("users", "charlie", json!({"name": "Charlie", "age": 35}))
            .await
            .unwrap();

        // Query with filter
        let query = Query::new().filter(Filter::gt("age", json!(25)));
        let results = core.query("users", query).await.unwrap();

        assert_eq!(results.len(), 2);
        // Alice (30) and Charlie (35)
    }

    #[tokio::test]
    async fn test_history() {
        let core = create_test_core().await;

        // Put multiple versions
        core.put("users", "alice", json!({"name": "Alice", "version": 1}))
            .await
            .unwrap();
        core.put("users", "alice", json!({"name": "Alice", "version": 2}))
            .await
            .unwrap();
        core.put("users", "alice", json!({"name": "Alice", "version": 3}))
            .await
            .unwrap();

        let history = core.history("users", "alice").await.unwrap();
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_time_travel() {
        let core = create_test_core().await;

        let _before = Utc::now();

        core.put("users", "alice", json!({"name": "Alice v1"}))
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let mid = Utc::now();

        core.put("users", "alice", json!({"name": "Alice v2"}))
            .await
            .unwrap();

        // Get at midpoint should return v1
        let at_mid = core.get_at("users", "alice", mid).await.unwrap();
        assert_eq!(at_mid.value.get("name").unwrap(), "Alice v1");

        // Get current should return v2
        let current = core.get("users", "alice").await.unwrap();
        assert_eq!(current.value.get("name").unwrap(), "Alice v2");
    }
}
