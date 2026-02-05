//! # KoruDelta — The Invisible Database
//!
//! **Tagline:** *"Invisible. Causal. Everywhere."*
//!
//! KoruDelta is a zero-configuration, causal database that gives you:
//! - **Git-like history** - Every change is versioned and auditable
//! - **Redis-like simplicity** - Minimal API, zero configuration
//! - **Mathematical guarantees** - Built on distinction calculus
//! - **Natural distribution** - Designed for eventual multi-node sync
//!
//! ## Quick Start
//!
//! ```ignore
//! use koru_delta::KoruDelta;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Start the database (zero configuration)
//!     let db = KoruDelta::start().await?;
//!
//!     // Store data
//!     db.put("users", "alice", json!({
//!         "name": "Alice",
//!         "email": "alice@example.com"
//!     })).await?;
//!
//!     // Retrieve data
//!     let user = db.get("users", "alice").await?;
//!     println!("User: {:?}", user);
//!
//!     // View history
//!     let history = db.history("users", "alice").await?;
//!     for entry in history {
//!         println!("{}: {:?}", entry.timestamp, entry.value);
//!     }
//!
//!     // Time travel
//!     use chrono::Utc;
//!     let timestamp = Utc::now();
//!     let past_value = db.get_at("users", "alice", timestamp).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Core API
//!
//! The KoruDelta API is designed to be minimal and intuitive:
//!
//! - [`KoruDelta::start()`] - Initialize the database
//! - [`KoruDelta::put()`] - Store a value (creates new version)
//! - [`KoruDelta::get()`] - Retrieve current value
//! - [`KoruDelta::history()`] - Get complete change history
//! - [`KoruDelta::get_at()`] - Time travel to specific timestamp
//!
//! ## Architecture
//!
//! KoruDelta is built on three layers:
//!
//! 1. **KoruDelta API** (`core`) - User-facing interface
//! 2. **Causal Storage** (`storage`) - Versioned key-value store
//! 3. **Distinction Engine** (`koru-lambda-core`) - Mathematical foundation
//!
//! The distinction engine provides content-addressing, determinism, and
//! structural integrity. The storage layer maintains causal history and
//! enables time travel. The KoruDelta API wraps it all in a clean interface.
//!
//! ## Thread Safety
//!
//! All KoruDelta operations are thread-safe. You can clone a `KoruDelta`
//! instance cheaply and share it across threads:
//!
//! ```ignore
//! let db = KoruDelta::start().await?;
//! let db_clone = db.clone(); // Cheap clone (Arc internally)
//!
//! tokio::spawn(async move {
//!     db_clone.put("data", "key", json!(42)).await.unwrap();
//! });
//! ```
//!
//! ## Status
//!
//! KoruDelta Phase 3 is complete:
//! - ✅ Single-node operations
//! - ✅ Causal history tracking
//! - ✅ Time travel queries
//! - ✅ Multi-node distribution with automatic sync
//! - ✅ Cluster management (join, peers, status)
//! - ✅ Gossip protocol for peer discovery
//! - ✅ Query engine (filter, sort, project, aggregate)
//! - ✅ Materialized views (create, refresh, query)
//! - ✅ Real-time subscriptions (change notifications)
//!
//! See [DESIGN.md](https://github.com/swyrknt/koru-delta/blob/main/DESIGN.md)
//! for the full architectural vision.

// Internal modules
mod core;
mod error;
mod mapper;
mod types;

// v2.0: Distinction-driven modules
pub mod causal_graph;
pub mod reference_graph;
pub mod memory;

// Storage module (public for testing and cluster operations)
pub mod storage;

// Query module
pub mod query;

// Views module
pub mod views;

// Subscriptions module
#[cfg(not(target_arch = "wasm32"))]
pub mod subscriptions;

// Public modules (not available on WASM - no filesystem/networking)
#[cfg(not(target_arch = "wasm32"))]
pub mod persistence;

#[cfg(not(target_arch = "wasm32"))]
pub mod network;

#[cfg(not(target_arch = "wasm32"))]
pub mod cluster;

// HTTP API (requires http feature, not WASM)
#[cfg(all(not(target_arch = "wasm32"), feature = "http"))]
pub mod http;

// WASM bindings (only when wasm feature is enabled)
#[cfg(feature = "wasm")]
pub mod wasm;

// Public API exports
pub use core::{DatabaseStats, KoruDelta};
pub use error::{DeltaError, DeltaResult};
pub use types::{FullKey, HistoryEntry, VersionedValue};

// Query exports
pub use query::{
    Aggregation, Filter, HistoryQuery, Query, QueryExecutor, QueryRecord, QueryResult, SortBy,
    SortOrder,
};

// Views exports
pub use views::{ViewData, ViewDefinition, ViewInfo, ViewManager};

// Subscriptions exports (non-WASM only)
#[cfg(not(target_arch = "wasm32"))]
pub use subscriptions::{
    ChangeEvent, ChangeType, SubscribableStorage, Subscription, SubscriptionId, SubscriptionInfo,
    SubscriptionManager,
};

// Cluster exports (non-WASM only)
#[cfg(not(target_arch = "wasm32"))]
pub use cluster::{ClusterConfig, ClusterNode, ClusterStatus};

#[cfg(not(target_arch = "wasm32"))]
pub use network::{NodeId, PeerInfo, PeerStatus};

// Re-export commonly used external types for convenience
pub use chrono::{DateTime, Utc};
pub use serde_json::{json, Value as JsonValue};

// Re-export the underlying engine for advanced use cases
pub use koru_lambda_core::DistinctionEngine;

/// Prelude module for convenient imports.
///
/// Import everything you need with:
/// ```ignore
/// use koru_delta::prelude::*;
/// ```
pub mod prelude {
    pub use crate::core::{DatabaseStats, KoruDelta};
    pub use crate::error::{DeltaError, DeltaResult};
    pub use crate::types::{HistoryEntry, VersionedValue};
    pub use chrono::{DateTime, Utc};
    pub use serde_json::{json, Value as JsonValue};

    // Query types
    pub use crate::query::{
        Aggregation, Filter, HistoryQuery, Query, QueryExecutor, QueryRecord, QueryResult, SortBy,
        SortOrder,
    };

    // Views types
    pub use crate::views::{ViewData, ViewDefinition, ViewInfo, ViewManager};

    // Subscriptions types (non-WASM only)
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::subscriptions::{
        ChangeEvent, ChangeType, SubscribableStorage, Subscription, SubscriptionId,
        SubscriptionInfo, SubscriptionManager,
    };

    // Cluster types (non-WASM only)
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::cluster::{ClusterConfig, ClusterNode, ClusterStatus};

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::network::{NodeId, PeerInfo, PeerStatus};
}
