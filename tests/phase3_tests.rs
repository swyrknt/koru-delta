/// Integration tests for KoruDelta Phase 3: Queries, Views, and Subscriptions.
///
/// These tests verify the Phase 3 functionality including:
/// - Query API (filter, sort, aggregate)
/// - Materialized views (create, refresh, query)
/// - Subscriptions (real-time change notifications)
use koru_delta::prelude::*;
use koru_delta::query::{Aggregation, Filter, HistoryQuery, Query};
use koru_delta::subscriptions::{ChangeType, Subscription};
use koru_delta::views::ViewDefinition;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Query Tests
// ============================================================================

#[tokio::test]
async fn test_basic_query() {
    let db = KoruDelta::start().await.unwrap();

    // Add test data.
    db.put(
        "users",
        "alice",
        json!({"name": "Alice", "age": 30, "status": "active"}),
    )
    .await
    .unwrap();
    db.put(
        "users",
        "bob",
        json!({"name": "Bob", "age": 25, "status": "inactive"}),
    )
    .await
    .unwrap();
    db.put(
        "users",
        "charlie",
        json!({"name": "Charlie", "age": 35, "status": "active"}),
    )
    .await
    .unwrap();

    // Query all users.
    let result = db.query("users", Query::new()).await.unwrap();
    assert_eq!(result.records.len(), 3);
}

#[tokio::test]
async fn test_query_with_filter() {
    let db = KoruDelta::start().await.unwrap();

    db.put("users", "alice", json!({"name": "Alice", "age": 30}))
        .await
        .unwrap();
    db.put("users", "bob", json!({"name": "Bob", "age": 25}))
        .await
        .unwrap();
    db.put("users", "charlie", json!({"name": "Charlie", "age": 35}))
        .await
        .unwrap();

    // Query users over 28.
    let result = db
        .query("users", Query::new().filter(Filter::gt("age", json!(28))))
        .await
        .unwrap();

    assert_eq!(result.records.len(), 2);
}

#[tokio::test]
async fn test_query_with_sort() {
    let db = KoruDelta::start().await.unwrap();

    db.put("users", "alice", json!({"name": "Alice", "age": 30}))
        .await
        .unwrap();
    db.put("users", "bob", json!({"name": "Bob", "age": 25}))
        .await
        .unwrap();
    db.put("users", "charlie", json!({"name": "Charlie", "age": 35}))
        .await
        .unwrap();

    // Query users sorted by age ascending.
    let result = db
        .query("users", Query::new().sort_by("age", true))
        .await
        .unwrap();

    assert_eq!(result.records.len(), 3);
    assert_eq!(result.records[0].key, "bob"); // Youngest first
    assert_eq!(result.records[2].key, "charlie"); // Oldest last
}

#[tokio::test]
async fn test_query_with_limit() {
    let db = KoruDelta::start().await.unwrap();

    for i in 0..10 {
        db.put("items", format!("item{}", i), json!({"value": i}))
            .await
            .unwrap();
    }

    // Query with limit.
    let result = db.query("items", Query::new().limit(5)).await.unwrap();

    assert_eq!(result.records.len(), 5);
    assert_eq!(result.total_count, 10);
}

#[tokio::test]
async fn test_query_aggregation_count() {
    let db = KoruDelta::start().await.unwrap();

    for i in 0..5 {
        db.put("counters", format!("c{}", i), json!({"value": i}))
            .await
            .unwrap();
    }

    let result = db
        .query("counters", Query::new().aggregate(Aggregation::count()))
        .await
        .unwrap();

    assert_eq!(result.aggregation, Some(json!(5)));
}

#[tokio::test]
async fn test_query_aggregation_sum() {
    let db = KoruDelta::start().await.unwrap();

    db.put("sales", "s1", json!({"amount": 100})).await.unwrap();
    db.put("sales", "s2", json!({"amount": 200})).await.unwrap();
    db.put("sales", "s3", json!({"amount": 300})).await.unwrap();

    let result = db
        .query("sales", Query::new().aggregate(Aggregation::sum("amount")))
        .await
        .unwrap();

    assert_eq!(result.aggregation, Some(json!(600.0)));
}

