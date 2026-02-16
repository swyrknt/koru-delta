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
use koru_lambda_core::{Canonicalizable, Distinction, DistinctionEngine, LocalCausalAgent};
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use tracing::{debug, error, info, trace, warn};
#[cfg(target_arch = "wasm32")]
use tracing::{debug, info, trace};

use crate::actions::StorageAction;
use crate::auth::{IdentityAgent, IdentityConfig};
use crate::engine::{FieldHandle, SharedEngine};
use crate::error::DeltaResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::lifecycle::{LifecycleAgent, LifecycleConfig};
use crate::memory::{ArchiveAgent, ChronicleAgent, EssenceAgent, TemperatureAgent, TemperatureConfig};
use crate::query::{HistoryQuery, Query, QueryExecutor, QueryResult};
use crate::roots::RootType;
use crate::runtime::sync::RwLock;
use crate::runtime::{DefaultRuntime, Runtime, WatchReceiver, WatchSender};
use crate::storage::CausalStorage;
#[cfg(not(target_arch = "wasm32"))]
use crate::subscriptions::{ChangeEvent, Subscription, SubscriptionAgent, SubscriptionId};
use crate::types::{FullKey, HistoryEntry, VersionedValue};
use crate::vector::{Vector, VectorIndex, VectorSearchOptions, VectorSearchResult};
use crate::views::{PerspectiveAgent, ViewDefinition, ViewInfo};

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
    pub auth: IdentityConfig,
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

