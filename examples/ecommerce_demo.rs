//! E-Commerce Order Management Demo
//!
//! Demonstrates all KoruDelta features in a real-world scenario:
//! - Basic CRUD operations
//! - Version history and time travel
//! - Advanced queries and aggregations
//! - Materialized views
//! - Real-time subscriptions
//!
//! Run with: cargo run --example ecommerce_demo

use koru_delta::query::{Filter, Query};
use koru_delta::subscriptions::Subscription;
use koru_delta::views::ViewDefinition;
use koru_delta::{DeltaResult, KoruDelta};
use serde_json::json;
use std::time::Duration;

fn print_header(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}\n", "=".repeat(60));
}

fn print_section(title: &str) {
    println!("\n--- {} ---\n", title);
}

#[tokio::main]
async fn main() -> DeltaResult<()> {
    println!("\n");
    print_header("KoruDelta E-Commerce Demo");
    println!("A comprehensive demonstration of the Invisible Database");
    println!("showcasing Git-like versioning, Redis-like simplicity,");
    println!("and powerful query capabilities.\n");

    // Initialize database
    let db = KoruDelta::start().await?;
    println!("✓ Database started (zero configuration!)\n");

    // =========================================================================
    // PART 1: Product Catalog Management
    // =========================================================================
    print_header("Part 1: Product Catalog Management");

    print_section("Creating Product Catalog");

    let products = vec![
        (
            "laptop",
            json!({
                "name": "MacBook Pro 16",
                "sku": "MBP-16-2024",
                "price": 2499.00,
                "stock": 50,
                "category": "electronics",
                "status": "active"
            }),
        ),
        (
            "phone",
            json!({
                "name": "iPhone 15 Pro",
                "sku": "IP15-PRO",
                "price": 1199.00,
                "stock": 100,
                "category": "electronics",
                "status": "active"
            }),
        ),
        (
            "headphones",
            json!({
                "name": "AirPods Pro 2",
                "sku": "APP-2",
                "price": 249.00,
                "stock": 200,
                "category": "electronics",
                "status": "active"
            }),
        ),
        (
            "charger",
            json!({
                "name": "MagSafe Charger",
                "sku": "MSG-CHG",
                "price": 39.00,
                "stock": 500,
                "category": "accessories",
                "status": "active"
            }),
        ),
        (
            "case",
            json!({
                "name": "iPhone Leather Case",
                "sku": "ILC-15",
                "price": 59.00,
                "stock": 150,
                "category": "accessories",
                "status": "active"
            }),
        ),
        (
            "watch",
            json!({
                "name": "Apple Watch Ultra 2",
                "sku": "AWU-2",
                "price": 799.00,
                "stock": 75,
                "category": "electronics",
                "status": "active"
            }),
        ),
    ];

    for (key, product) in &products {
        db.put("products", *key, product.clone()).await?;
        println!("  ✓ Created product: {}", product["name"]);
    }

    // =========================================================================
    // PART 2: Customer Profiles
    // =========================================================================
    print_header("Part 2: Customer Profiles");

    print_section("Creating Customer Profiles");

    let customers = vec![
        (
            "alice",
            json!({
                "name": "Alice Johnson",
                "email": "alice@example.com",
                "tier": "gold",
                "total_spent": 5420.00,
                "joined": "2023-01-15"
            }),
        ),
        (
            "bob",
            json!({
                "name": "Bob Smith",
                "email": "bob@example.com",
                "tier": "silver",
                "total_spent": 1250.00,
                "joined": "2023-06-22"
            }),
        ),
        (
            "charlie",
            json!({
                "name": "Charlie Brown",
                "email": "charlie@example.com",
                "tier": "bronze",
                "total_spent": 350.00,
                "joined": "2024-03-10"
            }),
        ),
    ];

    for (key, customer) in &customers {
        db.put("customers", *key, customer.clone()).await?;
        println!(
            "  ✓ Created customer: {} ({})",
            customer["name"], customer["tier"]
        );
    }

    // =========================================================================
    // PART 3: Order Management
    // =========================================================================
    print_header("Part 3: Order Management");

    print_section("Creating Orders");

    let orders = vec![
        (
            "ORD-001",
            json!({
                "customer_id": "alice",
                "items": [{"sku": "MBP-16-2024", "qty": 1, "price": 2499.00}],
                "total": 2499.00,
                "status": "delivered",
                "region": "west"
            }),
        ),
        (
            "ORD-002",
            json!({
                "customer_id": "bob",
                "items": [
                    {"sku": "IP15-PRO", "qty": 1, "price": 1199.00},
                    {"sku": "APP-2", "qty": 1, "price": 249.00}
                ],
                "total": 1448.00,
                "status": "shipped",
                "region": "east"
            }),
        ),
        (
            "ORD-003",
            json!({
                "customer_id": "alice",
                "items": [{"sku": "MSG-CHG", "qty": 2, "price": 39.00}],
                "total": 78.00,
                "status": "processing",
                "region": "west"
            }),
        ),
        (
            "ORD-004",
            json!({
                "customer_id": "charlie",
                "items": [{"sku": "ILC-15", "qty": 1, "price": 59.00}],
                "total": 59.00,
                "status": "pending",
                "region": "central"
            }),
        ),
        (
            "ORD-005",
            json!({
                "customer_id": "alice",
                "items": [
                    {"sku": "IP15-PRO", "qty": 2, "price": 1199.00},
                    {"sku": "ILC-15", "qty": 2, "price": 59.00}
                ],
                "total": 2516.00,
                "status": "pending",
                "region": "west"
            }),
        ),
        (
            "ORD-006",
            json!({
                "customer_id": "bob",
                "items": [{"sku": "AWU-2", "qty": 1, "price": 799.00}],
                "total": 799.00,
                "status": "shipped",
                "region": "east"
            }),
        ),
    ];

    for (key, order) in &orders {
        db.put("orders", *key, order.clone()).await?;
        println!(
            "  ✓ Created order: {} - ${:.2} ({})",
            key, order["total"], order["status"]
        );
    }

    // =========================================================================
    // PART 4: Version History & Time Travel
    // =========================================================================
    print_header("Part 4: Version History & Time Travel");

    print_section("Simulating Product Updates");

    // Record time before updates
    let before_sale = chrono::Utc::now();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Black Friday Sale - update laptop price
    println!("  Applying Black Friday sale...");
    db.put(
        "products",
        "laptop",
        json!({
            "name": "MacBook Pro 16",
            "sku": "MBP-16-2024",
            "price": 2199.00,  // $300 off!
            "stock": 50,
            "category": "electronics",
            "status": "active",
            "note": "Black Friday Sale"
        }),
    )
    .await?;
    println!("  ✓ Laptop price reduced: $2499 -> $2199");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stock depleted from sales
    db.put(
        "products",
        "laptop",
        json!({
            "name": "MacBook Pro 16",
            "sku": "MBP-16-2024",
            "price": 2199.00,
            "stock": 35,  // 15 sold!
            "category": "electronics",
            "status": "active",
            "note": "Black Friday Sale"
        }),
    )
    .await?;
    println!("  ✓ Stock updated: 50 -> 35 (15 units sold!)");

    print_section("Complete Version History");

    let history = db.history("products", "laptop").await?;
    println!("  Laptop has {} versions:\n", history.len());

    for (i, entry) in history.iter().enumerate() {
        let price = entry
            .value
            .get("price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let stock = entry
            .value
            .get("stock")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let note = entry
            .value
            .get("note")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        println!(
            "  Version {}: {} UTC",
            i + 1,
            entry.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
        println!("    Price: ${:.2}, Stock: {}, Note: {}", price, stock, note);
        println!("    Hash: {}...\n", &entry.version_id[..16]);
    }

    print_section("Time Travel Query");

    println!("  Querying laptop state BEFORE Black Friday sale...");
    let past_value = db.get_at("products", "laptop", before_sale).await?;
    let price = past_value
        .value
        .get("price")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let stock = past_value
        .value
        .get("stock")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    println!("  ✓ Price was: ${:.2}", price);
    println!("  ✓ Stock was: {}", stock);

    println!("\n  Current laptop state:");
    let current = db.get("products", "laptop").await?;
    let price = current.value.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let stock = current.value.get("stock").and_then(|v| v.as_i64()).unwrap_or(0);
    println!("  ✓ Price is: ${:.2}", price);
    println!("  ✓ Stock is: {}", stock);

    // =========================================================================
    // PART 5: Advanced Queries
    // =========================================================================
    print_header("Part 5: Advanced Queries & Aggregations");

    print_section("Filter: Pending Orders");

    let pending = db
        .query(
            "orders",
            Query::new().filter(Filter::eq("status", "pending")),
        )
        .await?;

    println!("  Found {} pending orders:", pending.records.len());
    for record in &pending.records {
        println!("    • {} - ${:.2}", record.key, record.value["total"]);
    }

    print_section("Filter: High-Value Orders (> $1000)");

    let high_value = db
        .query("orders", Query::new().filter(Filter::gt("total", 1000.0)))
        .await?;

    println!("  Found {} high-value orders:", high_value.records.len());
    for record in &high_value.records {
        println!(
            "    • {} - ${:.2} ({})",
            record.key, record.value["total"], record.value["status"]
        );
    }

    print_section("Filter: Electronics Products");

    let electronics = db
        .query(
            "products",
            Query::new().filter(Filter::eq("category", "electronics")),
        )
        .await?;

    println!(
        "  Found {} electronics products:",
        electronics.records.len()
    );
    for record in &electronics.records {
        println!(
            "    • {} - {} (${:.2})",
            record.key, record.value["name"], record.value["price"]
        );
    }

    print_section("Sort: Orders by Total (Descending)");

    let sorted = db
        .query("orders", Query::new().sort_by("total", false).limit(3))
        .await?;

    println!("  Top 3 orders by value:");
    for (i, record) in sorted.records.iter().enumerate() {
        println!(
            "    {}. {} - ${:.2}",
            i + 1,
            record.key,
            record.value["total"]
        );
    }

    print_section("Aggregations");

    // Count orders manually since query returns QueryResult
    let all_orders = db.query("orders", Query::new()).await?;
    println!("  Total orders: {}", all_orders.records.len());

    // Sum totals manually
    let total_revenue: f64 = all_orders
        .records
        .iter()
        .filter_map(|r| r.value.get("total").and_then(|t| t.as_f64()))
        .sum();
    println!("  Total revenue: ${:.2}", total_revenue);

    // Average
    let avg_order_value = if !all_orders.records.is_empty() {
        total_revenue / all_orders.records.len() as f64
    } else {
        0.0
    };
    println!("  Average order value: ${:.2}", avg_order_value);

    // Max order
    let max_order = all_orders
        .records
        .iter()
        .filter_map(|r| r.value.get("total").and_then(|t| t.as_f64()).map(|t| (r.key.clone(), t)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if let Some((key, total)) = max_order {
        println!("  Largest order: {} - ${:.2}", key, total);
    }

    // =========================================================================
    // PART 6: Materialized Views
    // =========================================================================
    print_header("Part 6: Materialized Views (Dashboard Caching)");

    print_section("Creating Views");

    // Pending orders view
    let pending_view = ViewDefinition::new("pending_orders", "orders")
        .with_query(Query::new().filter(Filter::eq("status", "pending")))
        .auto_refresh(true);
    db.create_view(pending_view).await?;
    println!("  ✓ Created 'pending_orders' view");

    // High value orders view
    let high_value_view = ViewDefinition::new("high_value_orders", "orders")
        .with_query(Query::new().filter(Filter::gt("total", 1000.0)));
    db.create_view(high_value_view).await?;
    println!("  ✓ Created 'high_value_orders' view");

    // Electronics products view
    let electronics_view = ViewDefinition::new("electronics", "products")
        .with_query(Query::new().filter(Filter::eq("category", "electronics")));
    db.create_view(electronics_view).await?;
    println!("  ✓ Created 'electronics' view");

    // Low stock view
    let low_stock_view = ViewDefinition::new("low_stock", "products")
        .with_query(Query::new().filter(Filter::lt("stock", 100)));
    db.create_view(low_stock_view).await?;
    println!("  ✓ Created 'low_stock' view");

    print_section("List All Views");

    let views = db.list_views().await;
    println!("  {} views configured:", views.len());
    for view in &views {
        println!("    • {} -> {}", view.name, view.source_collection);
    }

    print_section("Query Views (Instant Results!)");

    let pending_results = db.query_view("pending_orders").await?;
    println!(
        "  Pending Orders View: {} orders",
        pending_results.records.len()
    );
    for record in &pending_results.records {
        println!("    • {} - ${:.2}", record.key, record.value["total"]);
    }

    println!();

    let low_stock_results = db.query_view("low_stock").await?;
    println!(
        "  Low Stock View: {} products need restocking",
        low_stock_results.records.len()
    );
    for record in &low_stock_results.records {
        println!(
            "    • {} - {} units remaining",
            record.value["name"], record.value["stock"]
        );
    }

    // =========================================================================
    // PART 7: Real-Time Subscriptions
    // =========================================================================
    print_header("Part 7: Real-Time Subscriptions");

    print_section("Setting Up Order Notifications");

    // Subscribe to all order changes
    let (sub_id, mut rx) = db.subscribe(Subscription::collection("orders")).await;
    println!("  ✓ Subscribed to order changes (ID: {})", sub_id);

    print_section("Simulating Order Status Updates");

    // Process order ORD-004
    println!("  Processing order ORD-004...");
    db.put_notify(
        "orders",
        "ORD-004",
        json!({
            "customer_id": "charlie",
            "items": [{"sku": "ILC-15", "qty": 1, "price": 59.00}],
            "total": 59.00,
            "status": "processing",  // Changed from pending!
            "region": "central"
        }),
    )
    .await?;

    // Check for notification
    tokio::select! {
        event = rx.recv() => {
            if let Ok(e) = event {
                println!("  ✓ Received notification:");
                println!("    Type: {:?}", e.change_type);
                println!("    Collection: {}", e.collection);
                println!("    Key: {}", e.key);
                if let Some(val) = &e.value {
                    println!("    New status: {}", val["status"]);
                }
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            println!("  (notification pending)");
        }
    }

    // Ship order ORD-004
    println!("\n  Shipping order ORD-004...");
    db.put_notify(
        "orders",
        "ORD-004",
        json!({
            "customer_id": "charlie",
            "items": [{"sku": "ILC-15", "qty": 1, "price": 59.00}],
            "total": 59.00,
            "status": "shipped",  // Changed from processing!
            "region": "central"
        }),
    )
    .await?;

    // Check for notification
    tokio::select! {
        event = rx.recv() => {
            if let Ok(e) = event {
                println!("  ✓ Received notification:");
                println!("    Type: {:?}", e.change_type);
                println!("    Key: {}", e.key);
                if let Some(val) = &e.value {
                    println!("    New status: {}", val["status"]);
                }
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            println!("  (notification pending)");
        }
    }

    // Cleanup subscription
    db.unsubscribe(sub_id).await?;
    println!("\n  ✓ Unsubscribed from notifications");

    // =========================================================================
    // FINAL: Database Statistics
    // =========================================================================
    print_header("Final: Database Statistics");

    let stats = db.stats().await;
    println!("  Total Keys:      {}", stats.key_count);
    println!("  Total Versions:  {}", stats.total_versions);
    println!("  Namespaces:      {}", stats.namespace_count);

    print_header("Demo Complete!");
    println!("KoruDelta demonstrated:");
    println!("  ✓ Zero-configuration startup");
    println!("  ✓ JSON document storage");
    println!("  ✓ Git-like version history");
    println!("  ✓ Time travel queries");
    println!("  ✓ Advanced filtering & sorting");
    println!("  ✓ Aggregations (count, sum, avg, max)");
    println!("  ✓ Materialized views");
    println!("  ✓ Real-time subscriptions");
    println!("\n\"Invisible. Causal. Everywhere.\"\n");

    Ok(())
}
