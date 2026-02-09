# KoruDelta — The Invisible Database

[![Crates.io](https://img.shields.io/crates/v/koru-delta.svg)](https://crates.io/crates/koru-delta)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**Tagline:** *"Invisible. Causal. Everywhere."*

**One-line:** *"KoruDelta is the invisible database that gives you Git-like history, Redis-like speed, and distributed consistency—without configuration."*

## Current Status ✅ Production Ready (v2.0.0)

**v2.0.0 is production-ready for single-node deployments:**
- ✅ Zero-config setup
- ✅ Crash recovery (WAL + checksums)
- ✅ ~200+ writes/sec, ~159K reads/sec (validated)
- ✅ Materialized views with persistence
- ✅ Self-sovereign identity (proof-of-work auth)
- ✅ Vector embeddings & semantic search
- ✅ Real-time subscriptions
- ✅ Complete version history & time travel
- ✅ Automatic memory management (Hot/Warm/Cold/Deep tiers)
- ✅ Structured logging & resource limits
- ✅ WASM support for browsers

**Install in 10 seconds:**
```bash
cargo install koru-delta
```

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

### 🛡️ Production Hardened

Crash recovery, corruption detection, and structured logging built-in. Data survives power loss.

Validated through comprehensive stress testing:
- 10,000+ keys stored and retrieved
- 100+ version history depth
- 100 concurrent writers with no conflicts
- 100KB value storage

### ⏱ Built-in History

Every change is versioned. Time travel and auditing are one method away.

```rust
// Get the full history of changes
let history = db.history("users", "alice").await?;

// Time travel to a specific point
let past_user = db.get_at("users", "alice", timestamp).await?;
```

### 👁️ Materialized Views (v2.0.0)

Create persistent, auto-refreshing query results:

```rust
use koru_delta::views::ViewDefinition;
use koru_delta::query::{Query, Filter};

let view = ViewDefinition {
    name: "active_users".to_string(),
    source_collection: "users".to_string(),
    query: Query {
        filters: vec![Filter::eq("status", "active")],
        ..Default::default()
    },
    created_at: chrono::Utc::now(),
    description: Some("Active users only".to_string()),
    auto_refresh: true,
};

db.create_view(view).await?;

// Query the view (instant, cached results)
let results = db.query_view("active_users").await?;
```

Views persist across database restarts.

### 🔐 Self-Sovereign Auth (v2.0.0)

Built-in authentication with proof-of-work identity mining:

```rust
use koru_delta::auth::{mine_identity, IdentityUserData};

// Mine an identity (proves work to prevent spam)
let identity = mine_identity(
    IdentityUserData::new("alice"),
    4  // difficulty level
).await;

// identity.id is your public key
// identity.secret_key is your private key (keep secure!)
```

### 🔍 Vector Search (v2.0.0)

Store and search vector embeddings for semantic similarity:

```rust
use koru_delta::vector::Vector;

// Store an embedding
let embedding = Vector::new(vec![0.1, 0.2, 0.3, ...], "text-embedding-ada-002");
db.embed("vectors", "doc1", embedding, None).await?;

// Search for similar vectors
let query = Vector::new(vec![0.15, 0.25, 0.35, ...], "text-embedding-ada-002");
let results = db.embed_search(Some("vectors"), &query, 
    VectorSearchOptions { top_k: 10, threshold: 0.0, model_filter: None }
).await?;
```

### 🔔 Real-time Subscriptions (v2.0.0)

Get notified when data changes:

```rust
use koru_delta::subscriptions::{Subscription, ChangeType};

let (sub_id, mut rx) = db.subscribe(Subscription {
    collection: Some("users".to_string()),
    key: None,
    filter: None,
    change_types: vec![ChangeType::Insert, ChangeType::Update, ChangeType::Delete],
    name: Some("user-monitor".to_string()),
}).await;

while let Ok(event) = rx.recv().await {
    println!("Change: {:?} on {}/{}", 
        event.change_type, event.collection, event.key);
}
```

### 🌐 Runs Everywhere

Same core engine runs in servers, laptops, browsers, and edge devices.

**WASM/Browser:**
```javascript
import init, { KoruDeltaWasm } from 'koru-delta';

await init();

// Persistent database (IndexedDB)
const db = await KoruDeltaWasm.newPersistent();

await db.put('users', 'alice', { name: 'Alice', age: 30 });
const user = await db.get('users', 'alice');
```

### 🤝 HTTP API

Access KoruDelta over HTTP for remote operations and web integration:

```bash
# Start HTTP server
kdelta serve --port 8080

# From anywhere, use remote CLI
kdelta --url http://localhost:8080 get users/alice
kdelta --url http://localhost:8080 set users/bob '{"name": "Bob"}'
```

REST endpoints:
- `GET /api/v1/:namespace/:key` - Get value
- `PUT /api/v1/:namespace/:key` - Store value
- `GET /api/v1/:namespace/:key/history` - Get history
- `GET /api/v1/:namespace/:key/at/:timestamp` - Time travel
- `POST /api/v1/:namespace/query` - Execute queries

## Core Features

- **Zero-configuration** - Start a node with one line of code
- **Production hardened** - Crash recovery, corruption detection, structured logging
- **Causal history** - Every change is versioned with full audit trail
- **Time travel** - Query data at any point in history
- **Materialized views** - Persistent, auto-refreshing query caches
- **Self-sovereign auth** - Proof-of-work identity mining
- **Vector search** - Semantic similarity with embeddings
- **Real-time subscriptions** - Change notifications
- **Memory tiering** - Hot/Warm/Cold/Deep automatic management
- **WASM support** - Run in browsers with IndexedDB persistence
- **HTTP API** - RESTful endpoints for remote access
- **High performance** - ~200+ writes/sec, ~159K reads/sec
- **Query engine** - Filter, sort, project, and aggregate data
- **Thread-safe** - Concurrent operations with no data races
- **JSON native** - Store and query JSON documents naturally

## CLI Reference

```bash
# Install
cargo install koru-delta

# Basic Operations
kdelta set users/alice '{"name": "Alice", "age": 30}'
kdelta get users/alice
kdelta get users/alice --at "2026-02-04T12:00:00Z"  # Time travel
kdelta log users/alice              # Show history
kdelta diff users/alice             # Compare versions

# View Operations
kdelta view create active_users users --filter 'status = "active"'
kdelta view list
kdelta view refresh active_users
kdelta view query active_users
kdelta view delete active_users

# Query Operations
kdelta query users                              # Get all users
kdelta query users --filter 'age > 30'          # Filter
kdelta query users --sort name --limit 10       # Sort and limit
kdelta query users --count                      # Count records

# HTTP API Server
kdelta serve --port 8080

# Remote Operations
kdelta --url http://localhost:8080 get users/alice

# Database Info
kdelta status                       # Show database stats
kdelta list                         # List namespaces
```

## Examples

The `examples/` directory contains comprehensive demos:

```bash
# Full feature showcase - all v2.0.0 features
cargo run --example crisis_coordination_demo

# Distributed cluster validation
cargo run --example cluster_e2e_test

# Stress testing and edge cases
cargo run --example stress_test --release

# Original demos
cargo run --example ecommerce_demo
cargo run --example cluster_demo
```

## Performance (Validated)

From stress_test validation on macOS M1:

| Metric | Result |
|--------|--------|
| Write throughput | 200+ ops/sec |
| Read throughput | 158,859 reads/sec |
| Large values | Up to 100KB handled |
| Key capacity | 10,000+ keys tested |
| History depth | 100+ versions retained |
| Concurrent writers | 100 tasks, 0 conflicts |

## Architecture

KoruDelta is built on top of [koru-lambda-core](https://crates.io/crates/koru-lambda-core), a minimal axiomatic system for distributed computation. This gives KoruDelta:

- **Mathematical guarantees** - Safety and consistency from a formal foundation
- **Structural integrity** - Can't corrupt by design
- **Deterministic operations** - Same inputs always produce the same results
- **Natural distribution** - Consensus and sync emerge from the axioms

The math is your secret weapon, not your configuration burden.

### Persistence

KoruDelta uses a **Write-Ahead Log (WAL)** with content-addressed storage:

```
~/.korudelta/db/
├── wal/000001.wal          # Append-only log entries
└── values/ab/cd1234...     # Content-addressed values
```

**Why this matters:**
- **O(1) writes** - Append instead of rewriting
- **Crash-safe** - Never overwrites existing data
- **Automatic deduplication** - Identical values stored once
- **Immutable history** - The log is the history

## Project Stats

- **~15,000 lines** of Rust code
- **424 tests** (all passing)
- **0 clippy warnings**
- Cross-platform (Linux, macOS, Windows, WASM)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and [ARCHITECTURE.md](ARCHITECTURE.md) for technical details.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Links

- [GitHub Repository](https://github.com/swyrknt/koru-delta)
- [Design Document](DESIGN.md) - Design philosophy and decisions
- [Architecture](ARCHITECTURE.md) - Technical architecture
- [CLI Guide](CLI_GUIDE.md) - Complete command reference
- [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core) - The underlying distinction engine

---

*KoruDelta: Where data meets history, and simplicity meets power.*