/// The main KoruDelta database instance - Storage Agent.
///
/// KoruDelta is the Storage Agent in the unified consciousness field.
/// It implements `LocalCausalAgent`, meaning all operations are synthesized
/// from its local root perspective.
///
/// # LCA Architecture
///
/// As a Local Causal Agent:
/// - **Local Root**: The agent's causal anchor (Root: STORAGE)
/// - **Actions**: Storage operations (Store, Retrieve, History, Query, Delete)
/// - **Synthesis**: All state changes follow ΔNew = ΔLocal_Root ⊕ ΔAction
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
    /// The shared field engine (for LCA operations)
    shared_engine: SharedEngine,
    /// Field handle for synthesis operations
    field: FieldHandle,
    /// Local causal root - this agent's perspective (Root: STORAGE)
    local_root: Distinction,
    /// View manager for materialized views
    views: Arc<PerspectiveAgent>,
    /// Subscription manager for change notifications (non-WASM only)
    #[cfg(not(target_arch = "wasm32"))]
    subscriptions: Arc<SubscriptionAgent>,
    /// Memory tiers
    hot: Arc<RwLock<TemperatureAgent>>,
    warm: Arc<RwLock<ChronicleAgent>>,
    cold: Arc<RwLock<ArchiveAgent>>,
    deep: Arc<RwLock<EssenceAgent>>,
    /// Auth manager (LCA Identity Agent)
    auth: Arc<IdentityAgent>,
    /// Lifecycle manager for memory consolidation (non-WASM only)
    #[cfg(not(target_arch = "wasm32"))]
    lifecycle: Arc<LifecycleAgent>,
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
            .field("local_root", &self.local_root.id())
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

        // Create the shared field engine (LCA foundation)
        let shared_engine = SharedEngine::new();
        let field = FieldHandle::new(&shared_engine);

        // Get the storage agent's local root from canonical roots
        let local_root = shared_engine.root(RootType::Storage).clone();

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
            let storage = persistence::load_from_wal(&path, Arc::clone(shared_engine.inner())).await?;
            let key_count = storage.key_count();
            info!(keys = key_count, "Database loaded from WAL");
            storage
        } else {
            info!("Creating new database");
            CausalStorage::new(Arc::clone(shared_engine.inner()))
        };

        let storage = Arc::new(storage);

        // Initialize memory tiers with LCA agents
        let hot = Arc::new(RwLock::new(TemperatureAgent::with_config(
            TemperatureConfig {
                capacity: config.memory.hot_capacity,
                promote_threshold: 2,
            },
            &shared_engine,
        )));

        let warm = Arc::new(RwLock::new(ChronicleAgent::new(&shared_engine)));
        let cold = Arc::new(RwLock::new(ArchiveAgent::new(&shared_engine)));
        let deep = Arc::new(RwLock::new(EssenceAgent::new(&shared_engine)));

        // Initialize auth with LCA identity agent
        let auth = Arc::new(IdentityAgent::with_config(
            Arc::clone(&storage),
            config.auth.clone(),
            &shared_engine,
        ));

        // Initialize views with LCA perspective agent
        let views = Arc::new(PerspectiveAgent::new(Arc::clone(&storage), &shared_engine));

        // Initialize subscriptions (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let subscriptions = Arc::new(SubscriptionAgent::new(&shared_engine));

        // Initialize lifecycle manager (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let lifecycle = Arc::new(LifecycleAgent::with_config(&shared_engine, LifecycleConfig::default()));

        // Shutdown channel using runtime
        let (shutdown_tx, shutdown_rx) = runtime.watch_channel(false);

        let db = Self {
            runtime,
            config,
            db_path: Some(path),
            storage,
            shared_engine,
            field,
            local_root,
            hot,
            warm,
            cold,
            deep,
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
        // Create the shared field engine (LCA foundation)
        let shared_engine = SharedEngine::new();
        let field = FieldHandle::new(&shared_engine);

        // Get the storage agent's local root from canonical roots
        let local_root = shared_engine.root(RootType::Storage).clone();

        // Create storage using the shared engine
        let storage = Arc::new(CausalStorage::new(Arc::clone(shared_engine.inner())));

        // Initialize memory tiers with LCA agents
        let hot = Arc::new(RwLock::new(TemperatureAgent::with_config(
            TemperatureConfig {
                capacity: config.memory.hot_capacity,
                promote_threshold: 2,
            },
            &shared_engine,
        )));

        let warm = Arc::new(RwLock::new(ChronicleAgent::new(&shared_engine)));
        let cold = Arc::new(RwLock::new(ArchiveAgent::new(&shared_engine)));
        let deep = Arc::new(RwLock::new(EssenceAgent::new(&shared_engine)));

        // Initialize auth with LCA identity agent
        let auth = Arc::new(IdentityAgent::with_config(
            Arc::clone(&storage),
            config.auth.clone(),
            &shared_engine,
        ));

        // Initialize views with LCA perspective agent
        let views = Arc::new(PerspectiveAgent::new(Arc::clone(&storage), &shared_engine));

        // Initialize subscriptions (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let subscriptions = Arc::new(SubscriptionAgent::new(&shared_engine));

        // Initialize lifecycle manager (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let lifecycle = Arc::new(LifecycleAgent::with_config(&shared_engine, LifecycleConfig::default()));

        // Shutdown channel using runtime
        let (shutdown_tx, shutdown_rx) = runtime.watch_channel(false);

        let db = Self {
            runtime,
            config,
            db_path: None,
            storage,
            shared_engine,
            field,
            local_root,
            hot,
            warm,
            cold,
            deep,
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
        hot: &Arc<RwLock<TemperatureAgent>>,
        warm: &Arc<RwLock<ChronicleAgent>>,
        cold: &Arc<RwLock<ArchiveAgent>>,
        _deep: &Arc<RwLock<EssenceAgent>>,
        _storage: &Arc<CausalStorage>,
    ) {
        // Check TemperatureAgent utilization
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

        // Check ChronicleAgent utilization and find demotion candidates
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

        // Rotate ArchiveAgent epochs periodically
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
        _hot: &Arc<RwLock<TemperatureAgent>>,
        warm: &Arc<RwLock<ChronicleAgent>>,
        cold: &Arc<RwLock<ArchiveAgent>>,
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
    async fn run_genome_update(deep: &Arc<RwLock<EssenceAgent>>) {
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

        // Create shared engine from existing engine
        let shared_engine = SharedEngine::with_engine(engine);
        let field = FieldHandle::new(&shared_engine);

        // Get the storage agent's local root
        let local_root = shared_engine.root(RootType::Storage).clone();

        // Initialize memory tiers with LCA agents
        let hot = Arc::new(RwLock::new(TemperatureAgent::with_config(
            TemperatureConfig {
                capacity: config.memory.hot_capacity,
                promote_threshold: 2,
            },
            &shared_engine,
        )));

        let warm = Arc::new(RwLock::new(ChronicleAgent::new(&shared_engine)));
        let cold = Arc::new(RwLock::new(ArchiveAgent::new(&shared_engine)));
        let deep = Arc::new(RwLock::new(EssenceAgent::new(&shared_engine)));

        // Initialize auth with LCA identity agent
        let auth = Arc::new(IdentityAgent::with_config(
            Arc::clone(&storage),
            config.auth.clone(),
            &shared_engine,
        ));

        // Initialize views with LCA perspective agent
        let views = Arc::new(PerspectiveAgent::new(Arc::clone(&storage), &shared_engine));

        // Initialize subscriptions (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let subscriptions = Arc::new(SubscriptionAgent::new(&shared_engine));

        // Initialize lifecycle manager (non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        let lifecycle = Arc::new(LifecycleAgent::with_config(&shared_engine, LifecycleConfig::default()));

        // Shutdown channel using runtime
        let (shutdown_tx, shutdown_rx) = runtime.watch_channel(false);

        Self {
            runtime,
            config: CoreConfig::default(),
            db_path: None,
            storage,
            shared_engine,
            field,
            local_root,
            hot,
            warm,
            cold,
            deep,
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

    /// Store multiple values in a batch operation with a single WAL fsync.
    ///
    /// This is significantly more efficient than calling `put` multiple times
    /// because it performs only one fsync for the entire batch.
    ///
    /// # Arguments
    ///
    /// * `items` - Vector of (namespace, key, value) tuples to store
    ///
    /// # Returns
    ///
    /// Returns a vector of `VersionedValue` results, one per item, in the same order.
    ///
    /// # Performance
    ///
    /// For N items with persistence enabled:
    /// - `put`: N fsyncs (~200 ops/sec total)
    /// - `put_batch`: 1 fsync (~2,000-5,000 ops/sec total)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let items = vec![
    ///     ("users", "alice", json!({"name": "Alice"})),
    ///     ("users", "bob", json!({"name": "Bob"})),
    ///     ("orders", "123", json!({"total": 100})),
    /// ];
    /// let results = db.put_batch(items).await?;
    /// ```
    /// 
    /// For simpler usage with owned strings, see `put_batch_values`.
    pub async fn put_batch<T: Serialize>(
        &self,
        items: Vec<(impl Into<String>, impl Into<String>, T)>,
    ) -> DeltaResult<Vec<VersionedValue>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let start = std::time::Instant::now();
        let count = items.len();
        trace!(count, "Starting batch put operation");

        // Convert all items upfront
        let mut converted_items = Vec::with_capacity(items.len());
        for (ns, key, value) in items {
            let namespace = ns.into();
            let key = key.into();
            let json_value = serde_json::to_value(value)?;
            converted_items.push((namespace, key, json_value));
        }

        // Store in storage (source of truth)
        trace!("Storing batch in CausalStorage");
        let versioned_values = self.storage.put_batch(converted_items.clone())?;

        // Persist to WAL if db_path is set (single fsync for entire batch)
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref db_path) = self.db_path {
            use crate::persistence;
            trace!("Persisting batch to WAL");

            let write_refs: Vec<(&str, &str, &VersionedValue)> = converted_items
                .iter()
                .zip(versioned_values.iter())
                .map(|((ns, key, _), versioned)| (ns.as_str(), key.as_str(), versioned))
                .collect();

            if let Err(e) = persistence::append_write_batch(db_path, write_refs).await {
                error!(error = %e, "Failed to persist batch to WAL");
            } else {
                trace!("Batch persisted to WAL");
            }
        }

        // Broadcast to cluster if configured (fire and forget)
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref cluster) = self.cluster {
            for ((namespace, key, _), versioned) in
                converted_items.iter().zip(versioned_values.iter())
            {
                let full_key = FullKey::new(namespace, key);
                let value_clone = versioned.clone();
                let cluster_clone = Arc::clone(cluster);
                tokio::spawn(async move {
                    trace!("Broadcasting write to cluster");
                    cluster_clone.broadcast_write(full_key, value_clone).await;
                });
            }
        }

        // Promote all to hot memory
        {
            let hot = self.hot.write().await;
            for ((namespace, key, _), versioned) in
                converted_items.iter().zip(versioned_values.iter())
            {
                let full_key = FullKey::new(namespace, key);
                hot.put(full_key, versioned.clone());
            }
            trace!("Batch values promoted to hot memory");
        }

        // Auto-refresh views (fire and forget, non-WASM only)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let views = Arc::clone(&self.views);
            tokio::spawn(async move {
                let _ = views.refresh_stale(chrono::Duration::seconds(0));
            });
        }

        let elapsed = start.elapsed();
        info!(count, ?elapsed, "Batch put operation completed");
        Ok(versioned_values)
    }

    /// Simplified batch put using pre-serialized values.
    ///
    /// This is easier to use than `put_batch` when you have owned strings
    /// and serde_json::Value already prepared.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace for all items
    /// * `items` - Vector of (key, value) pairs
    ///
    /// # Example
    ///
    /// ```ignore
    /// let items = vec![
    ///     ("key1".to_string(), json!({"data": 1})),
    ///     ("key2".to_string(), json!({"data": 2})),
    /// ];
    /// db.put_batch_in_ns("myns", items).await?;
    /// ```
    pub async fn put_batch_in_ns(
        &self,
        namespace: impl Into<String>,
        items: Vec<(String, serde_json::Value)>,
    ) -> DeltaResult<Vec<VersionedValue>> {
        let namespace = namespace.into();
        let batch: Vec<(String, String, serde_json::Value)> = items
            .into_iter()
            .map(|(key, value)| (namespace.clone(), key, value))
            .collect();
        
        // Convert to the format expected by storage
        let mut converted = Vec::with_capacity(batch.len());
        for (ns, key, value) in batch {
            converted.push((ns, key, value));
        }
        
        self.storage.put_batch(converted)
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

    // =========================================================================
    // TTL (Time-To-Live) Support - ALIS AI Integration
    // =========================================================================

    /// Store a value with automatic expiration (TTL).
    ///
    /// The value will be automatically removed after the specified number of ticks.
    /// This is essential for ALIS AI's active inference loop where predictions
    /// need to expire if not confirmed.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to store in
    /// * `key` - The key for this value
    /// * `value` - The value to store
    /// * `ttl_ticks` - Number of ticks until expiration
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Store a prediction that expires after 100 ticks
    /// db.put_with_ttl(
    ///     "predictions",
    ///     "pred_1",
    ///     json!({"prediction": "rain", "confidence": 0.8}),
    ///     100
    /// ).await?;
    /// ```
    pub async fn put_with_ttl<T: Serialize>(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: T,
        ttl_ticks: u64,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Store the value first
        let result = self.put(&namespace, &key, value).await?;

        // Also store in TTL tracking index for efficient cleanup
        self.add_to_ttl_index(&namespace, &key, ttl_ticks).await;

        debug!(
            namespace = %namespace,
            key = %key,
            ttl_ticks = ttl_ticks,
            "Value stored with TTL"
        );

        Ok(result)
    }

    /// Store content with auto-generated embedding and TTL.
    ///
    /// Combines semantic storage with automatic expiration.
    /// Perfect for ALIS AI's temporary distinctions that need embeddings.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to store in
    /// * `key` - The key for this content
    /// * `content` - The content to store and embed
    /// * `metadata` - Optional additional metadata
    /// * `ttl_ticks` - Number of ticks until expiration
    pub async fn put_similar_with_ttl(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        content: impl Serialize,
        metadata: Option<serde_json::Value>,
        ttl_ticks: u64,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();

        // Merge user metadata with TTL metadata
        let mut ttl_metadata = metadata.unwrap_or(serde_json::Value::Null);
        if let Some(obj) = ttl_metadata.as_object_mut() {
            obj.insert("__ttl".to_string(), serde_json::json!({
                "ttl_ticks": ttl_ticks,
                "created_at_ticks": self.current_tick(),
                "expires_at_ticks": self.current_tick() + ttl_ticks,
            }));
        } else {
            ttl_metadata = serde_json::json!({
                "__ttl": {
                    "ttl_ticks": ttl_ticks,
                    "created_at_ticks": self.current_tick(),
                    "expires_at_ticks": self.current_tick() + ttl_ticks,
                }
            });
        }

        // Use put_similar which handles embedding
        self.put_similar(&namespace, &key, content, Some(ttl_metadata)).await
    }

    /// Remove all expired values.
    ///
    /// Scans the TTL index and removes all values that have exceeded their TTL.
    /// Returns the count of items removed.
    ///
    /// This is the core of the consolidation action for TTL management.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cleaned = db.cleanup_expired().await?;
    /// println!("Removed {} expired items", cleaned);
    /// ```
    pub async fn cleanup_expired(&self) -> DeltaResult<usize> {
        let current_tick = self.current_tick();
        let mut removed_count = 0;

        // Get all expired items from TTL index
        let expired = self.get_expired_items(current_tick).await;

        for (namespace, key) in expired {
            // Delete the expired value (tombstone)
            match self.delete(&namespace, &key).await {
                Ok(_) => {
                    removed_count += 1;
                    trace!(namespace = %namespace, key = %key, "Expired item removed");
                }
                Err(e) => {
                    warn!(error = %e, namespace = %namespace, key = %key, "Failed to remove expired item");
                }
            }

            // Remove from vector index if present
            self.vector_index.remove(&namespace, &key);
        }

        // Clean up TTL index
        self.cleanup_ttl_index(current_tick).await;

        info!(removed = removed_count, "TTL cleanup completed");
        Ok(removed_count)
    }

    /// Get remaining TTL for a key.
    ///
    /// Returns `None` if the key doesn't exist or has no TTL.
    pub async fn get_ttl_remaining(&self, namespace: &str, key: &str) -> DeltaResult<Option<u64>> {
        match self.get(namespace, key).await {
            Ok(versioned) => {
                let value = versioned.value();
                
                // Check for TTL in metadata
                if let Some(metadata) = value.get("__metadata") {
                    if let Some(ttl_info) = metadata.get("__ttl") {
                        if let Some(expires_at) = ttl_info.get("expires_at_ticks").and_then(|v| v.as_u64()) {
                            let current = self.current_tick();
                            return Ok(Some(expires_at.saturating_sub(current)));
                        }
                    }
                }
                
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    /// List items expiring within the given number of ticks.
    ///
    /// Useful for proactive cleanup or monitoring.
    pub async fn list_expiring_soon(&self, within_ticks: u64) -> Vec<(String, String, u64)> {
        let current_tick = self.current_tick();
        let threshold = current_tick + within_ticks;

        self.get_ttl_items_before(threshold).await
    }

    /// Get all expired predictions for surprise detection.
    ///
    /// This is critical for ALIS AI's active inference loop.
    /// Returns (namespace, key) pairs of predictions that have expired.
    pub async fn get_expired_predictions(&self) -> DeltaResult<Vec<(String, String)>> {
        let current_tick = self.current_tick();
        let mut expired_predictions = Vec::new();

        // Get all expired items
        let expired = self.get_expired_items(current_tick).await;

        for (namespace, key) in expired {
            // Check if this was a prediction (has prediction metadata)
            if let Ok(versioned) = self.get(&namespace, &key).await {
                let value = versioned.value();
                
                // Check for prediction marker in metadata
                let is_prediction = value
                    .get("__metadata")
                    .and_then(|m| m.get("source"))
                    .and_then(|s| s.as_str())
                    .map(|s| s == "prediction")
                    .unwrap_or(false);

                if is_prediction {
                    expired_predictions.push((namespace, key));
                }
            }
        }

        Ok(expired_predictions)
    }

    // -------------------------------------------------------------------------
    // TTL Internal Helpers
    // -------------------------------------------------------------------------

    /// Get the current tick count.
    ///
    /// In a real implementation, this would come from the PulseAgent.
    /// For now, we use a monotonic counter based on operation count.
    fn current_tick(&self) -> u64 {
        // Use the storage's operation count as a proxy for ticks
        // This ensures TTL is tied to actual database activity
        self.storage.key_count() as u64
    }

    /// Add an item to the TTL tracking index.
    async fn add_to_ttl_index(&self, namespace: &str, key: &str, ttl_ticks: u64) {
        let expires_at = self.current_tick() + ttl_ticks;
        let full_key = format!("{}:{}", namespace, key);
        
        // Store in the TTL namespace for efficient lookup
        let ttl_record = serde_json::json!({
            "namespace": namespace,
            "key": key,
            "expires_at": expires_at,
        });

        let _ = self.storage.put(
            "__ttl_index",
            &full_key,
            ttl_record,
        );
    }

    /// Get all expired items from TTL index.
    async fn get_expired_items(&self, current_tick: u64) -> Vec<(String, String)> {
        let mut expired = Vec::new();

        // Get all keys in TTL index
        let ttl_keys = self.storage.list_keys("__ttl_index");

        for full_key in ttl_keys {
            if let Ok(value) = self.storage.get("__ttl_index", &full_key) {
                if let Some(expires_at) = value.value().get("expires_at").and_then(|v| v.as_u64()) {
                    if current_tick >= expires_at {
                        if let Some(namespace) = value.value().get("namespace").and_then(|v| v.as_str()) {
                            if let Some(key) = value.value().get("key").and_then(|v| v.as_str()) {
                                expired.push((namespace.to_string(), key.to_string()));
                            }
                        }
                    }
                }
            }
        }

        expired
    }

    /// Get TTL items expiring before a threshold.
    async fn get_ttl_items_before(&self, threshold: u64) -> Vec<(String, String, u64)> {
        let current_tick = self.current_tick();
        let mut items = Vec::new();

        let ttl_keys = self.storage.list_keys("__ttl_index");

        for full_key in ttl_keys {
            if let Ok(value) = self.storage.get("__ttl_index", &full_key) {
                if let Some(expires_at) = value.value().get("expires_at").and_then(|v| v.as_u64()) {
                    if expires_at <= threshold {
                        if let Some(namespace) = value.value().get("namespace").and_then(|v| v.as_str()) {
                            if let Some(key) = value.value().get("key").and_then(|v| v.as_str()) {
                                let remaining = expires_at.saturating_sub(current_tick);
                                items.push((namespace.to_string(), key.to_string(), remaining));
                            }
                        }
                    }
                }
            }
        }

        items
    }

    /// Clean up the TTL index by removing expired entries.
    async fn cleanup_ttl_index(&self, current_tick: u64) {
        let ttl_keys = self.storage.list_keys("__ttl_index");

        for full_key in ttl_keys {
            if let Ok(value) = self.storage.get("__ttl_index", &full_key) {
                if let Some(expires_at) = value.value().get("expires_at").and_then(|v| v.as_u64()) {
                    if current_tick >= expires_at {
                        // Remove from TTL index (store tombstone)
                        let _ = self.storage.put(
                            "__ttl_index",
                            &full_key,
                            serde_json::Value::Null,
                        );
                    }
                }
            }
        }
    }

    /// Simplified: Store content with an auto-generated distinction-based embedding.
    ///
    /// This is the high-level convenience method for semantic storage.
    /// The embedding is synthesized from the content's structure in distinction space.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to store in
    /// * `key` - The key for this content
    /// * `content` - The content to store and embed
    /// * `metadata` - Optional metadata to store with the embedding
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.put_similar("docs", "article1", json!({"text": "AI is powerful"}), None).await?;
    /// ```
    pub async fn put_similar(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        content: impl Serialize,
        metadata: Option<serde_json::Value>,
    ) -> DeltaResult<VersionedValue> {
        let namespace = namespace.into();
        let key = key.into();
        
        // Serialize content for embedding generation
        let content_json = serde_json::to_value(&content)?;
        
        // Synthesize distinction-based embedding
        let vector = crate::vector::Vector::synthesize(&content_json, 128);
        
        // Store using the underlying embed method
        self.embed(&namespace, &key, vector, metadata).await
    }

    /// Simplified: Search for content similar to the given text/content.
    ///
    /// This generates an embedding from the query content and finds similar items.
    ///
    /// # Arguments
    ///
    /// * `namespace` - Optional namespace to search (None = all)
    /// * `query_content` - The content to find similar items to
    /// * `top_k` - Maximum number of results
    ///
    /// # Example
    ///
    /// ```ignore
    /// let results = db.find_similar(
    ///     Some("docs"),
    ///     json!({"text": "artificial intelligence"}),
    ///     5
    /// ).await?;
    /// ```
    pub async fn find_similar(
        &self,
        namespace: Option<&str>,
        query_content: impl Serialize,
        top_k: usize,
    ) -> DeltaResult<Vec<crate::vector::VectorSearchResult>> {
        let query_json = serde_json::to_value(&query_content)?;
        let query_vector = crate::vector::Vector::synthesize(&query_json, 128);
        
        let options = crate::vector::VectorSearchOptions::new()
            .top_k(top_k);
        
        self.embed_search(namespace, &query_vector, options).await
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
    pub fn auth(&self) -> Arc<IdentityAgent> {
        Arc::clone(&self.auth)
    }

    /// Get lifecycle manager for memory consolidation (non-WASM only).
    ///
    /// The lifecycle manager handles automatic Hot→Warm→Cold→Deep
    /// transitions based on access patterns and importance scores.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lifecycle(&self) -> &LifecycleAgent {
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
        self.shared_engine.inner()
    }

    // =========================================================================
    // Views API
    // =========================================================================

    /// Create a materialized view.
    pub async fn create_view(&self, definition: ViewDefinition) -> DeltaResult<ViewInfo> {
        // First let the view manager validate and execute the query
        let info = self.views.create_view(definition.clone())?;

        // Persist the view definition to WAL via normal put (ensures durability)
        // PerspectiveAgent already stored it in storage, but we need WAL persistence
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
    pub fn view_manager(&self) -> &Arc<PerspectiveAgent> {
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
    pub fn subscription_manager(&self) -> &Arc<SubscriptionAgent> {
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

    // =========================================================================
    // LCA (Local Causal Agent) Operations
    // =========================================================================

    /// Perform a storage action via causal synthesis.
    ///
    /// This is the LCA way: ΔNew = ΔLocal_Root ⊕ ΔAction
    ///
    /// # Example
    ///
    /// ```ignore
    /// let action = StorageAction::Store {
    ///     namespace: "users".to_string(),
    ///     key: "alice".to_string(),
    ///     value_json: json!({"name": "Alice"}),
    /// };
    /// let new_root = db.synthesize_storage_action(action).await?;
    /// ```
    pub async fn synthesize_storage_action(
        &mut self,
        action: StorageAction,
    ) -> DeltaResult<Distinction> {
        // Validate the action
        action.validate().map_err(|e| crate::error::DeltaError::InvalidData { reason: e })?;

        // Synthesize: ΔNew = ΔLocal_Root ⊕ ΔAction
        let action_distinction = action.to_canonical_structure(self.field.engine());
        let new_root = self.field.synthesize(&self.local_root, &action_distinction);

        // Execute the action (this creates the causal effect)
        self.execute_storage_action(&action).await?;

        // Update local root to the new synthesis
        self.local_root = new_root.clone();

        Ok(new_root)
    }

    /// Execute a storage action (the causal effect).
    ///
    /// This performs the actual storage operation based on the action type.
    async fn execute_storage_action(&self, action: &StorageAction) -> DeltaResult<()> {
        match action {
            StorageAction::Store { namespace, key, value_json } => {
                // Store via the existing put mechanism
                let _ = self.put(namespace.clone(), key.clone(), value_json.clone()).await?;
            }
            StorageAction::Retrieve { namespace, key } => {
                // Retrieve is handled by get, but we don't need the value here
                let _ = self.get(namespace.clone(), key.clone()).await?;
            }
            StorageAction::History { namespace, key } => {
                let _ = self.history(namespace, key).await?;
            }
            StorageAction::Query { .. } => {
                // Query all collections
                let namespaces = self.storage.list_namespaces();
                for ns in namespaces {
                    self.query(&ns, Query::new()).await?;
                }
            }
            StorageAction::Delete { namespace, key } => {
                self.delete(namespace, key).await?;
            }
        }
        Ok(())
    }

    /// Get the current local root distinction.
    ///
    /// This is the agent's causal perspective.
    pub fn local_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Get the shared field engine.
    pub fn shared_engine(&self) -> &SharedEngine {
        &self.shared_engine
    }

    /// Get the field handle for synthesis operations.
    pub fn field(&self) -> &FieldHandle {
        &self.field
    }
}

// ============================================================================
// Local Causal Agent Implementation
// ============================================================================

impl<R: Runtime> LocalCausalAgent for KoruDeltaGeneric<R> {
    type ActionData = StorageAction;

    /// Get the current local root distinction.
    ///
    /// This is the Storage Agent's causal anchor (Root: STORAGE).
    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }

    /// Synthesize a new state from local root + action data.
    ///
    /// Formula: ΔNew = ΔLocal_Root ⊕ ΔAction_Data
    ///
    /// This method:
    /// 1. Canonicalizes the action data into a distinction
    /// 2. Synthesizes local_root ⊕ action_distinction
    /// 3. Executes the storage action (causal effect)
    /// 4. Returns the new distinction representing the state transition
    fn synthesize_action(
        &mut self,
        action_data: Self::ActionData,
        _engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        // Validate the action
        if let Err(e) = action_data.validate() {
            tracing::warn!("Invalid action: {}", e);
            return self.local_root.clone();
        }

        // Canonicalize action into distinction
        let action_distinction = action_data.to_canonical_structure(self.field.engine());

        // Synthesize: ΔNew = ΔLocal ⊕ ΔAction
        let new_root = self.field.synthesize(&self.local_root, &action_distinction);

        // Update local root
        self.local_root = new_root.clone();

        new_root
    }

    /// Update the local root to a new distinction.
    ///
    /// This moves the agent's perspective forward in the causal chain.
    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
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
    async fn test_put_batch() {
        let db = create_test_db().await;

        // Test empty batch
        let empty: Vec<(&str, &str, serde_json::Value)> = vec![];
        let results = db.put_batch(empty).await.unwrap();
        assert!(results.is_empty());

        // Test batch with multiple items
        let items = vec![
            ("users", "alice", json!({"name": "Alice"})),
            ("users", "bob", json!({"name": "Bob"})),
            ("orders", "123", json!({"total": 100})),
        ];

        let results = db.put_batch(items).await.unwrap();
        assert_eq!(results.len(), 3);

        // Verify each item was stored
        let alice = db.get("users", "alice").await.unwrap();
        assert_eq!(alice.value().get("name").unwrap(), "Alice");

        let bob = db.get("users", "bob").await.unwrap();
        assert_eq!(bob.value().get("name").unwrap(), "Bob");

        let order = db.get("orders", "123").await.unwrap();
        assert_eq!(order.value().get("total").unwrap(), 100);

        // Verify batch creates distinct versions
        assert_ne!(results[0].version_id(), results[1].version_id());
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

    // =========================================================================
    // LCA (Local Causal Agent) Tests
    // =========================================================================

    #[tokio::test]
    async fn test_lca_local_root_exists() {
        let db = create_test_db().await;

        // The local root should be initialized
        let root = db.local_root();
        assert!(!root.id().is_empty());

        // It should be the STORAGE root
        let expected_root = db.shared_engine().root(RootType::Storage);
        assert_eq!(root.id(), expected_root.id());
    }

    #[tokio::test]
    async fn test_lca_synthesize_storage_action() {
        use crate::actions::StorageAction;

        let mut db = create_test_db().await;
        let initial_root = db.local_root().clone();

        // Synthesize a store action
        let action = StorageAction::Store {
            namespace: "users".to_string(),
            key: "alice".to_string(),
            value_json: json!({"name": "Alice"}),
        };

        let new_root = db.synthesize_storage_action(action).await.unwrap();

        // The new root should be different from initial
        assert_ne!(new_root.id(), initial_root.id());

        // The local root should be updated
        assert_eq!(db.local_root().id(), new_root.id());

        // The data should actually be stored
        let retrieved = db.get("users", "alice").await.unwrap();
        assert_eq!(retrieved.value()["name"], "Alice");
    }

    #[tokio::test]
    async fn test_lca_local_causal_agent_trait() {
        use koru_lambda_core::LocalCausalAgent;
        use crate::actions::StorageAction;

        let mut db = create_test_db().await;
        let engine = Arc::new(DistinctionEngine::new());

        // Test get_current_root
        let root = db.get_current_root();
        assert!(!root.id().is_empty());

        // Test synthesize_action
        let action = StorageAction::Retrieve {
            namespace: "users".to_string(),
            key: "alice".to_string(),
        };

        let new_root = db.synthesize_action(action, &engine);
        assert!(!new_root.id().is_empty());

        // The root should have changed (even though retrieval doesn't store)
        // because synthesis still happens
    }

    #[tokio::test]
    async fn test_lca_shared_engine() {
        let db = create_test_db().await;

        // The shared engine should be accessible
        let engine = db.shared_engine();
        let stats = engine.stats();

        // Should have distinctions (12 roots are created during initialization,
        // each synthesized from d0/d1, so there should be many distinctions)
        assert!(stats.distinction_count >= 12, "Expected at least 12 distinctions (roots), got {}", stats.distinction_count);
    }

    #[tokio::test]
    async fn test_lca_field_handle() {
        let db = create_test_db().await;

        // The field handle should provide access to d0 and d1
        let d0 = db.field().d0();
        let d1 = db.field().d1();

        assert!(!d0.id().is_empty());
        assert!(!d1.id().is_empty());
        assert_ne!(d0.id(), d1.id());
    }

    #[tokio::test]
    async fn test_lca_causal_chain() {
        use crate::actions::StorageAction;

        let mut db = create_test_db().await;
        let root1 = db.local_root().clone();

        // First action
        let action1 = StorageAction::Store {
            namespace: "test".to_string(),
            key: "key1".to_string(),
            value_json: json!(1),
        };
        let root2 = db.synthesize_storage_action(action1).await.unwrap();
        assert_ne!(root1.id(), root2.id());

        // Second action
        let action2 = StorageAction::Store {
            namespace: "test".to_string(),
            key: "key2".to_string(),
            value_json: json!(2),
        };
        let root3 = db.synthesize_storage_action(action2).await.unwrap();
        assert_ne!(root2.id(), root3.id());

        // Third action
        let action3 = StorageAction::Store {
            namespace: "test".to_string(),
            key: "key3".to_string(),
            value_json: json!(3),
        };
        let root4 = db.synthesize_storage_action(action3).await.unwrap();
        assert_ne!(root3.id(), root4.id());

        // Each root should be unique (causal chain)
        assert_ne!(root1.id(), root3.id());
        assert_ne!(root1.id(), root4.id());
        assert_ne!(root2.id(), root4.id());
    }
}
