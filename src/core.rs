//! Unified KoruDelta Core - Complete Implementation
//!
//! This module provides the main KoruDelta database instance, integrating:
//! - CausalStorage (versioned key-value storage)
//! - Memory tiering (Hot/Warm/Cold/Deep)
//! - Auth (self-sovereign identity)
//! - Reconciliation (sync)
//! - Views (materialized queries)
//! - Subscriptions (change notifications)
//!
//! # Design Philosophy
//!
//! - **Simple API**: put, get, history, get_at, query - clean and minimal
//! - **Complete functionality**: All filters, sorting, aggregation work correctly
//! - **Async-ready**: Future-proof for distributed operations
//! - **Thread-safe**: Share KoruDelta instances across threads safely
//!
//! # Example
//!
//! ```ignore
//! use koru_delta::KoruDelta;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db = KoruDelta::start().await?;
//!
//!     // Store data
//!     db.put("users", "alice", json!({"name": "Alice"})).await?;
//!
//!     // Retrieve data
//!     let user = db.get("users", "alice").await?;
//!
//!     // Query with filters
//!     let results = db.query("users", Query::new().filter(Filter::eq("active", true))).await?;
//!
//!     // View history
//!     let history = db.history("users", "alice").await?;
//!
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
#[cfg(not(target_arch = "wasm32"))]
use futures::FutureExt;
use koru_lambda_core::DistinctionEngine;
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use tracing::{debug, error, info, trace, warn};
#[cfg(target_arch = "wasm32")]
use tracing::{debug, info, trace};

use crate::auth::{AuthConfig, AuthManager};
use crate::error::DeltaResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::lifecycle::{LifecycleConfig, LifecycleManager};
use crate::memory::{ColdMemory, DeepMemory, HotConfig, HotMemory, WarmMemory};
use crate::processes::ProcessRunner;
use crate::query::{HistoryQuery, Query, QueryExecutor, QueryResult};
use crate::reconciliation::ReconciliationManager;
use crate::runtime::sync::RwLock;
use crate::runtime::{DefaultRuntime, Runtime, WatchReceiver, WatchSender};
use crate::storage::CausalStorage;
#[cfg(not(target_arch = "wasm32"))]
use crate::subscriptions::{ChangeEvent, Subscription, SubscriptionId, SubscriptionManager};
use crate::types::{FullKey, HistoryEntry, VersionedValue};
use crate::vector::{Vector, VectorIndex, VectorSearchOptions, VectorSearchResult};
use crate::views::{ViewDefinition, ViewInfo, ViewManager};

#[cfg(not(target_arch = "wasm32"))]
use crate::cluster::ClusterNode;

/// Configuration for KoruDelta.
#[derive(Debug, Clone, Default)]
pub struct CoreConfig {
    /// Memory tier configuration
    pub memory: MemoryConfig,
    /// Process configuration
    pub processes: ProcessConfig,
    /// Auth configuration
    pub auth: AuthConfig,
    /// Reconciliation configuration
    pub reconciliation: ReconciliationConfig,
    /// Resource limits (memory, disk)
    pub limits: ResourceLimits,
}

/// Resource limits for the database.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB (0 = unlimited)
    pub max_memory_mb: usize,
    /// Maximum disk usage in MB (0 = unlimited)
    pub max_disk_mb: usize,
    /// Maximum open files (0 = unlimited)
    pub max_open_files: usize,
    /// Maximum concurrent connections (0 = unlimited)
    pub max_connections: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,     // 512MB default
            max_disk_mb: 10 * 1024, // 10GB default
            max_open_files: 256,
            max_connections: 100,
        }
    }
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
            consolidation_interval: Duration::from_secs(300),
            distillation_interval: Duration::from_secs(3600),
            genome_interval: Duration::from_secs(86400),
        }
    }
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sync_interval: Duration::from_secs(30),
        }
    }
}

/// The main KoruDelta database instance.
///
/// KoruDelta is the invisible database that gives you:
/// - Git-like history (every change is versioned)
/// - Redis-like simplicity (minimal API, zero configuration)
/// - Mathematical guarantees (built on distinction calculus)
///
/// # Thread Safety
///
/// KoruDelta is fully thread-safe and can be cloned cheaply to share
/// across threads (uses Arc internally).
///
/// # Runtime
///
/// KoruDelta is generic over the async runtime. Use `KoruDelta` for the
/// default runtime (Tokio on native, WASM in browsers), or `KoruDeltaGeneric<R>`
/// for a custom runtime.
#[derive(Clone)]
pub struct KoruDeltaGeneric<R: Runtime> {
    /// Async runtime for spawning tasks
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    runtime: R,
    /// Configuration
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    config: CoreConfig,
    /// Database path for persistence (None = in-memory only)
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    db_path: Option<PathBuf>,
    /// The underlying storage engine
    storage: Arc<CausalStorage>,
    /// The distinction engine (for advanced operations)
    engine: Arc<DistinctionEngine>,
    /// View manager for materialized views
    views: Arc<ViewManager>,
    /// Subscription manager for change notifications (non-WASM only)
    #[cfg(not(target_arch = "wasm32"))]
    subscriptions: Arc<SubscriptionManager>,
    /// Memory tiers
    hot: Arc<RwLock<HotMemory>>,
    warm: Arc<RwLock<WarmMemory>>,
    cold: Arc<RwLock<ColdMemory>>,
    deep: Arc<RwLock<DeepMemory>>,
    /// Process runner for background tasks (Phase 7)
    #[allow(dead_code)]
    process_runner: Option<Arc<ProcessRunner>>,
    /// Reconciliation manager for distributed sync (Phase 7/8)
    #[allow(dead_code)]
    reconciliation: Arc<RwLock<ReconciliationManager>>,
    /// Auth manager
    auth: Arc<AuthManager>,
    /// Lifecycle manager for memory consolidation (non-WASM only)
    #[cfg(not(target_arch = "wasm32"))]
    lifecycle: Arc<LifecycleManager>,
    /// Vector index for similarity search
    vector_index: VectorIndex,
    /// Cluster node for distributed operation (optional)
    #[cfg(not(target_arch = "wasm32"))]
    cluster: Option<Arc<ClusterNode>>,
    /// Shutdown signal
    shutdown_tx: WatchSender<bool>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    shutdown_rx: WatchReceiver<bool>,
}

