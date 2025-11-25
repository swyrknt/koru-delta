/// Core KoruDelta database implementation.
///
/// This module provides the main user-facing API for KoruDelta. It wraps
/// the storage layer with a clean, ergonomic interface that hides the
/// complexity of distinction calculus and causal history tracking.
///
/// # Design Philosophy
///
/// - **Simple API**: put, get, history, get_at - nothing more
/// - **Async-ready**: Future-proof for distributed operations
/// - **Type-safe**: Leverage Rust's type system for correctness
/// - **Thread-safe**: Share KoruDelta instances across threads safely
use crate::error::{DeltaError, DeltaResult};
use crate::query::{HistoryQuery, Query, QueryExecutor, QueryResult};
use crate::storage::CausalStorage;
use crate::subscriptions::{ChangeEvent, Subscription, SubscriptionId, SubscriptionManager};
use crate::types::{HistoryEntry, VersionedValue};
use crate::views::{ViewDefinition, ViewInfo, ViewManager};
use chrono::{DateTime, Utc};
use koru_lambda_core::DistinctionEngine;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::broadcast;

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
/// # Example
///
/// ```ignore
/// use koru_delta::KoruDelta;
/// use serde_json::json;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let db = KoruDelta::start().await?;
///
///     // Store data
///     db.put("users", "alice", json!({
///         "name": "Alice",
///         "email": "alice@example.com"
///     })).await?;
///
///     // Retrieve data
///     let user = db.get("users", "alice").await?;
///     println!("User: {:?}", user);
///
///     // View history
///     let history = db.history("users", "alice").await?;
///     println!("Changes: {}", history.len());
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct KoruDelta {
    /// The underlying storage engine
    storage: Arc<CausalStorage>,
    /// The distinction engine (for advanced operations)
    engine: Arc<DistinctionEngine>,
    /// View manager for materialized views
    views: Arc<ViewManager>,
    /// Subscription manager for change notifications
    subscriptions: Arc<SubscriptionManager>,
}

impl std::fmt::Debug for KoruDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KoruDelta")
            .field("storage", &self.storage)
            .field("engine", &self.engine)
            .finish()
    }
}

impl KoruDelta {
    /// Start a new KoruDelta instance.
    ///
    /// This is the zero-configuration entry point. No config files,
    /// no setup rituals - just start and use.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let db = KoruDelta::start().await?;
    /// ```
    pub async fn start() -> DeltaResult<Self> {
        let engine = Arc::new(DistinctionEngine::new());
        let storage = Arc::new(CausalStorage::new(Arc::clone(&engine)));
        let views = Arc::new(ViewManager::new(Arc::clone(&storage)));
        let subscriptions = Arc::new(SubscriptionManager::new());

        Ok(Self {
            storage,
            engine,
            views,
            subscriptions,
        })
    }

    /// Start a KoruDelta instance with a provided distinction engine.
    ///
    /// This is useful for testing or when you need fine-grained control
    /// over the underlying engine configuration.
    pub async fn start_with_engine(engine: Arc<DistinctionEngine>) -> DeltaResult<Self> {
        let storage = Arc::new(CausalStorage::new(Arc::clone(&engine)));
        let views = Arc::new(ViewManager::new(Arc::clone(&storage)));
        let subscriptions = Arc::new(SubscriptionManager::new());

        Ok(Self {
            storage,
            engine,
            views,
            subscriptions,
        })
    }

    /// Create a KoruDelta instance from existing storage and engine.
    ///
    /// This is used by the persistence layer to restore a database from disk.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = Arc::new(DistinctionEngine::new());
    /// let storage = Arc::new(persistence::load(path, engine.clone()).await?);
    /// let db = KoruDelta::from_storage(storage, engine);
    /// ```
    pub fn from_storage(storage: Arc<CausalStorage>, engine: Arc<DistinctionEngine>) -> Self {
        let views = Arc::new(ViewManager::new(Arc::clone(&storage)));
        let subscriptions = Arc::new(SubscriptionManager::new());

        Self {
            storage,
            engine,
            views,
            subscriptions,
        }
    }