#[tokio::test]
async fn test_query_combined() {
    let db = KoruDelta::start().await.unwrap();

    db.put(
        "products",
        "p1",
        json!({"name": "Widget", "price": 10.0, "in_stock": true}),
    )
    .await
    .unwrap();
    db.put(
        "products",
        "p2",
        json!({"name": "Gadget", "price": 25.0, "in_stock": false}),
    )
    .await
    .unwrap();
    db.put(
        "products",
        "p3",
        json!({"name": "Sprocket", "price": 15.0, "in_stock": true}),
    )
    .await
    .unwrap();
    db.put(
        "products",
        "p4",
        json!({"name": "Gizmo", "price": 30.0, "in_stock": true}),
    )
    .await
    .unwrap();

    // Filter in_stock, sort by price descending, limit to 2.
    let result = db
        .query(
            "products",
            Query::new()
                .filter(Filter::eq("in_stock", json!(true)))
                .sort_by("price", false) // descending
                .limit(2),
        )
        .await
        .unwrap();

    assert_eq!(result.records.len(), 2);
    assert_eq!(result.records[0].key, "p4"); // Gizmo $30 (highest)
    assert_eq!(result.records[1].key, "p3"); // Sprocket $15
}

#[tokio::test]
async fn test_history_query() {
    let db = KoruDelta::start().await.unwrap();

    // Create multiple versions.
    db.put("counter", "clicks", json!({"count": 1}))
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    db.put("counter", "clicks", json!({"count": 5}))
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    db.put("counter", "clicks", json!({"count": 10}))
        .await
        .unwrap();

    // Query history for versions with count > 3.
    let query = HistoryQuery::new().with_query(Query::new().filter(Filter::gt("count", json!(3))));

    let results = db.query_history("counter", "clicks", query).await.unwrap();

    assert_eq!(results.len(), 2); // count 5 and count 10
}

// ============================================================================
// View Tests
// ============================================================================

#[tokio::test]
async fn test_view_creation() {
    let db = KoruDelta::start().await.unwrap();

    db.put(
        "users",
        "alice",
        json!({"name": "Alice", "status": "active"}),
    )
    .await
    .unwrap();
    db.put("users", "bob", json!({"name": "Bob", "status": "inactive"}))
        .await
        .unwrap();
    db.put(
        "users",
        "charlie",
        json!({"name": "Charlie", "status": "active"}),
    )
    .await
    .unwrap();

    // Create a view for active users.
    let definition = ViewDefinition::new("active_users", "users")
        .with_query(Query::new().filter(Filter::eq("status", json!("active"))))
        .with_description("All active users");

    let info = db.create_view(definition).await.unwrap();

    assert_eq!(info.name, "active_users");
    assert_eq!(info.record_count, 2); // Alice and Charlie
}

#[tokio::test]
async fn test_view_query() {
    let db = KoruDelta::start().await.unwrap();

    db.put(
        "products",
        "p1",
        json!({"category": "electronics", "price": 100}),
    )
    .await
    .unwrap();
    db.put("products", "p2", json!({"category": "books", "price": 20}))
        .await
        .unwrap();
    db.put(
        "products",
        "p3",
        json!({"category": "electronics", "price": 150}),
    )
    .await
    .unwrap();

    // Create view.
    let definition = ViewDefinition::new("electronics", "products")
        .with_query(Query::new().filter(Filter::eq("category", json!("electronics"))));

    db.create_view(definition).await.unwrap();

    // Query the view.
    let result = db.query_view("electronics").await.unwrap();

    assert_eq!(result.records.len(), 2);
}