/// Type alias for KoruDelta with the default runtime.
///
/// On native platforms: uses TokioRuntime
/// On WASM: uses WasmRuntime
pub type KoruDelta = KoruDeltaGeneric<DefaultRuntime>;

impl<R: Runtime> std::fmt::Debug for KoruDeltaGeneric<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KoruDelta")
            .field("storage", &self.storage)
            .field("engine", &self.engine)
            .finish()
    }
}

impl<R: Runtime> KoruDeltaGeneric<R> {
    /// Start a new KoruDelta instance with default configuration.
    ///
    /// This is the zero-configuration entry point (in-memory only).
    pub async fn start() -> DeltaResult<Self> {
        info!("Starting KoruDelta in-memory instance");
        let runtime = R::new();
        let db = Self::new_with_runtime(CoreConfig::default(), runtime).await?;
        info!("KoruDelta in-memory instance started");
        Ok(db)
    }

    /// Start a new KoruDelta instance with persistence at the given path.
    ///
    /// If the path exists and contains a database, it will be loaded.
    /// If the path doesn't exist, a new database will be created.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let db = KoruDelta::start_with_path(Path::new("~/.korudelta/db")).await?;
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn start_with_path(path: impl Into<PathBuf>) -> DeltaResult<Self> {
        use crate::persistence;

        let path = path.into();
        let path_display = path.display().to_string();
        info!(db_path = %path_display, "Starting KoruDelta with persistence");

        let config = CoreConfig::default();
        let runtime = R::new();

        // Acquire lock and check for unclean shutdown
        let lock_state = persistence::acquire_lock(&path).await?;
        if lock_state == persistence::LockState::Unclean {
            warn!("Unclean shutdown detected, running recovery");
        } else {
            debug!("Lock acquired successfully");
        }

        // Load from WAL if exists
        let storage = if persistence::exists(&path).await {
            info!("Loading existing database from WAL");
            let engine = Arc::new(DistinctionEngine::new());
            let storage = persistence::load_from_wal(&path, engine).await?;
            let key_count = storage.key_count();
            info!(keys = key_count, "Database loaded from WAL");
            storage
        } else {
            info!("Creating new database");
            CausalStorage::new(Arc::new(DistinctionEngine::new()))
        };

        let storage = Arc::new(storage);
        let engine = storage.engine();

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
        let auth = Arc::new(AuthManager::with_config(
            Arc::clone(&storage),
            config.auth.clone(),
        ));

        // Initialize views
        let views = Arc::new(ViewManager::new(Arc::clone(&storage)));

        // Initialize subscriptions (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let subscriptions = Arc::new(SubscriptionManager::new());

        // Initialize lifecycle manager (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let lifecycle = Arc::new(LifecycleManager::new(LifecycleConfig::default()));

        // Shutdown channel using runtime
        let (shutdown_tx, shutdown_rx) = runtime.watch_channel(false);

        let db = Self {
            runtime,
            config,
            db_path: Some(path),
            storage,
            engine,
            hot,
            warm,
            cold,
            deep,
            process_runner: None,
            reconciliation,
            auth,
            #[cfg(not(target_arch = "wasm32"))]
            lifecycle,
            views,
            #[cfg(not(target_arch = "wasm32"))]
            subscriptions,
            vector_index: VectorIndex::new_flat(),
            #[cfg(not(target_arch = "wasm32"))]
            cluster: None,
            shutdown_tx,
            shutdown_rx,
        };

        // Start background processes if enabled (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        if db.config.processes.enabled {
            db.start_background_processes().await;
        }

