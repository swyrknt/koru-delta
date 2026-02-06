# KoruDelta — The Invisible Database

[![Crates.io](https://img.shields.io/crates/v/koru-delta.svg)](https://crates.io/crates/koru-delta)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**Tagline:** *"Invisible. Causal. Everywhere."*

**One-line:** *"KoruDelta is the invisible database that gives you Git-like history, Redis-like speed, and distributed consistency—without configuration."*

## Get Started in 10 Seconds

```rust
use koru_delta::KoruDelta;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = KoruDelta::start().await?;

    db.put("users", "alice", serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com"
    })).await?;

    let user = db.get("users", "alice").await?;
    println!("User: {:?}", user);

    Ok(())
}
```

## Why KoruDelta?

### 🧠 Invisible Operations

`KoruDelta::start()` is all you need. No config files, no cluster rituals.

### 🤝 Always Consistent

Multiple nodes? KoruDelta automatically syncs and keeps data consistent.

### ⏱ Built-in History

Every change is versioned. Time travel and auditing are one method away.

```rust
// Get the full history of changes
let history = db.history("users", "alice").await?;

// Time travel to a specific point
let past_user = db.get_at("users", "alice", timestamp).await?;
```

### 🌐 Runs Everywhere

Same core engine runs in servers, laptops, browsers, and edge devices.

### 🤝 Automatic Distribution

Multiple nodes sync automatically with zero configuration:

```bash
# Machine 1 - Start a node
kdelta start

# Machine 2 - Join the cluster
kdelta start --join 192.168.1.100:7878

# Now you have a distributed cluster!
```

### 🔌 HTTP API

Access KoruDelta over HTTP for remote operations and web integration:

```bash
# Start HTTP server
kdelta serve --port 8080

# From anywhere, use remote CLI
kdelta --url http://localhost:8080 get users/alice
kdelta --url http://localhost:8080 set users/bob '{"name": "Bob"}'

# Time travel via HTTP
kdelta --url http://localhost:8080 get users/alice --at "2026-02-04T12:00:00Z"
```

REST endpoints:
- `GET /api/v1/:namespace/:key` - Get value
- `PUT /api/v1/:namespace/:key` - Store value
- `GET /api/v1/:namespace/:key/history` - Get history
- `GET /api/v1/:namespace/:key/at/:timestamp` - Time travel
- `POST /api/v1/:namespace/query` - Execute queries

### 🔐 Self-Sovereign Auth

Built-in authentication with zero configuration. Users own their keys.

```rust
use koru_delta::auth::{AuthManager, IdentityUserData, Permission};

let auth = AuthManager::new(storage);

// Create identity (mines proof-of-work)
let (identity, secret_key) = auth.create_identity(IdentityUserData {
    display_name: Some("Alice".to_string()),
    ..Default::default()
})?;

// Authenticate via challenge-response
let challenge = auth.create_challenge(&identity.public_key)?;
let response = sign_challenge(&secret_key, &challenge)?;
let session = auth.verify_and_create_session(&identity.public_key, &challenge, &response)?;

// Grant capabilities
auth.grant_capability(&identity, &secret_key, &grantee, 
    ResourcePattern::Namespace("documents".to_string()),
    Permission::Write, None)?;
```

### 🔍 Powerful Queries

Filter, sort, and aggregate your data with a fluent query API:

```rust
use koru_delta::query::{Query, Filter, Aggregation};

// Find active users over 30, sorted by name
let results = db.query("users", Query::new()
    .filter(Filter::gt("age", 30))
    .filter(Filter::eq("status", "active"))
    .sort_by("name", true)
    .limit(10)
).await?;

// Aggregate sales by region
let total = db.query("sales", Query::new()
    .aggregate(Aggregation::sum("amount"))
).await?;
```

### 📊 Materialized Views

Pre-compute and cache query results for instant access:

```rust
use koru_delta::views::ViewDefinition;

// Create a view of active users
let view = ViewDefinition::new("active_users", "users")
    .with_query(Query::new().filter(Filter::eq("status", "active")))
    .auto_refresh(true);

db.create_view(view).await?;

// Query the view (instant, cached results)
let results = db.query_view("active_users").await?;
```

### 🔔 Real-time Subscriptions

Get notified when data changes:

```rust
use koru_delta::subscriptions::Subscription;

// Subscribe to user changes
let (id, mut rx) = db.subscribe(Subscription::collection("users")).await;

// React to changes in real-time
while let Ok(event) = rx.recv().await {
    println!("Change: {} {}/{}", event.change_type, event.collection, event.key);
}
```

## Core Features

- **Zero-configuration** - Start a node with one line of code
- **Causal history** - Every change is an event in a versioned timeline
- **Time travel** - Query data at any point in history
- **Visual diffs** - Compare versions with Git-style colored output
- **JSON native** - Store and query JSON documents naturally
- **Content-addressed** - Built on koru-lambda-core's distinction calculus
- **Thread-safe** - Concurrent operations with no data races
- **WASM-ready** - Run in browsers, Node.js, and edge environments
- **HTTP API** - RESTful endpoints for remote access
- **Remote CLI** - Connect to any KoruDelta instance over HTTP
- **CLI included** - Full-featured command-line tool for interactive use
- **High performance** - ~340ns reads, 27K+ writes/sec
- **Query engine** - Filter, sort, project, and aggregate data
- **Materialized views** - Cache query results for instant access
- **Real-time subscriptions** - Get notified when data changes

## CLI Reference

```bash
# Basic Operations
kdelta set users/alice '{"name": "Alice", "age": 30}'
kdelta get users/alice
kdelta get users/alice --at "2026-02-04T12:00:00Z"  # Time travel
kdelta log users/alice              # Show history
kdelta diff users/alice             # Compare versions

# Remote Operations (via HTTP)
kdelta --url http://localhost:8080 get users/alice
kdelta --url http://localhost:8080 set users/bob '{"name": "Bob"}'
kdelta --url http://localhost:8080 query users --filter 'age > 30'

# Cluster Operations
kdelta start                        # Start a cluster node
kdelta start --join 192.168.1.100   # Join existing cluster
kdelta peers                        # Show cluster peers

# HTTP API Server
kdelta serve                        # Start HTTP server on port 8080
kdelta serve --port 3000            # Start on custom port
kdelta status                       # Show database stats

# Query Operations
kdelta query users                              # Get all users
kdelta query users --filter 'age > 30'          # Filter
kdelta query users --sort name --limit 10       # Sort and limit
kdelta query users --count                      # Count records
kdelta query sales --sum amount                 # Aggregate

# View Operations
kdelta view create active_users users --filter 'status = "active"'
kdelta view list
kdelta view refresh active_users
kdelta view query active_users
kdelta view delete active_users

# Watch for Changes
kdelta watch users                  # Watch collection
kdelta watch users/alice            # Watch specific key
kdelta watch --all                  # Watch everything
```

## Architecture

KoruDelta is built on top of [koru-lambda-core](https://crates.io/crates/koru-lambda-core), a minimal axiomatic system for distributed computation. This gives KoruDelta:

- **Mathematical guarantees** - Safety and consistency from a formal foundation
- **Structural integrity** - Can't corrupt by design
- **Deterministic operations** - Same inputs always produce the same results
- **Natural distribution** - Consensus and sync emerge from the axioms

The math is your secret weapon, not your configuration burden.

### Persistence

KoruDelta uses a **Write-Ahead Log (WAL)** with content-addressed storage for durability and performance:

```
~/.korudelta/db/
├── wal/000001.wal          # Append-only log entries
└── values/ab/cd1234...     # Content-addressed values (by hash)
```

**Why this matters:**
- **O(1) writes** - Append 120 bytes instead of rewriting the entire database
- **Crash-safe** - Never overwrites existing data
- **Automatic deduplication** - Identical values stored once by content hash
- **Immutable history** - The log is the history

Backwards compatible with legacy snapshot files.

## Status

**All three phases complete!** KoruDelta is feature-complete and production-ready.

### ✅ Phase 1: Magical Single Node (Complete)
- Simple key/value + JSON document storage
- Full causal history tracking
- Time travel queries (`get_at`)
- Visual diff command for comparing versions
- Clean, simple API (Rust + WASM)
- Full-featured CLI tool (`kdelta`)
- **Write-Ahead Log (WAL) persistence** - O(1) append-only writes, content-addressed storage

### ✅ Phase 2: Automatic Distribution (Complete)
- Multi-node clustering (`kdelta start --join`)
- Automatic data sync between nodes
- Gossip protocol for peer discovery
- Cluster health monitoring
- Snapshot sync on join

### ✅ Phase 3: Advanced Features (Complete)
- Query engine (filter, sort, project, aggregate)
- Materialized views (create, refresh, query)
- Real-time subscriptions (change notifications)
- History queries across versions

### Project Stats
- **~6,350 lines** of Rust code
- **198 tests** (all passing)
- **~340ns** read latency
- **~27K** writes/sec throughput
- Cross-platform (Linux, macOS, Windows, WASM)

## Examples

The `examples/` directory contains runnable demos:

```bash
# E-Commerce demo: CRUD, versioning, queries, views, subscriptions
cargo run --example ecommerce_demo

# Clustering demo: Multi-node replication and peer discovery
cargo run --example cluster_demo
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and [ARCHITECTURE.md](ARCHITECTURE.md) for technical details.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Links

- [GitHub Repository](https://github.com/swyrknt/koru-delta) - Source code and issues
- [Design Document](DESIGN.md) - Design philosophy and decisions
- [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core) - The underlying distinction engine

---

*KoruDelta: Where data meets history, and simplicity meets power.*
