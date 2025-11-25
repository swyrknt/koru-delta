/// Materialized views for KoruDelta.
///
/// This module provides materialized views that automatically maintain
/// pre-computed query results. Views can be:
///
/// - **Created** from a query definition
/// - **Refreshed** on demand or automatically
/// - **Queried** directly for fast access to computed results
///
/// # Example
///
/// ```ignore
/// use koru_delta::views::{ViewDefinition, ViewManager};
/// use koru_delta::query::{Query, Filter, Aggregation};
///
/// // Create a view for active users over 21
/// let view_def = ViewDefinition::new("active_adults", "users")
///     .with_query(Query::new()
///         .filter(Filter::eq("status", "active"))
///         .filter(Filter::gte("age", 21)));
///
/// let manager = ViewManager::new(storage);
/// manager.create_view(view_def)?;
///
/// // Query the view
/// let results = manager.query_view("active_adults")?;
/// ```
use crate::error::{DeltaError, DeltaResult};
use crate::query::{Query, QueryExecutor, QueryRecord, QueryResult};
use crate::storage::CausalStorage;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Definition of a materialized view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewDefinition {
    /// Unique name of the view.
    pub name: String,
    /// Source collection to query.
    pub source_collection: String,
    /// Query to execute for this view.
    pub query: Query,
    /// When this view was created.
    pub created_at: DateTime<Utc>,
    /// Optional description.
    pub description: Option<String>,
    /// Whether this view auto-refreshes on writes.
    pub auto_refresh: bool,
}

impl ViewDefinition {
    /// Create a new view definition.
    pub fn new(name: impl Into<String>, source_collection: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source_collection: source_collection.into(),
            query: Query::new(),
            created_at: Utc::now(),
            description: None,
            auto_refresh: false,
        }
    }

    /// Set the query for this view.
    pub fn with_query(mut self, query: Query) -> Self {
        self.query = query;
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Enable auto-refresh on writes.
    pub fn auto_refresh(mut self, enabled: bool) -> Self {
        self.auto_refresh = enabled;
        self
    }
}

/// Cached view data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewData {
    /// The view definition.
    pub definition: ViewDefinition,
    /// Cached records.
    pub records: Vec<QueryRecord>,
    /// When this cache was last refreshed.
    pub last_refreshed: DateTime<Utc>,
    /// Total record count.
    pub total_count: usize,
}

impl ViewData {
    /// Create new view data from a definition and query result.
    pub fn from_result(definition: ViewDefinition, result: QueryResult) -> Self {
        Self {
            definition,
            records: result.records,
            last_refreshed: Utc::now(),
            total_count: result.total_count,
        }
    }

    /// Check if the view needs refresh based on age.
    pub fn needs_refresh(&self, max_age: chrono::Duration) -> bool {
        Utc::now() - self.last_refreshed > max_age
    }
}

/// Information about a view (for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewInfo {
    /// Name of the view.
    pub name: String,
    /// Source collection.
    pub source_collection: String,
    /// Description if available.
    pub description: Option<String>,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last refreshed.
    pub last_refreshed: DateTime<Utc>,
    /// Number of records in the view.
    pub record_count: usize,
    /// Whether auto-refresh is enabled.
    pub auto_refresh: bool,
}

impl From<&ViewData> for ViewInfo {
    fn from(data: &ViewData) -> Self {
        Self {
            name: data.definition.name.clone(),
            source_collection: data.definition.source_collection.clone(),
            description: data.definition.description.clone(),
            created_at: data.definition.created_at,
            last_refreshed: data.last_refreshed,
            record_count: data.records.len(),
            auto_refresh: data.definition.auto_refresh,
        }
    }
}

/// Manager for materialized views.
///
/// Handles creation, refresh, and querying of views.
pub struct ViewManager {
    storage: Arc<CausalStorage>,
    views: DashMap<String, ViewData>,
}