        Ok(db)
    }

    /// Create a new KoruDelta with the given configuration.
    pub async fn new(config: CoreConfig) -> DeltaResult<Self> {
        let runtime = R::new();
        Self::new_with_runtime(config, runtime).await
    }

    /// Create a new KoruDelta with the given configuration and runtime.
    pub async fn new_with_runtime(config: CoreConfig, runtime: R) -> DeltaResult<Self> {
        let engine = Arc::new(DistinctionEngine::new());
        let storage = Arc::new(CausalStorage::new(Arc::clone(&engine)));

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
        let auth = Arc::new(AuthManager::with_config(
            Arc::clone(&storage),
            config.auth.clone(),
        ));

        // Initialize views
        let views = Arc::new(ViewManager::new(Arc::clone(&storage)));

        // Initialize subscriptions (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let subscriptions = Arc::new(SubscriptionManager::new());

        // Initialize lifecycle manager (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let lifecycle = Arc::new(LifecycleManager::new(LifecycleConfig::default()));

        // Shutdown channel using runtime
        let (shutdown_tx, shutdown_rx) = runtime.watch_channel(false);

        let db = Self {
            runtime,
            config,
            db_path: None,
            storage,
            engine,
            hot,
            warm,
            cold,
            deep,
            process_runner: None,
            reconciliation,
            auth,
            #[cfg(not(target_arch = "wasm32"))]
            lifecycle,
            views,
            #[cfg(not(target_arch = "wasm32"))]
            subscriptions,
            vector_index: VectorIndex::new_flat(),
            #[cfg(not(target_arch = "wasm32"))]
            cluster: None,
            shutdown_tx,
            shutdown_rx,
        };

        // Start background processes if enabled (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        if db.config.processes.enabled {
            db.start_background_processes().await;
        }

        Ok(db)
    }

    /// Attach a cluster node for distributed operation.
    ///
    /// This enables automatic broadcast of writes to cluster peers.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_cluster(mut self, cluster: Arc<ClusterNode>) -> Self {
        self.cluster = Some(cluster);
        self
    }

    /// Start background processes (consolidation, distillation, genome update).
    #[cfg(not(target_arch = "wasm32"))]
    async fn start_background_processes(&self) {
        let hot = Arc::clone(&self.hot);
        let warm = Arc::clone(&self.warm);
        let cold = Arc::clone(&self.cold);
        let deep = Arc::clone(&self.deep);
        let storage = Arc::clone(&self.storage);
        let mut shutdown = self.shutdown_rx.clone();
        let runtime = self.runtime.clone();

        let consolidation_interval = self.config.processes.consolidation_interval;
        let distillation_interval = self.config.processes.distillation_interval;
        let genome_interval = self.config.processes.genome_interval;

        // Spawn consolidation task
        let runtime_clone = runtime.clone();
        runtime.spawn(async move {
            let mut interval = runtime_clone.interval(consolidation_interval);
            loop {
                futures::select! {
                    _ = interval.tick().fuse() => {
                        // Consolidation: Move data between tiers
                        Self::run_consolidation(
                            &hot, &warm, &cold, &deep, &storage
                        ).await;
                    }
                    _ = Self::watch_shutdown(&mut shutdown).fuse() => {
                        break;
                    }
                }
            }
        });

        // Spawn distillation task
        let hot = Arc::clone(&self.hot);
        let warm = Arc::clone(&self.warm);
        let cold = Arc::clone(&self.cold);
        let storage = Arc::clone(&self.storage);
        let mut shutdown = self.shutdown_rx.clone();
        let runtime_clone = runtime.clone();

        runtime.spawn(async move {
            let mut interval = runtime_clone.interval(distillation_interval);
            loop {
                futures::select! {
                    _ = interval.tick().fuse() => {
                        // Distillation: Remove noise, keep essence
                        Self::run_distillation(
                            &hot, &warm, &cold, &storage
                        ).await;
                    }
                    _ = Self::watch_shutdown(&mut shutdown).fuse() => {
                        break;
                    }
                }
            }
        });

        // Spawn genome update task
        let deep = Arc::clone(&self.deep);
        let mut shutdown = self.shutdown_rx.clone();
        let runtime_clone = runtime.clone();

        runtime.spawn(async move {
            let mut interval = runtime_clone.interval(genome_interval);
            loop {
                futures::select! {
                    _ = interval.tick().fuse() => {
                        // Genome update: Extract causal topology
                        Self::run_genome_update(&deep).await;
                    }
                    _ = Self::watch_shutdown(&mut shutdown).fuse() => {
                        break;
                    }
                }
            }
        });
    }

    /// Helper to watch for shutdown signal.
    #[cfg(not(target_arch = "wasm32"))]
    async fn watch_shutdown(shutdown: &mut WatchReceiver<bool>) {
        loop {
            if let Ok(()) = shutdown.changed().await {
                if shutdown.borrow_and_update() {
                    return;
                }
            } else {
                return;
            }
        }
    }

    /// Run consolidation: Move data between memory tiers.
    ///
    /// This is the "heartbeat" of the memory system - continuously
    /// moves data based on temperature (access patterns).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    async fn run_consolidation(
        hot: &Arc<RwLock<HotMemory>>,
        warm: &Arc<RwLock<WarmMemory>>,
        cold: &Arc<RwLock<ColdMemory>>,
        _deep: &Arc<RwLock<DeepMemory>>,
        _storage: &Arc<CausalStorage>,
    ) {
        // Check HotMemory utilization
        let hot_util = {
            let hot = hot.read().await;
            let stats = hot.stats();
            if stats.capacity > 0 {
                stats.current_size as f64 / stats.capacity as f64
            } else {
                0.0
            }
        };

        // If Hot is over 80% full, natural eviction handles it
        // via LRU on next put()
        if hot_util > 0.8 {
            // Hot memory is getting full - natural eviction will handle it
        }

        // Check WarmMemory utilization and find demotion candidates
        let demotion_candidates = {
            let warm = warm.read().await;
            warm.find_demotion_candidates(10)
        };

        // Demote low-access items from warm to cold
        if !demotion_candidates.is_empty() {
            let warm = warm.write().await;
            let cold = cold.write().await;

            for id in demotion_candidates {
                warm.demote(&id);
                // In full implementation, would move to cold
                cold.consolidate_distinction(&id);
            }
        }

        // Rotate ColdMemory epochs periodically
        {
            let cold = cold.write().await;
            // Rotate if current epoch is getting large
            cold.rotate_epoch();
        }
    }

    /// Run distillation: Remove low-fitness distinctions.
    ///
    /// Natural selection for data - high-fitness distinctions survive,
    /// low-fitness (noise) gets marked for garbage collection.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    async fn run_distillation(
        _hot: &Arc<RwLock<HotMemory>>,
        warm: &Arc<RwLock<WarmMemory>>,
        cold: &Arc<RwLock<ColdMemory>>,
        _storage: &Arc<CausalStorage>,
    ) {
        // Find promotion candidates (high fitness) in warm
        let promotion_candidates = {
            let warm = warm.read().await;
            warm.find_promotion_candidates(10)
        };

        // Promote high-fitness items (mark for hot consideration)
        if !promotion_candidates.is_empty() {
            let warm = warm.write().await;
            for (_, id) in promotion_candidates {
                warm.promote(&id);
            }
        }

        // Compress cold memory epochs
        {
            let cold = cold.write().await;
            cold.compress_old_epochs();
        }
    }

    /// Run genome update: Extract causal topology for backup.
    ///
    /// Creates a minimal "DNA" representation of the causal graph
    /// that can regenerate the full system state.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    async fn run_genome_update(deep: &Arc<RwLock<DeepMemory>>) {
        // Extract genome using the genome update process
        // This captures the causal topology (structure, not content)
        let genome = crate::processes::GenomeUpdateProcess::new().extract_genome();

        // Store in deep memory
        let deep = deep.write().await;
        deep.store_genome("latest", genome.clone());

        // Also store timestamped version for history
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        deep.store_genome(&format!("genome_{}", timestamp), genome);
    }

    /// Create a KoruDelta from existing storage and engine.
    ///
    /// This is primarily used for persistence testing and recovery scenarios.
    pub fn from_storage(storage: Arc<CausalStorage>, engine: Arc<DistinctionEngine>) -> Self {
        let runtime = R::new();
        Self::from_storage_with_runtime(storage, engine, runtime)
    }

    /// Create a KoruDelta from existing storage, engine, and runtime.
    ///
    /// This is primarily used for persistence testing and recovery scenarios
    /// where a specific runtime is needed.
    pub fn from_storage_with_runtime(
        storage: Arc<CausalStorage>,
        engine: Arc<DistinctionEngine>,
        runtime: R,
    ) -> Self {
        let config = CoreConfig::default();

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
        let auth = Arc::new(AuthManager::with_config(
            Arc::clone(&storage),
            config.auth.clone(),
        ));

        // Initialize views
        let views = Arc::new(ViewManager::new(Arc::clone(&storage)));

        // Initialize subscriptions (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let subscriptions = Arc::new(SubscriptionManager::new());

        // Initialize lifecycle manager (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let lifecycle = Arc::new(LifecycleManager::new(LifecycleConfig::default()));

        // Shutdown channel using runtime
        let (shutdown_tx, shutdown_rx) = runtime.watch_channel(false);

        Self {
            runtime,
            config: CoreConfig::default(),
            db_path: None,
            storage,
            engine,
            hot,
            warm,
            cold,
            deep,
            process_runner: None,
            reconciliation,
            auth,
            #[cfg(not(target_arch = "wasm32"))]
            lifecycle,
            views,
            #[cfg(not(target_arch = "wasm32"))]
            subscriptions,
            vector_index: VectorIndex::new_flat(),
            #[cfg(not(target_arch = "wasm32"))]
            cluster: None,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Store a value with automatic memory tiering.
    pub async fn put<T: Serialize>(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: T,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();
        trace!("Serializing value");
        let json_value = serde_json::to_value(value)?;

        // Store in storage (source of truth)
        trace!("Storing in CausalStorage");
        let versioned = self.storage.put(&namespace, &key, json_value)?;
        let version_id = versioned.version_id().to_string();
        debug!(version = %version_id, "Value stored");

        // Persist to WAL if db_path is set
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref db_path) = self.db_path {
            use crate::persistence;
            trace!("Persisting to WAL");
            if let Err(e) = persistence::append_write(db_path, &namespace, &key, &versioned).await {
                error!(error = %e, "Failed to persist write to WAL");
            } else {
                trace!("Write persisted to WAL");
            }
        }

        // Broadcast to cluster if configured
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref cluster) = self.cluster {
            let full_key = FullKey::new(&namespace, &key);
            let value_clone = versioned.clone();
            let cluster_clone = Arc::clone(cluster);
            tokio::spawn(async move {
                trace!("Broadcasting write to cluster");
                cluster_clone.broadcast_write(full_key, value_clone).await;
            });
        }

        // Promote to hot memory
        {
            let full_key = FullKey::new(&namespace, &key);
            let hot = self.hot.write().await;
            hot.put(full_key, versioned.clone());
            trace!("Value promoted to hot memory");
        }

        // Auto-refresh views (fire and forget, non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let views = Arc::clone(&self.views);
            tokio::spawn(async move {
                let _ = views.refresh_stale(chrono::Duration::seconds(0));
            });
        }

        info!(version = %version_id, "Put operation completed");
        Ok(versioned)
    }

    /// Get the current value for a key.
    ///
    /// Searches through memory tiers: Hot → Warm → Cold → Deep → Storage
    /// On hit in lower tiers, promotes value up for faster future access.
    pub async fn get(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();
        let full_key = FullKey::new(&namespace, &key);
        trace!("Starting tiered memory lookup");

        // Tier 1: Hot memory (fastest)
        {
            let hot = self.hot.read().await;
            if let Some(v) = hot.get(&full_key) {
                trace!("Hot memory hit");
                return Ok(v.clone());
            }
        }
        trace!("Hot memory miss");

        // Tier 2: Warm memory (recently evicted from hot)
        // First check if key has a mapping in warm
        let warm_id = {
            let warm = self.warm.read().await;
            warm.get_by_key(&full_key)
        };

        if let Some(id) = warm_id {
            let warm = self.warm.read().await;
            if let Some((_, value)) = warm.get(&id) {
                // Promote to hot for faster future access
                drop(warm);
                self.promote_to_hot(full_key.clone(), value.clone()).await;
                return Ok(value);
            }
        }

        // Tier 3: Cold memory (consolidated epochs)
        // Check cold storage for the distinction
        let cold_id = {
            let cold = self.cold.read().await;
            cold.get_by_key(&full_key)
        };

        if let Some(id) = cold_id {
            let cold = self.cold.read().await;
            if let Some((_, _epoch)) = cold.get(&id) {
                // Value found in cold - need to retrieve from storage
                // and promote through warm to hot
                drop(cold);
                if let Ok(value) = self.storage.get(&namespace, &key) {
                    self.promote_through_tiers(full_key, value.clone()).await;
                    return Ok(value);
                }
            }
        }

        // Tier 4: Deep memory (genomic/archival)
        // Deep stores genomes, not individual values
        // But we can check if this key is referenced in recent genomes
        // If so, it indicates the data is "important" and should be kept hot
        let _deep = self.deep.read().await;
        // Deep memory check happens during genome update, not per-get
        drop(_deep);

        // Tier 5: CausalStorage (source of truth)
        match self.storage.get(&namespace, &key) {
            Ok(value) => {
                // Promote to hot for future fast access
                self.promote_to_hot(full_key, value.clone()).await;
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }

    /// Promote a value to hot memory.
    async fn promote_to_hot(&self, key: FullKey, value: VersionedValue) {
        let hot = self.hot.write().await;
        // This may evict something to warm
        let evicted = hot.put(key.clone(), value.clone());

        // Handle eviction if needed
        if let Some(crate::memory::Evicted {
            distinction_id: _,
            versioned,
        }) = evicted
        {
            drop(hot);
            let warm = self.warm.write().await;
            warm.put(key, versioned);
        }
    }

    /// Promote a value through all tiers (Cold→Warm→Hot).
    async fn promote_through_tiers(&self, key: FullKey, value: VersionedValue) {
        // Add to warm first
        {
            let warm = self.warm.write().await;
            warm.put(key.clone(), value.clone());
        }

        // Then add to hot (may trigger warm eviction)
        self.promote_to_hot(key, value).await;
    }

    /// Get the versioned value (metadata included).
    pub async fn get_versioned(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        self.get(namespace, key).await
    }

    /// Synchronous get (for non-async contexts).
    pub fn get_sync(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();
        self.storage.get(&namespace, &key)
    }

    /// Time travel: Get the value at a specific point in time.
    pub async fn get_at(
        &self,
        namespace: &str,
        key: &str,
        timestamp: DateTime<Utc>,
    ) -> DeltaResult<VersionedValue> {
        self.storage.get_at(namespace, key, timestamp)
    }

    /// Get complete history for a key.
    pub async fn history(&self, namespace: &str, key: &str) -> DeltaResult<Vec<HistoryEntry>> {
        self.storage.history(namespace, key)
    }

    /// Query history with filters.
    pub async fn query_history(
        &self,
        namespace: &str,
        key: &str,
        history_query: HistoryQuery,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let mut entries = self.storage.history(namespace, key)?;

        // Apply time range filters
        if let Some(from) = history_query.from_time {
            entries.retain(|e| e.timestamp >= from);
        }
        if let Some(to) = history_query.to_time {
            entries.retain(|e| e.timestamp <= to);
        }

        // Apply value filters using QueryExecutor
        if !history_query.query.filters.is_empty() {
            entries.retain(|e| {
                history_query
                    .query
                    .filters
                    .iter()
                    .all(|f| f.matches_value(&e.value))
            });
        }

        // Apply latest limit
        if let Some(latest) = history_query.latest {
            let start = entries.len().saturating_sub(latest);
            entries = entries.split_off(start);
        }

        Ok(entries)
    }

    // ============================================================================
    // Vector / Embedding Operations (AI Infrastructure)
    // ============================================================================

    /// Store a vector embedding.
    ///
    /// Vectors are stored as versioned values with metadata, enabling:
    /// - Automatic versioning of embeddings
    /// - Time travel for embeddings
    /// - Causal tracking of embedding changes
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace for the embedding (e.g., "documents", "embeddings")
    /// * `key` - The unique key for this embedding
    /// * `vector` - The vector embedding to store
    /// * `metadata` - Optional JSON metadata to store with the vector
    ///
    /// # Example
    ///
    /// ```ignore
    /// let embedding = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
    /// db.embed("docs", "article1", embedding, Some(json!({"title": "AI"}))).await?;
    /// ```
    pub async fn embed(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        vector: Vector,
        metadata: Option<serde_json::Value>,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Serialize vector with metadata
        let value = crate::vector::vector_to_json(&vector, metadata);

        // Store in database (this handles versioning, persistence, etc.)
        let versioned = self.put(&namespace, &key, value).await?;

        // Add to vector index for fast similarity search
        let full_key = FullKey::new(&namespace, &key);
        self.vector_index.add(full_key, vector);

        debug!(namespace = %namespace, key = %key, "Vector embedding stored");
        Ok(versioned)
    }

    /// Search for similar vectors using cosine similarity.
    ///
    /// Performs approximate nearest neighbor search on stored embeddings.
    /// Results are sorted by similarity (highest first).
    ///
    /// # Arguments
    ///
    /// * `namespace` - Optional namespace to search (None = search all)
    /// * `query` - The query vector to search for
    /// * `options` - Search options (top_k, threshold, model_filter)
    ///
    /// # Returns
    ///
    /// A vector of search results sorted by similarity score.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let query = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
    /// let results = db.embed_search(Some("docs"), &query, VectorSearchOptions::new().top_k(5)).await?;
    /// for result in results {
    ///     println!("{}: similarity = {}", result.key, result.score);
    /// }
    /// ```
    pub async fn embed_search(
        &self,
        namespace: Option<&str>,
        query: &Vector,
        options: VectorSearchOptions,
    ) -> DeltaResult<Vec<VectorSearchResult>> {
        // Search the vector index
        let mut results = self.vector_index.search(query, &options);

        // Filter by namespace if specified
        if let Some(ns) = namespace {
            results.retain(|r| r.namespace == ns);
        }

        // Re-apply top_k after namespace filtering
        results.truncate(options.top_k);

        debug!(results = results.len(), "Vector search completed");
        Ok(results)
    }

    /// Search for similar vectors at a specific point in time.
    ///
    /// This is a unique feature of KoruDelta - you can query what vectors
    /// were similar at any historical timestamp.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to search (optional - searches all if None)
    /// * `query` - The query vector
    /// * `timestamp` - ISO 8601 timestamp to search at (e.g., "2026-02-07T12:00:00Z")
    /// * `options` - Search options (top_k, threshold, model_filter)
    ///
    /// # Returns
    ///
    /// A vector of search results as they would have appeared at that time.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // What was similar to my query last Tuesday?
    /// let results = db.similar_at(
    ///     Some("docs"),
    ///     &query,
    ///     "2026-02-01T10:00:00Z",
    ///     VectorSearchOptions::new().top_k(5)
    /// ).await?;
    /// ```
    pub async fn similar_at(
        &self,
        namespace: Option<&str>,
        query: &Vector,
        timestamp: &str,
        options: VectorSearchOptions,
    ) -> DeltaResult<Vec<VectorSearchResult>> {
        use crate::vector::{HnswConfig, HnswIndex};

        // Parse timestamp
        let target_time = timestamp.parse::<DateTime<Utc>>().map_err(|e| {
            crate::error::DeltaError::InvalidData {
                reason: format!("Invalid timestamp '{}': {}", timestamp, e),
            }
        })?;

        // Get all keys in the namespace(s)
        let namespaces_to_search: Vec<String> = match namespace {
            Some(ns) => vec![ns.to_string()],
            None => self.storage.list_namespaces(),
        };

        // Build temporary index with vectors that existed at that time
        let temp_index = HnswIndex::new(HnswConfig::default());
        let mut vector_count = 0;

        for ns in &namespaces_to_search {
            let keys = self.storage.list_keys(ns);
            for key in keys {
                // Try to get the value at that timestamp
                match self.storage.get_at(ns, &key, target_time) {
                    Ok(versioned) => {
                        // Check if it's a valid vector
                        if let Some(vector) = crate::vector::json_to_vector(versioned.value()) {
                            // Check model filter
                            if let Some(ref filter) = options.model_filter {
                                if vector.model() != filter {
                                    continue;
                                }
                            }

                            let full_key = FullKey::new(ns.clone(), key);
                            let _ = temp_index.add(full_key.to_canonical_string(), vector);
                            vector_count += 1;
                        }
                    }
                    Err(_) => {
                        // Key didn't exist at that time, skip
                        continue;
                    }
                }
            }
        }

        debug!(
            vectors = vector_count,
            timestamp = %timestamp,
            "Time-travel vector search"
        );

        if vector_count == 0 {
            return Ok(Vec::new());
        }

        // Search the temporary index
        let results = temp_index.search(query, options.top_k, 50);

        // Filter by namespace and threshold
        let mut filtered: Vec<VectorSearchResult> = results
            .into_iter()
            .filter(|r| {
                // Namespace filter already applied during construction
                r.score >= options.threshold
            })
            .collect();

        // Apply top_k
        filtered.truncate(options.top_k);

        Ok(filtered)
    }

    /// Get a stored vector by key.
    ///
    /// Returns None if the key doesn't exist or if the stored value
    /// is not a valid vector.
    pub async fn get_embed(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<Option<Vector>> {
        let namespace = namespace.into();
        let key = key.into();

        match self.storage.get(&namespace, &key) {
            Ok(versioned) => {
                let vector = crate::vector::json_to_vector(versioned.value());
                Ok(vector)
            }
            Err(_) => Ok(None),
        }
    }

    /// Delete a vector embedding.
    ///
    /// Removes the vector from the search index and stores a null value
    /// (since KoruDelta is append-only, we can't truly delete).
    ///
    /// To "undelete", retrieve the previous version using `history()`.
    pub async fn delete_embed(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Remove from index
        self.vector_index.remove(&namespace, &key);

        // Store null value (mark as deleted)
        let versioned = self.put(&namespace, &key, serde_json::Value::Null).await?;

        debug!(namespace = %namespace, key = %key, "Vector embedding deleted (index removed)");
        Ok(versioned)
    }

    /// Query with full filter, sort, projection, and aggregation support.
    pub async fn query(&self, namespace: &str, query: Query) -> DeltaResult<QueryResult> {
        let items = self
            .storage
            .scan_collection(namespace)
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    value.value().clone(),
                    value.timestamp(),
                    value.version_id().to_string(),
                )
            });

        QueryExecutor::execute(&query, items)
    }

    /// Check if a key exists.
    pub async fn contains(&self, namespace: impl Into<String>, key: impl Into<String>) -> bool {
        let namespace = namespace.into();
        let key = key.into();
        let full_key = FullKey::new(&namespace, &key);

        // Check hot first
        {
            if let Some(hot) = self.hot.try_read() {
                if hot.contains_key(&full_key) {
                    return true;
                }
            }
        }

        // Fallback to storage
        self.storage.contains_key(&namespace, &key)
    }

    /// Check if a key exists (alias for contains).
    pub async fn contains_key(&self, namespace: &str, key: &str) -> bool {
        self.contains(namespace, key).await
    }

    /// Delete a key (marks as deleted by storing null).
    pub async fn delete(&self, namespace: &str, key: &str) -> DeltaResult<()> {
        // Store null as tombstone
        self.put(namespace, key, serde_json::Value::Null).await?;
        Ok(())
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
    pub async fn stats(&self) -> DatabaseStats {
        DatabaseStats {
            key_count: self.storage.key_count(),
            total_versions: self.storage.total_version_count(),
            namespace_count: self.storage.list_namespaces().len(),
        }
    }

    /// Get auth manager.
    pub fn auth(&self) -> &AuthManager {
        &self.auth
    }

    /// Get lifecycle manager for memory consolidation (non-WASM only).
    ///
    /// The lifecycle manager handles automatic Hot→Warm→Cold→Deep
    /// transitions based on access patterns and importance scores.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lifecycle(&self) -> &LifecycleManager {
        &self.lifecycle
    }

    /// Create a workspace.
    ///
    /// Workspaces provide isolated, versioned storage with natural lifecycle.
    /// Each workspace is independent - data in one doesn't affect others.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let db = KoruDelta::start().await?;
    ///
    /// // General purpose workspace
    /// let project = db.workspace("project-alpha");
    /// project.store("config", data, MemoryPattern::Reference).await?;
    ///
    /// // AI agent workspace
    /// let agent = db.workspace("agent-42").ai_context();
    /// agent.remember_episode("User asked about Python").await?;
    ///
    /// // Audit workspace
    /// let audit = db.workspace("audit-2026");
    /// audit.store("tx-123", transaction, MemoryPattern::Event).await?;
    /// ```
    pub fn workspace(&self, name: impl Into<String>) -> crate::memory::Workspace<R> {
        crate::memory::Workspace::new(self.clone(), name)
    }

    /// Get storage reference.
    pub fn storage(&self) -> &Arc<CausalStorage> {
        &self.storage
    }

    /// Get distinction engine reference.
    pub fn engine(&self) -> &Arc<DistinctionEngine> {
        &self.engine
    }

    // =========================================================================
    // Views API
    // =========================================================================

    /// Create a materialized view.
    pub async fn create_view(&self, definition: ViewDefinition) -> DeltaResult<ViewInfo> {
        // First let the view manager validate and execute the query
        let info = self.views.create_view(definition.clone())?;

        // Persist the view definition to WAL via normal put (ensures durability)
        // ViewManager already stored it in storage, but we need WAL persistence
        #[cfg(not(target_arch = "wasm32"))]
        if self.db_path.is_some() {
            use crate::views::VIEW_NAMESPACE;
            let def_value = serde_json::to_value(&definition)?;
            self.put(VIEW_NAMESPACE, &definition.name, def_value)
                .await?;
        }

        Ok(info)
    }

    /// List all views.
    pub async fn list_views(&self) -> Vec<ViewInfo> {
        self.views.list_views()
    }

    /// Refresh a view.
    pub async fn refresh_view(&self, name: &str) -> DeltaResult<ViewInfo> {
        self.views.refresh_view(name)
    }

    /// Query a view.
    pub async fn query_view(&self, name: &str) -> DeltaResult<QueryResult> {
        self.views.query_view(name)
    }

    /// Delete a materialized view.
    pub async fn delete_view(&self, name: &str) -> DeltaResult<()> {
        self.views.delete_view(name)?;

        // Persist the deletion to WAL
        #[cfg(not(target_arch = "wasm32"))]
        if self.db_path.is_some() {
            use crate::views::VIEW_NAMESPACE;
            self.put(VIEW_NAMESPACE, name, serde_json::Value::Null)
                .await?;
        }

        Ok(())
    }

    /// Get view manager.
    pub fn view_manager(&self) -> &Arc<ViewManager> {
        &self.views
    }

    // =========================================================================
    // Subscriptions API (non-WASM only)
    // =========================================================================

    /// Subscribe to changes.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn subscribe(
        &self,
        subscription: Subscription,
    ) -> (
        SubscriptionId,
        tokio::sync::broadcast::Receiver<ChangeEvent>,
    ) {
        self.subscriptions.subscribe(subscription)
    }

    /// Unsubscribe from changes.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn unsubscribe(&self, id: SubscriptionId) -> DeltaResult<()> {
        self.subscriptions.unsubscribe(id)
    }

    /// List all subscriptions.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn list_subscriptions(&self) -> Vec<crate::subscriptions::SubscriptionInfo> {
        self.subscriptions.list_subscriptions()
    }

    /// Get subscription manager.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn subscription_manager(&self) -> &Arc<SubscriptionManager> {
        &self.subscriptions
    }

    /// Store a value and notify subscribers (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn put_notify<T: Serialize>(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: T,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Get previous value and check if key exists before put
        let (exists, previous_value) = match self.get(&namespace, &key).await {
            Ok(v) => (true, Some(v.value().clone())),
            Err(_) => (false, None),
        };

        // Store the value
        let versioned = self.put(&namespace, &key, value).await?;

        // Determine change type
        let change_type = if exists {
            crate::subscriptions::ChangeType::Update
        } else {
            crate::subscriptions::ChangeType::Insert
        };

        // Notify subscribers
        let event = ChangeEvent {
            change_type,
            collection: namespace.clone(),
            key: key.clone(),
            value: Some(versioned.value().clone()),
            previous_value,
            timestamp: Utc::now(),
            version_id: Some(versioned.version_id().to_string()),
            previous_version_id: versioned.previous_version().map(|s| s.to_string()),
        };
        self.subscriptions.notify(event);

        // Auto-refresh views for this collection
        let _ = self.views.refresh_for_collection(&namespace);

        Ok(versioned)
    }

    // =========================================================================
    // Lifecycle
    // =========================================================================

    /// Shutdown the database.
    pub async fn shutdown(self) -> DeltaResult<()> {
        info!("Shutting down KoruDelta");

        let _ = self.shutdown_tx.send(true);
        trace!("Shutdown signal sent to background processes");

        // Release database lock
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref db_path) = self.db_path {
            use crate::persistence;
            if let Err(e) = persistence::release_lock(db_path).await {
                error!(error = %e, "Failed to release database lock");
            } else {
                trace!("Database lock released");
            }
        }

        // TODO: Wait for background processes to complete
        info!("KoruDelta shutdown complete");
        Ok(())
    }
}