#[tokio::test]
async fn test_view_refresh() {
    let db = KoruDelta::start().await.unwrap();

    db.put("items", "a", json!({"value": 1})).await.unwrap();

    // Create view.
    let definition = ViewDefinition::new("all_items", "items");
    db.create_view(definition).await.unwrap();

    // Initially one record.
    let result = db.query_view("all_items").await.unwrap();
    assert_eq!(result.records.len(), 1);

    // Add more data.
    db.put("items", "b", json!({"value": 2})).await.unwrap();
    db.put("items", "c", json!({"value": 3})).await.unwrap();

    // Still one record until refresh.
    let result = db.query_view("all_items").await.unwrap();
    assert_eq!(result.records.len(), 1);

    // Refresh view.
    db.refresh_view("all_items").await.unwrap();

    // Now three records.
    let result = db.query_view("all_items").await.unwrap();
    assert_eq!(result.records.len(), 3);
}

#[tokio::test]
async fn test_view_list() {
    let db = KoruDelta::start().await.unwrap();

    db.put("data", "x", json!(1)).await.unwrap();

    db.create_view(ViewDefinition::new("view1", "data"))
        .await
        .unwrap();
    db.create_view(ViewDefinition::new("view2", "data"))
        .await
        .unwrap();
    db.create_view(ViewDefinition::new("view3", "data"))
        .await
        .unwrap();

    let views = db.list_views().await;
    assert_eq!(views.len(), 3);
}

#[tokio::test]
async fn test_view_delete() {
    let db = KoruDelta::start().await.unwrap();

    db.put("data", "x", json!(1)).await.unwrap();

    db.create_view(ViewDefinition::new("myview", "data"))
        .await
        .unwrap();

    let views = db.list_views().await;
    assert_eq!(views.len(), 1);

    db.delete_view("myview").await.unwrap();

    let views = db.list_views().await;
    assert_eq!(views.len(), 0);
}

// ============================================================================
// Subscription Tests
// ============================================================================

#[tokio::test]
async fn test_subscription_basic() {
    let db = KoruDelta::start().await.unwrap();

    // Subscribe to all changes.
    let (_id, mut rx) = db.subscribe(Subscription::all()).await;

    // Write with notifications.
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    // Should receive notification.
    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(event.collection, "users");
    assert_eq!(event.key, "alice");
    assert_eq!(event.change_type, ChangeType::Insert);
}

#[tokio::test]
async fn test_subscription_collection_filter() {
    let db = KoruDelta::start().await.unwrap();

    // Subscribe only to users collection.
    let (_id, mut rx) = db.subscribe(Subscription::collection("users")).await;

    // Write to products (should not receive).
    db.put_notify("products", "widget", json!({"price": 10}))
        .await
        .unwrap();

    // Write to users (should receive).
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    // Should receive only the users event.
    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(event.collection, "users");
}

#[tokio::test]
async fn test_subscription_key_filter() {
    let db = KoruDelta::start().await.unwrap();

    // Subscribe to specific key.
    let (_id, mut rx) = db.subscribe(Subscription::key("users", "alice")).await;

    // Write to different key (should not receive).
    db.put_notify("users", "bob", json!({"name": "Bob"}))
        .await
        .unwrap();

    // Write to subscribed key (should receive).
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    // Should receive only the alice event.
    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(event.key, "alice");
}

#[tokio::test]
async fn test_subscription_update_event() {
    let db = KoruDelta::start().await.unwrap();

    let (_id, mut rx) = db.subscribe(Subscription::all()).await;

    // First insert.
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.change_type, ChangeType::Insert);

    // Update.
    db.put_notify("users", "alice", json!({"name": "Alice", "age": 30}))
        .await
        .unwrap();

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.change_type, ChangeType::Update);
    assert!(event.previous_value.is_some());
}

#[tokio::test]
async fn test_subscription_inserts_only() {
    let db = KoruDelta::start().await.unwrap();

    let (_id, mut rx) = db.subscribe(Subscription::all().inserts_only()).await;

    // First insert - should receive.
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.change_type, ChangeType::Insert);

    // Update - should NOT receive (inserts only).
    db.put_notify("users", "alice", json!({"name": "Alice Updated"}))
        .await
        .unwrap();

    // Timeout expected since we shouldn't receive the update.
    let result = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
    assert!(result.is_err()); // Timeout
}