impl ViewManager {
    /// Create a new view manager.
    pub fn new(storage: Arc<CausalStorage>) -> Self {
        Self {
            storage,
            views: DashMap::new(),
        }
    }

    /// Create a new view.
    pub fn create_view(&self, definition: ViewDefinition) -> DeltaResult<ViewInfo> {
        let name = definition.name.clone();

        // Check if view already exists.
        if self.views.contains_key(&name) {
            return Err(DeltaError::StorageError(format!(
                "View '{}' already exists",
                name
            )));
        }

        // Execute the query to populate the view.
        let result = self.execute_view_query(&definition)?;

        // Store the view.
        let view_data = ViewData::from_result(definition, result);
        let info = ViewInfo::from(&view_data);
        self.views.insert(name, view_data);

        Ok(info)
    }

    /// Get a view by name.
    pub fn get_view(&self, name: &str) -> Option<ViewData> {
        self.views.get(name).map(|v| v.value().clone())
    }

    /// List all views.
    pub fn list_views(&self) -> Vec<ViewInfo> {
        self.views
            .iter()
            .map(|entry| ViewInfo::from(entry.value()))
            .collect()
    }

    /// Refresh a view.
    pub fn refresh_view(&self, name: &str) -> DeltaResult<ViewInfo> {
        let mut entry = self
            .views
            .get_mut(name)
            .ok_or_else(|| DeltaError::StorageError(format!("View '{}' not found", name)))?;

        let definition = entry.definition.clone();
        let result = self.execute_view_query(&definition)?;

        // Update the cached data.
        entry.records = result.records;
        entry.total_count = result.total_count;
        entry.last_refreshed = Utc::now();

        Ok(ViewInfo::from(entry.value()))
    }

    /// Refresh all views.
    pub fn refresh_all(&self) -> DeltaResult<Vec<ViewInfo>> {
        let names: Vec<String> = self.views.iter().map(|e| e.key().clone()).collect();
        let mut results = Vec::new();

        for name in names {
            results.push(self.refresh_view(&name)?);
        }

        Ok(results)
    }

    /// Refresh views that need updating based on max age.
    pub fn refresh_stale(&self, max_age: chrono::Duration) -> DeltaResult<Vec<ViewInfo>> {
        let stale: Vec<String> = self
            .views
            .iter()
            .filter(|entry| entry.value().needs_refresh(max_age))
            .map(|entry| entry.key().clone())
            .collect();

        let mut results = Vec::new();
        for name in stale {
            results.push(self.refresh_view(&name)?);
        }

        Ok(results)
    }

    /// Query a view.
    pub fn query_view(&self, name: &str) -> DeltaResult<QueryResult> {
        let view = self
            .views
            .get(name)
            .ok_or_else(|| DeltaError::StorageError(format!("View '{}' not found", name)))?;

        Ok(QueryResult {
            records: view.records.clone(),
            total_count: view.total_count,
            aggregation: None,
        })
    }

    /// Query a view with additional filtering.
    pub fn query_view_with_filter(&self, name: &str, query: &Query) -> DeltaResult<QueryResult> {
        let view = self
            .views
            .get(name)
            .ok_or_else(|| DeltaError::StorageError(format!("View '{}' not found", name)))?;

        // Apply additional filtering to the cached records.
        let items = view.records.iter().map(|r| {
            (
                r.key.clone(),
                r.value.clone(),
                r.timestamp,
                r.version_id.clone(),
            )
        });

        QueryExecutor::execute(query, items)
    }

    /// Delete a view.
    pub fn delete_view(&self, name: &str) -> DeltaResult<()> {
        self.views
            .remove(name)
            .ok_or_else(|| DeltaError::StorageError(format!("View '{}' not found", name)))?;
        Ok(())
    }

    /// Check if a view exists.
    pub fn view_exists(&self, name: &str) -> bool {
        self.views.contains_key(name)
    }