/// Database statistics.
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of unique keys
    pub key_count: usize,
    /// Total number of versions
    pub total_versions: usize,
    /// Number of namespaces
    pub namespace_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn create_test_db() -> KoruDelta {
        let config = CoreConfig::default();
        KoruDelta::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_core_creation() {
        let db = create_test_db().await;
        let stats = db.stats().await;
        assert_eq!(stats.key_count, 0);
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let db = create_test_db().await;

        let value = json!({"name": "Alice", "age": 30});
        db.put("users", "alice", value.clone()).await.unwrap();

        let retrieved = db.get("users", "alice").await.unwrap();
        assert_eq!(*retrieved.value(), value);
    }

    #[tokio::test]
    async fn test_contains_key() {
        let db = create_test_db().await;

        assert!(!db.contains_key("users", "alice").await);

        db.put("users", "alice", json!({"name": "Alice"}))
            .await
            .unwrap();

        assert!(db.contains_key("users", "alice").await);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let db = create_test_db().await;

        db.put("users", "alice", json!({"name": "Alice"}))
            .await
            .unwrap();
        db.put("users", "bob", json!({"name": "Bob"}))
            .await
            .unwrap();

        let keys = db.list_keys("users").await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"alice".to_string()));
        assert!(keys.contains(&"bob".to_string()));
    }

    #[tokio::test]
    async fn test_history() {
        let db = create_test_db().await;

        db.put("doc", "readme", json!({"version": 1}))
            .await
            .unwrap();
        db.put("doc", "readme", json!({"version": 2}))
            .await
            .unwrap();
        db.put("doc", "readme", json!({"version": 3}))
            .await
            .unwrap();

        let history = db.history("doc", "readme").await.unwrap();
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_time_travel() {
        let db = create_test_db().await;

        db.put("doc", "readme", json!({"version": 1}))
            .await
            .unwrap();
        let t2 = Utc::now();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        db.put("doc", "readme", json!({"version": 2}))
            .await
            .unwrap();

        let v_at_t2 = db.get_at("doc", "readme", t2).await.unwrap();
        assert_eq!(v_at_t2.value()["version"], 1);
    }

    #[tokio::test]
    async fn test_query_with_filter() {
        use crate::query::Filter;

        let db = create_test_db().await;

        db.put("users", "alice", json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();
        db.put("users", "bob", json!({"name": "Bob", "age": 25}))
            .await
            .unwrap();
        db.put("users", "charlie", json!({"name": "Charlie", "age": 35}))
            .await
            .unwrap();

        let result = db
            .query("users", Query::new().filter(Filter::gt("age", 25)))
            .await
            .unwrap();

        assert_eq!(result.records.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let db = create_test_db().await;

        let stats1 = db.stats().await;
        assert_eq!(stats1.key_count, 0);
        assert_eq!(stats1.total_versions, 0);

        db.put("users", "alice", json!({"user": "alice", "v": 1}))
            .await
            .unwrap();
        db.put("users", "alice", json!({"user": "alice", "v": 2}))
            .await
            .unwrap();
        db.put("users", "bob", json!({"user": "bob", "v": 1}))
            .await
            .unwrap();

        let stats2 = db.stats().await;
        assert_eq!(stats2.key_count, 2);
        assert_eq!(stats2.total_versions, 3);
        assert_eq!(stats2.namespace_count, 1);
    }
}
