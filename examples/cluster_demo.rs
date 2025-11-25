//! Multi-Node Clustering Demo
//!
//! Demonstrates KoruDelta's distributed capabilities:
//! - Starting multiple nodes
//! - Automatic peer discovery
//! - Data replication across nodes
//! - Cluster health monitoring
//!
//! Run with: cargo run --example cluster_demo

use koru_delta::cluster::{ClusterConfig, ClusterNode};
use koru_delta::storage::CausalStorage;
use koru_delta::DeltaResult;
use koru_lambda_core::DistinctionEngine;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

fn print_header(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}\n", "=".repeat(60));
}

fn print_section(title: &str) {
    println!("\n--- {} ---\n", title);
}

/// Helper function to create test storage and engine.
fn create_storage() -> (Arc<CausalStorage>, Arc<DistinctionEngine>) {
    let engine = Arc::new(DistinctionEngine::new());
    let storage = Arc::new(CausalStorage::new(Arc::clone(&engine)));
    (storage, engine)
}

/// Helper function to create a cluster config with localhost and random port.
fn random_port_config() -> ClusterConfig {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    ClusterConfig::new().bind_addr(addr)
}

#[tokio::main]
async fn main() -> DeltaResult<()> {
    println!("\n");
    print_header("KoruDelta Multi-Node Cluster Demo");
    println!("Demonstrating automatic data replication and peer discovery\n");

    // =========================================================================
    // PART 1: Start Primary Node
    // =========================================================================
    print_header("Part 1: Starting Primary Node");

    print_section("Creating Node 1 (Primary)");

    let (storage1, engine1) = create_storage();
    let config1 = random_port_config();
    let node1 = ClusterNode::new(Arc::clone(&storage1), engine1, config1);
    node1.start().await?;

    println!("  ✓ Node 1 started");
    println!("  ✓ Node ID: {}", node1.node_id());
    println!("  ✓ Listening on: {}", node1.bind_addr());

    // Wait for node to fully start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Add some data to the primary node
    print_section("Adding Data to Node 1");

    storage1.put(
        "inventory",
        "item-001",
        json!({
            "name": "Widget A",
            "quantity": 100,
            "price": 29.99
        }),
    )?;
    println!("  ✓ Added item-001: Widget A");

    storage1.put(
        "inventory",
        "item-002",
        json!({
            "name": "Widget B",
            "quantity": 50,
            "price": 49.99
        }),
    )?;
    println!("  ✓ Added item-002: Widget B");

    storage1.put(
        "inventory",
        "item-003",
        json!({
            "name": "Widget C",
            "quantity": 25,
            "price": 99.99
        }),
    )?;
    println!("  ✓ Added item-003: Widget C");

    // =========================================================================
    // PART 2: Start Secondary Node and Join Cluster
    // =========================================================================
    print_header("Part 2: Starting Secondary Node");

    print_section("Creating Node 2 (Joining Cluster)");

    let (storage2, engine2) = create_storage();
    let config2 = random_port_config().join(node1.bind_addr());
    let node2 = ClusterNode::new(Arc::clone(&storage2), engine2, config2);

    println!(
        "  ✓ Node 2 configured to join cluster at {}",
        node1.bind_addr()
    );

    node2.start().await?;
    println!("  ✓ Node 2 started");
    println!("  ✓ Node ID: {}", node2.node_id());
    println!("  ✓ Listening on: {}", node2.bind_addr());

    // Wait for sync
    tokio::time::sleep(Duration::from_millis(300)).await;

    // =========================================================================
    // PART 3: Verify Data Replication
    // =========================================================================
    print_header("Part 3: Verifying Data Replication");

    print_section("Reading Data from Node 2 (Replicated)");

    match storage2.get("inventory", "item-001") {
        Ok(item) => println!("  Node 2 - item-001: {}", item.value()["name"]),
        Err(e) => println!("  Node 2 - item-001: (error: {})", e),
    }

    match storage2.get("inventory", "item-002") {
        Ok(item) => println!("  Node 2 - item-002: {}", item.value()["name"]),
        Err(e) => println!("  Node 2 - item-002: (error: {})", e),
    }

    match storage2.get("inventory", "item-003") {
        Ok(item) => println!("  Node 2 - item-003: {}", item.value()["name"]),
        Err(e) => println!("  Node 2 - item-003: (error: {})", e),
    }

    // Check if node2 has synced data
    let node2_keys = storage2.list_keys("inventory");
    if !node2_keys.is_empty() {
        println!("\n  ✓ Data successfully replicated to Node 2!");
        println!("  ✓ Node 2 has {} inventory items", node2_keys.len());
    } else {
        println!("\n  ⚠ Data replication in progress...");
    }

    // =========================================================================
    // PART 4: Two-Way Replication
    // =========================================================================
    print_header("Part 4: Two-Way Replication");

    print_section("Adding Data on Node 2");

    storage2.put(
        "inventory",
        "item-004",
        json!({
            "name": "Widget D",
            "quantity": 75,
            "price": 39.99
        }),
    )?;
    println!("  ✓ Node 2: Added item-004: Widget D");

    // Wait for sync
    tokio::time::sleep(Duration::from_millis(300)).await;

    print_section("Verifying Replication to Node 1");

    match storage1.get("inventory", "item-004") {
        Ok(item) => {
            println!(
                "  Node 1 - item-004: {} (replicated from Node 2!)",
                item.value()["name"]
            );
            println!("\n  ✓ Bidirectional replication working!");
        }
        Err(_) => {
            println!("  ⚠ item-004 not yet replicated to Node 1");
        }
    }

    // =========================================================================
    // PART 5: Cluster Status
    // =========================================================================
    print_header("Part 5: Cluster Status");

    let status1 = node1.status().await;
    print_section("Node 1 Status");
    println!("  Running: {}", status1.is_running);
    println!("  Peers: {}", status1.peer_count);
    println!("  Healthy Peers: {}", status1.healthy_peers);
    println!("  Keys: {}", storage1.key_count());
    println!("  Total Versions: {}", storage1.total_version_count());

    let status2 = node2.status().await;
    print_section("Node 2 Status");
    println!("  Running: {}", status2.is_running);
    println!("  Peers: {}", status2.peer_count);
    println!("  Healthy Peers: {}", status2.healthy_peers);
    println!("  Keys: {}", storage2.key_count());
    println!("  Total Versions: {}", storage2.total_version_count());

    // =========================================================================
    // PART 6: Start Third Node
    // =========================================================================
    print_header("Part 6: Adding Third Node");

    print_section("Creating Node 3");

    let (storage3, engine3) = create_storage();
    let config3 = random_port_config().join(node1.bind_addr());
    let node3 = ClusterNode::new(Arc::clone(&storage3), engine3, config3);
    node3.start().await?;

    println!("  ✓ Node 3 started");
    println!("  ✓ Node ID: {}", node3.node_id());
    println!("  ✓ Listening on: {}", node3.bind_addr());

    // Wait for sync
    tokio::time::sleep(Duration::from_millis(300)).await;

    print_section("Verifying Full Cluster Sync");

    let keys = storage3.list_keys("inventory");
    println!("  Node 3 has {} inventory items:", keys.len());
    for key in &keys {
        if let Ok(item) = storage3.get("inventory", key.as_str()) {
            println!("    • {}: {}", key, item.value()["name"]);
        }
    }

    if keys.len() >= 3 {
        println!("\n  ✓ All 3 nodes synchronized!");
    }

    // =========================================================================
    // PART 7: Peer Discovery
    // =========================================================================
    print_header("Part 7: Peer Discovery");

    print_section("Node 1 Peers");
    let peers1 = node1.peers();
    println!("  Node 1 knows {} peer(s):", peers1.len());
    for peer in &peers1 {
        println!(
            "    • {} at {} ({:?})",
            peer.node_id, peer.address, peer.status
        );
    }

    print_section("Node 2 Peers");
    let peers2 = node2.peers();
    println!("  Node 2 knows {} peer(s):", peers2.len());
    for peer in &peers2 {
        println!(
            "    • {} at {} ({:?})",
            peer.node_id, peer.address, peer.status
        );
    }

    print_section("Node 3 Peers");
    let peers3 = node3.peers();
    println!("  Node 3 knows {} peer(s):", peers3.len());
    for peer in &peers3 {
        println!(
            "    • {} at {} ({:?})",
            peer.node_id, peer.address, peer.status
        );
    }

    // =========================================================================
    // Cleanup
    // =========================================================================
    print_header("Cleanup");

    node1.stop().await?;
    println!("  ✓ Node 1 stopped");

    node2.stop().await?;
    println!("  ✓ Node 2 stopped");

    node3.stop().await?;
    println!("  ✓ Node 3 stopped");

    print_header("Cluster Demo Complete!");
    println!("Demonstrated:");
    println!("  ✓ Multi-node cluster startup");
    println!("  ✓ Automatic peer discovery");
    println!("  ✓ Bidirectional data replication");
    println!("  ✓ Dynamic node joining");
    println!("  ✓ Cluster status monitoring");
    println!("\n\"Automatic Distribution. Zero Configuration.\"\n");

    Ok(())
}