    /// Get access to the internal storage for persistence operations.
    ///
    /// This is used by the CLI and other tools to save the database to disk.
    pub fn storage(&self) -> &Arc<CausalStorage> {
        &self.storage
    }

    /// Store a value in the database.
    ///
    /// This creates a new version in the causal history. The value is
    /// automatically timestamped and linked to its previous version.
    ///
    /// # Arguments
    ///
    /// - `namespace`: Logical grouping (like a table or collection)
    /// - `key`: Unique identifier within the namespace
    /// - `value`: Any JSON-serializable value
    ///
    /// # Returns
    ///
    /// Returns the versioned value that was stored, including:
    /// - Content-addressed version ID
    /// - Timestamp of the write
    /// - Link to previous version (if any)
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.put("users", "alice", json!({"name": "Alice"})).await?;
    /// db.put("counters", "visits", json!(42)).await?;
    /// db.put("config", "theme", json!("dark")).await?;
    /// ```
    pub async fn put<T: Serialize>(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: T,
    ) -> DeltaResult<VersionedValue> {
        let json_value = serde_json::to_value(value).map_err(DeltaError::SerializationError)?;

        self.storage.put(namespace, key, json_value)
    }

    /// Retrieve the current value for a key.
    ///
    /// Returns the most recent version, or an error if the key doesn't exist.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = db.get("users", "alice").await?;
    /// println!("Name: {}", user.value()["name"]);
    /// ```
    pub async fn get(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<JsonValue> {
        let versioned = self.storage.get(namespace, key)?;
        Ok(versioned.value().clone())
    }

    /// Get the full versioned value (including metadata).
    ///
    /// Unlike `get()` which returns just the JSON value, this returns
    /// the complete VersionedValue with timestamp, version ID, and
    /// previous version link.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let versioned = db.get_versioned("users", "alice").await?;
    /// println!("Version ID: {}", versioned.version_id());
    /// println!("Timestamp: {}", versioned.timestamp());
    /// println!("Previous: {:?}", versioned.previous_version());
    /// ```
    pub async fn get_versioned(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<VersionedValue> {
        self.storage.get(namespace, key)
    }

    /// Time travel: Get the value at a specific point in time.
    ///
    /// This traverses the causal history to find the most recent version
    /// at or before the given timestamp.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chrono::DateTime;
    ///
    /// let timestamp = DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")?
    ///     .with_timezone(&Utc);
    /// let past_value = db.get_at("users", "alice", timestamp).await?;
    /// ```
    pub async fn get_at(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> DeltaResult<JsonValue> {
        let versioned = self.storage.get_at(namespace, key, timestamp)?;
        Ok(versioned.value().clone())
    }

    /// Get the complete history for a key.
    ///
    /// Returns all versions that have ever been written, in chronological
    /// order (oldest to newest). This enables full audit trails and
    /// time-series analysis.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let history = db.history("counters", "visits").await?;
    /// for entry in history {
    ///     println!("{}: {}", entry.timestamp, entry.value);
    /// }
    /// ```
    pub async fn history(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        self.storage.history(namespace, key)
    }

    /// Check if a key exists.
    ///
    /// This is a lightweight operation that only checks the current state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if db.contains("users", "alice").await {
    ///     println!("User exists");
    /// }
    /// ```
    pub async fn contains(&self, namespace: impl Into<String>, key: impl Into<String>) -> bool {
        self.storage.contains_key(namespace, key)
    }

    /// Get statistics about the database.
    ///
    /// Returns useful metrics like number of keys, total versions, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let stats = db.stats().await;
    /// println!("Keys: {}", stats.key_count);
    /// println!("Versions: {}", stats.total_versions);
    /// ```
    pub async fn stats(&self) -> DatabaseStats {
        DatabaseStats {
            key_count: self.storage.key_count(),
            total_versions: self.storage.total_version_count(),
            namespace_count: self.storage.list_namespaces().len(),
        }
    }

    /// List all namespaces currently in use.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let namespaces = db.list_namespaces().await;
    /// for ns in namespaces {
    ///     println!("Namespace: {}", ns);
    /// }
    /// ```
    pub async fn list_namespaces(&self) -> Vec<String> {
        self.storage.list_namespaces()
    }

    /// List all keys in a specific namespace.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.list_keys("users").await;
    /// for user_key in users {
    ///     println!("User: {}", user_key);
    /// }
    /// ```
    pub async fn list_keys(&self, namespace: &str) -> Vec<String> {
        self.storage.list_keys(namespace)
    }

    /// Get access to the underlying distinction engine.
    ///
    /// This is provided for advanced use cases where you need direct
    /// access to the engine for custom operations. Most users won't
    /// need this.
    pub fn engine(&self) -> &Arc<DistinctionEngine> {
        &self.engine
    }

    // =========================================================================
    // Query API (Phase 3)
    // =========================================================================

    /// Execute a query against a collection.
    ///
    /// Returns records matching the query criteria, with optional
    /// filtering, projection, sorting, and aggregation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use koru_delta::query::{Query, Filter};
    ///
    /// let query = Query::new()
    ///     .filter(Filter::gt("age", 30))
    ///     .sort_by("name", true)
    ///     .limit(10);
    ///
    /// let results = db.query("users", query).await?;
    /// for record in results.records {
    ///     println!("{}: {:?}", record.key, record.value);
    /// }
    /// ```
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

    /// Execute a history query against a key's versions.
    ///
    /// Queries across the version history of a specific key.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use koru_delta::query::{HistoryQuery, Query, Filter};
    ///
    /// let query = HistoryQuery::new()
    ///     .with_query(Query::new().filter(Filter::gt("count", 10)))
    ///     .latest(5);
    ///
    /// let history = db.query_history("counters", "visits", query).await?;
    /// ```
    pub async fn query_history(
        &self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        query: HistoryQuery,
    ) -> DeltaResult<Vec<HistoryEntry>> {
        let history = self.storage.history(namespace, key)?;
        QueryExecutor::execute_history(&query, history)
    }

    // =========================================================================
    // Views API (Phase 3)
    // =========================================================================

    /// Create a materialized view.
    ///
    /// Materialized views cache query results for fast access.
    /// They can be refreshed on demand or automatically.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use koru_delta::views::ViewDefinition;
    /// use koru_delta::query::{Query, Filter};
    ///
    /// let view = ViewDefinition::new("active_users", "users")
    ///     .with_query(Query::new().filter(Filter::eq("status", "active")))
    ///     .with_description("All active users");
    ///
    /// let info = db.create_view(view).await?;
    /// println!("Created view with {} records", info.record_count);
    /// ```
    pub async fn create_view(&self, definition: ViewDefinition) -> DeltaResult<ViewInfo> {
        self.views.create_view(definition)
    }

    /// List all materialized views.
    pub async fn list_views(&self) -> Vec<ViewInfo> {
        self.views.list_views()
    }

    /// Refresh a materialized view.
    ///
    /// Re-executes the view's query and updates the cached results.
    pub async fn refresh_view(&self, name: &str) -> DeltaResult<ViewInfo> {
        self.views.refresh_view(name)
    }

    /// Query a materialized view.
    ///
    /// Returns the cached results from the view.
    pub async fn query_view(&self, name: &str) -> DeltaResult<QueryResult> {
        self.views.query_view(name)
    }

    /// Delete a materialized view.
    pub async fn delete_view(&self, name: &str) -> DeltaResult<()> {
        self.views.delete_view(name)
    }

    /// Get access to the view manager.
    pub fn view_manager(&self) -> &Arc<ViewManager> {
        &self.views
    }

    // =========================================================================
    // Subscriptions API (Phase 3)
    // =========================================================================

    /// Subscribe to changes.
    ///
    /// Returns a subscription ID and a receiver for change events.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use koru_delta::subscriptions::Subscription;
    ///
    /// let (sub_id, mut rx) = db.subscribe(Subscription::collection("users")).await;
    ///
    /// // In an async context:
    /// while let Some(event) = rx.recv().await {
    ///     println!("Change: {:?}", event);
    /// }
    /// ```
    pub async fn subscribe(
        &self,
        subscription: Subscription,
    ) -> (SubscriptionId, broadcast::Receiver<ChangeEvent>) {
        self.subscriptions.subscribe(subscription)
    }

    /// Unsubscribe from changes.
    pub async fn unsubscribe(&self, id: SubscriptionId) -> DeltaResult<()> {
        self.subscriptions.unsubscribe(id)
    }

    /// List all active subscriptions.
    pub async fn list_subscriptions(&self) -> Vec<crate::subscriptions::SubscriptionInfo> {
        self.subscriptions.list_subscriptions()
    }

    /// Get access to the subscription manager.
    pub fn subscription_manager(&self) -> &Arc<SubscriptionManager> {
        &self.subscriptions
    }

    /// Store a value and notify subscribers.
    ///
    /// This is like `put()` but also triggers subscription notifications.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // First subscribe
    /// let (_, mut rx) = db.subscribe(Subscription::collection("users")).await;
    ///
    /// // Then write with notifications
    /// db.put_notify("users", "alice", json!({"name": "Alice"})).await?;
    ///
    /// // Receive the notification
    /// let event = rx.recv().await?;
    /// ```
    pub async fn put_notify<T: Serialize>(
        &self,
        namespace: impl Into<String> + Clone,
        key: impl Into<String> + Clone,
        value: T,
    ) -> DeltaResult<VersionedValue> {
        let ns = namespace.clone().into();
        let k = key.clone().into();

        // Check if this is an update.
        let previous = self.storage.get(&ns, &k).ok();

        // Perform the write.
        let result = self.put(namespace, key, value).await?;

        // Notify subscribers.
        if let Some(prev) = previous {
            self.subscriptions.notify_update(&ns, &k, &result, &prev);
        } else {
            self.subscriptions.notify_insert(&ns, &k, &result);
        }

        // Notify views that may need auto-refresh.
        let _ = self.views.on_write(&ns, &k);

        Ok(result)
    }
}

/// Database statistics.
///
/// Provides a snapshot of the current database state for monitoring
/// and debugging purposes.
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseStats {
    /// Number of unique keys across all namespaces
    pub key_count: usize,
    /// Total number of versions (including all history)
    pub total_versions: usize,
    /// Number of unique namespaces
    pub namespace_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_start() {
        let db = KoruDelta::start().await;
        assert!(db.is_ok());
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let db = KoruDelta::start().await.unwrap();
        let value = json!({"name": "Alice", "age": 30});

        db.put("users", "alice", value.clone()).await.unwrap();
        let retrieved = db.get("users", "alice").await.unwrap();

        assert_eq!(retrieved, value);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let db = KoruDelta::start().await.unwrap();

        let result = db.get("users", "nonexistent").await;
        assert!(matches!(result, Err(DeltaError::KeyNotFound { .. })));
    }

    #[tokio::test]
    async fn test_versioned_get() {
        let db = KoruDelta::start().await.unwrap();

        db.put("users", "alice", json!({"age": 30})).await.unwrap();
        let versioned = db.get_versioned("users", "alice").await.unwrap();

        assert_eq!(versioned.value(), &json!({"age": 30}));
        assert!(!versioned.version_id().is_empty());
        assert!(versioned.previous_version().is_none()); // First version
    }

    #[tokio::test]
    async fn test_history() {
        let db = KoruDelta::start().await.unwrap();

        db.put("counter", "clicks", json!(1)).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        db.put("counter", "clicks", json!(2)).await.unwrap();
        sleep(Duration::from_millis(10)).await;
        db.put("counter", "clicks", json!(3)).await.unwrap();

        let history = db.history("counter", "clicks").await.unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].value, json!(1));
        assert_eq!(history[1].value, json!(2));
        assert_eq!(history[2].value, json!(3));
    }

    #[tokio::test]
    async fn test_time_travel() {
        let db = KoruDelta::start().await.unwrap();

        let v1 = db
            .put("doc", "readme", json!({"version": 1}))
            .await
            .unwrap();
        let t1 = v1.timestamp();

        sleep(Duration::from_millis(50)).await;
        let v2 = db
            .put("doc", "readme", json!({"version": 2}))
            .await
            .unwrap();
        let t2 = v2.timestamp();

        sleep(Duration::from_millis(50)).await;
        db.put("doc", "readme", json!({"version": 3}))
            .await
            .unwrap();

        // Time travel to t1
        let v_at_t1 = db.get_at("doc", "readme", t1).await.unwrap();
        assert_eq!(v_at_t1, json!({"version": 1}));

        // Time travel to t2
        let v_at_t2 = db.get_at("doc", "readme", t2).await.unwrap();
        assert_eq!(v_at_t2, json!({"version": 2}));
    }

    #[tokio::test]
    async fn test_contains() {
        let db = KoruDelta::start().await.unwrap();

        assert!(!db.contains("users", "alice").await);
        db.put("users", "alice", json!({})).await.unwrap();
        assert!(db.contains("users", "alice").await);
    }

    #[tokio::test]
    async fn test_stats() {
        let db = KoruDelta::start().await.unwrap();

        let stats1 = db.stats().await;
        assert_eq!(stats1.key_count, 0);
        assert_eq!(stats1.total_versions, 0);

        db.put("users", "alice", json!(1)).await.unwrap();
        db.put("users", "alice", json!(2)).await.unwrap();
        db.put("users", "bob", json!(1)).await.unwrap();

        let stats2 = db.stats().await;
        assert_eq!(stats2.key_count, 2);
        assert_eq!(stats2.total_versions, 3);
        assert_eq!(stats2.namespace_count, 1);
    }

    #[tokio::test]
    async fn test_list_namespaces() {
        let db = KoruDelta::start().await.unwrap();

        db.put("users", "alice", json!({})).await.unwrap();
        db.put("sessions", "s1", json!({})).await.unwrap();
        db.put("config", "app", json!({})).await.unwrap();

        let namespaces = db.list_namespaces().await;
        assert_eq!(namespaces, vec!["config", "sessions", "users"]);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let db = KoruDelta::start().await.unwrap();

        db.put("users", "alice", json!({})).await.unwrap();
        db.put("users", "bob", json!({})).await.unwrap();
        db.put("users", "charlie", json!({})).await.unwrap();

        let keys = db.list_keys("users").await;
        assert_eq!(keys, vec!["alice", "bob", "charlie"]);
    }

    #[tokio::test]
    async fn test_complex_json_types() {
        let db = KoruDelta::start().await.unwrap();

        // Test various JSON types
        db.put("data", "string", "Hello").await.unwrap();
        db.put("data", "number", 42).await.unwrap();
        db.put("data", "bool", true).await.unwrap();
        db.put("data", "array", vec![1, 2, 3]).await.unwrap();
        db.put("data", "object", json!({"key": "value"}))
            .await
            .unwrap();

        assert_eq!(db.get("data", "string").await.unwrap(), json!("Hello"));
        assert_eq!(db.get("data", "number").await.unwrap(), json!(42));
        assert_eq!(db.get("data", "bool").await.unwrap(), json!(true));
        assert_eq!(db.get("data", "array").await.unwrap(), json!([1, 2, 3]));
        assert_eq!(
            db.get("data", "object").await.unwrap(),
            json!({"key": "value"})
        );
    }

    #[tokio::test]
    async fn test_clone_and_share() {
        let db = KoruDelta::start().await.unwrap();
        let db_clone = db.clone();

        // Write with original
        db.put("data", "key1", json!(1)).await.unwrap();

        // Read with clone
        let value = db_clone.get("data", "key1").await.unwrap();
        assert_eq!(value, json!(1));

        // Write with clone
        db_clone.put("data", "key2", json!(2)).await.unwrap();

        // Read with original
        let value = db.get("data", "key2").await.unwrap();
        assert_eq!(value, json!(2));
    }
}