    /// Get the number of views.
    pub fn view_count(&self) -> usize {
        self.views.len()
    }

    /// Notify the manager of a write to refresh auto-refresh views.
    pub fn on_write(&self, collection: &str, _key: &str) -> DeltaResult<()> {
        let auto_refresh_views: Vec<String> = self
            .views
            .iter()
            .filter(|entry| {
                entry.value().definition.auto_refresh
                    && entry.value().definition.source_collection == collection
            })
            .map(|entry| entry.key().clone())
            .collect();

        for name in auto_refresh_views {
            self.refresh_view(&name)?;
        }

        Ok(())
    }

    /// Execute the query for a view definition.
    fn execute_view_query(&self, definition: &ViewDefinition) -> DeltaResult<QueryResult> {
        // Get all items from the source collection.
        let items = self
            .storage
            .scan_collection(&definition.source_collection)
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    value.value().clone(),
                    value.timestamp(),
                    value.version_id().to_string(),
                )
            });

        QueryExecutor::execute(&definition.query, items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{Aggregation, Filter};
    use koru_lambda_core::DistinctionEngine;
    use serde_json::json;

    fn create_test_storage() -> Arc<CausalStorage> {
        let engine = Arc::new(DistinctionEngine::new());
        Arc::new(CausalStorage::new(engine))
    }

    #[test]
    fn test_view_creation() {
        let storage = create_test_storage();

        // Add some test data.
        storage
            .put(
                "users",
                "alice",
                json!({"name": "Alice", "age": 30, "status": "active"}),
            )
            .unwrap();
        storage
            .put(
                "users",
                "bob",
                json!({"name": "Bob", "age": 25, "status": "inactive"}),
            )
            .unwrap();
        storage
            .put(
                "users",
                "charlie",
                json!({"name": "Charlie", "age": 35, "status": "active"}),
            )
            .unwrap();

        let manager = ViewManager::new(storage);

        // Create a view for active users.
        let definition = ViewDefinition::new("active_users", "users")
            .with_query(Query::new().filter(Filter::eq("status", json!("active"))))
            .with_description("All active users");

        let info = manager.create_view(definition).unwrap();

        assert_eq!(info.name, "active_users");
        assert_eq!(info.record_count, 2); // Alice and Charlie
    }

    #[test]
    fn test_view_query() {
        let storage = create_test_storage();

        storage
            .put(
                "products",
                "p1",
                json!({"name": "Widget", "price": 10.0, "category": "tools"}),
            )
            .unwrap();
        storage
            .put(
                "products",
                "p2",
                json!({"name": "Gadget", "price": 20.0, "category": "electronics"}),
            )
            .unwrap();
        storage
            .put(
                "products",
                "p3",
                json!({"name": "Sprocket", "price": 15.0, "category": "tools"}),
            )
            .unwrap();

        let manager = ViewManager::new(storage);

        let definition = ViewDefinition::new("tools_view", "products")
            .with_query(Query::new().filter(Filter::eq("category", json!("tools"))));

        manager.create_view(definition).unwrap();

        let result = manager.query_view("tools_view").unwrap();
        assert_eq!(result.records.len(), 2);
    }

    #[test]
    fn test_view_refresh() {
        let storage = create_test_storage();

        storage.put("items", "a", json!({"value": 1})).unwrap();

        let manager = ViewManager::new(storage.clone());

        let definition = ViewDefinition::new("all_items", "items");
        manager.create_view(definition).unwrap();

        // Initially one record.
        let result = manager.query_view("all_items").unwrap();
        assert_eq!(result.records.len(), 1);

        // Add more data.
        storage.put("items", "b", json!({"value": 2})).unwrap();
        storage.put("items", "c", json!({"value": 3})).unwrap();

        // Still one record until refresh.
        let result = manager.query_view("all_items").unwrap();
        assert_eq!(result.records.len(), 1);

        // Refresh.
        manager.refresh_view("all_items").unwrap();

        // Now three records.
        let result = manager.query_view("all_items").unwrap();
        assert_eq!(result.records.len(), 3);
    }

    #[test]
    fn test_view_list_and_delete() {
        let storage = create_test_storage();

        storage.put("data", "x", json!(1)).unwrap();

        let manager = ViewManager::new(storage);

        manager
            .create_view(ViewDefinition::new("view1", "data"))
            .unwrap();
        manager
            .create_view(ViewDefinition::new("view2", "data"))
            .unwrap();
        manager
            .create_view(ViewDefinition::new("view3", "data"))
            .unwrap();

        assert_eq!(manager.view_count(), 3);

        let views = manager.list_views();
        assert_eq!(views.len(), 3);

        manager.delete_view("view2").unwrap();
        assert_eq!(manager.view_count(), 2);
        assert!(!manager.view_exists("view2"));
    }

    #[test]
    fn test_duplicate_view_error() {
        let storage = create_test_storage();
        storage.put("data", "x", json!(1)).unwrap();

        let manager = ViewManager::new(storage);

        manager
            .create_view(ViewDefinition::new("myview", "data"))
            .unwrap();

        let result = manager.create_view(ViewDefinition::new("myview", "data"));
        assert!(result.is_err());
    }

    #[test]
    fn test_view_with_aggregation() {
        let storage = create_test_storage();

        storage
            .put("sales", "s1", json!({"amount": 100, "region": "north"}))
            .unwrap();
        storage
            .put("sales", "s2", json!({"amount": 200, "region": "south"}))
            .unwrap();
        storage
            .put("sales", "s3", json!({"amount": 150, "region": "north"}))
            .unwrap();

        let manager = ViewManager::new(storage);

        let definition = ViewDefinition::new("north_sales", "sales").with_query(
            Query::new()
                .filter(Filter::eq("region", json!("north")))
                .aggregate(Aggregation::sum("amount")),
        );

        manager.create_view(definition).unwrap();

        let result = manager.query_view("north_sales").unwrap();
        assert_eq!(result.records.len(), 2); // Two north sales
    }

    #[test]
    fn test_auto_refresh() {
        let storage = create_test_storage();

        storage.put("counters", "c1", json!({"value": 1})).unwrap();

        let manager = ViewManager::new(storage.clone());

        let definition = ViewDefinition::new("counters_view", "counters").auto_refresh(true);

        manager.create_view(definition).unwrap();

        // Initially one record.
        let result = manager.query_view("counters_view").unwrap();
        assert_eq!(result.records.len(), 1);

        // Add data and trigger on_write.
        storage.put("counters", "c2", json!({"value": 2})).unwrap();
        manager.on_write("counters", "c2").unwrap();

        // Should now have two records.
        let result = manager.query_view("counters_view").unwrap();
        assert_eq!(result.records.len(), 2);
    }

    #[test]
    fn test_query_view_with_additional_filter() {
        let storage = create_test_storage();

        storage
            .put(
                "users",
                "u1",
                json!({"name": "Alice", "age": 30, "active": true}),
            )
            .unwrap();
        storage
            .put(
                "users",
                "u2",
                json!({"name": "Bob", "age": 25, "active": true}),
            )
            .unwrap();
        storage
            .put(
                "users",
                "u3",
                json!({"name": "Charlie", "age": 35, "active": false}),
            )
            .unwrap();

        let manager = ViewManager::new(storage);

        // Create a view of all users.
        let definition = ViewDefinition::new("all_users", "users");
        manager.create_view(definition).unwrap();

        // Query view with additional filter.
        let additional_filter = Query::new().filter(Filter::gte("age", json!(30)));
        let result = manager
            .query_view_with_filter("all_users", &additional_filter)
            .unwrap();

        assert_eq!(result.records.len(), 2); // Alice and Charlie
    }
}