#[tokio::test]
async fn test_subscription_unsubscribe() {
    let db = KoruDelta::start().await.unwrap();

    let (id, _rx) = db.subscribe(Subscription::all()).await;

    let subs = db.list_subscriptions().await;
    assert_eq!(subs.len(), 1);

    db.unsubscribe(id).await.unwrap();

    let subs = db.list_subscriptions().await;
    assert_eq!(subs.len(), 0);
}

#[tokio::test]
async fn test_multiple_subscriptions() {
    let db = KoruDelta::start().await.unwrap();

    let (_id1, mut rx1) = db.subscribe(Subscription::all()).await;
    let (_id2, mut rx2) = db.subscribe(Subscription::collection("users")).await;

    // Write to users.
    db.put_notify("users", "alice", json!({"name": "Alice"}))
        .await
        .unwrap();

    // Both should receive.
    let e1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
        .await
        .unwrap()
        .unwrap();
    let e2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(e1.key, "alice");
    assert_eq!(e2.key, "alice");
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[tokio::test]
async fn test_view_with_auto_refresh_on_write() {
    let db = KoruDelta::start().await.unwrap();

    db.put("items", "a", json!({"value": 1})).await.unwrap();

    // Create auto-refresh view.
    let definition = ViewDefinition::new("auto_items", "items").auto_refresh(true);
    db.create_view(definition).await.unwrap();

    // Initially one record.
    let result = db.query_view("auto_items").await.unwrap();
    assert_eq!(result.records.len(), 1);

    // Add data using put_notify (triggers auto-refresh).
    db.put_notify("items", "b", json!({"value": 2}))
        .await
        .unwrap();

    // Should now have two records.
    let result = db.query_view("auto_items").await.unwrap();
    assert_eq!(result.records.len(), 2);
}

#[tokio::test]
async fn test_query_on_empty_collection() {
    let db = KoruDelta::start().await.unwrap();

    let result = db.query("nonexistent", Query::new()).await.unwrap();

    assert_eq!(result.records.len(), 0);
    assert_eq!(result.total_count, 0);
}

#[tokio::test]
async fn test_query_projection() {
    let db = KoruDelta::start().await.unwrap();

    db.put(
        "users",
        "alice",
        json!({"name": "Alice", "age": 30, "email": "alice@example.com", "password": "secret"}),
    )
    .await
    .unwrap();

    // Query with projection.
    let result = db
        .query("users", Query::new().project(&["name", "email"]))
        .await
        .unwrap();

    assert_eq!(result.records.len(), 1);

    let value = &result.records[0].value;
    assert!(value.get("name").is_some());
    assert!(value.get("email").is_some());
    assert!(value.get("password").is_none()); // Not projected
    assert!(value.get("age").is_none()); // Not projected
}

#[tokio::test]
async fn test_complex_filter() {
    let db = KoruDelta::start().await.unwrap();

    db.put(
        "users",
        "u1",
        json!({"name": "Alice", "age": 25, "active": true}),
    )
    .await
    .unwrap();
    db.put(
        "users",
        "u2",
        json!({"name": "Bob", "age": 35, "active": false}),
    )
    .await
    .unwrap();
    db.put(
        "users",
        "u3",
        json!({"name": "Charlie", "age": 30, "active": true}),
    )
    .await
    .unwrap();
    db.put(
        "users",
        "u4",
        json!({"name": "Diana", "age": 28, "active": true}),
    )
    .await
    .unwrap();

    // Complex filter: active AND age >= 28.
    let filter = Filter::and(vec![
        Filter::eq("active", json!(true)),
        Filter::gte("age", json!(28)),
    ]);

    let result = db
        .query("users", Query::new().filter(filter))
        .await
        .unwrap();

    assert_eq!(result.records.len(), 2); // Charlie (30) and Diana (28)
}
