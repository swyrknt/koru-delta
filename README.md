# KoruDelta — The Invisible Database

[![Crates.io](https://img.shields.io/crates/v/koru-delta.svg)](https://crates.io/crates/koru-delta)
[![Docs.rs](https://docs.rs/koru-delta/badge.svg)](https://docs.rs/koru-delta)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**Tagline:** *"Invisible. Causal. Everywhere."*

**One-line:** *KoruDelta gives you Git-like history, Redis-like speed, and distributed consistency—without configuration.*

## What Makes It Different?

| Feature | SQLite | Redis | PostgreSQL | **KoruDelta** |
|---------|--------|-------|------------|---------------|
| Zero-config | ✅ | ❌ | ❌ | ✅ |
| Time travel / audit | ❌ | ❌ | ❌ (complex) | ✅ Built-in |
| Vector search | ❌ (extension) | ❌ | ✅ (pgvector) | ✅ Native |
| Causal consistency | ❌ | ❌ | ❌ | ✅ |
| Binary size | ~1MB | ~10MB | ~100MB | **~11MB** |
| History retention | ❌ | ❌ | ❌ | ✅ Unlimited |
| Materialized views | ❌ | ❌ | ✅ | ✅ Native |

**KoruDelta isn't just another database—it's a *time-aware* database.**

Every write is versioned. Query data as it existed 5 minutes ago. Compare versions with Git-style diffs. Build audit trails without application changes.

## Quick Start

```bash
# Install (10 seconds)
cargo install koru-delta

# Use (0 configuration)
kdelta set users/alice '{"name": "Alice", "age": 30}'
kdelta get users/alice
kdelta log users/alice          # Full history
kdelta get users/alice --at "2026-02-01T00:00:00Z"  # Time travel
```

```rust
use koru_delta::KoruDelta;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = KoruDelta::start().await?;
    
    // Single write
    db.put("users", "alice", serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com"
    })).await?;
    
    // Batch write (10-50x faster for bulk operations)
    db.put_batch(vec![
        ("products", "p1", serde_json::json!({"name": "Widget", "price": 9.99})),
        ("products", "p2", serde_json::json!({"name": "Gadget", "price": 19.99})),
    ]).await?;
    
    // Get current value
    let user = db.get("users", "alice").await?;
    
    // Get history
    let history = db.history("users", "alice").await?;
    
    // Time travel
    let past_user = db.get_at("users", "alice", timestamp).await?;
    
    Ok(())
}
```

## Performance

**Real benchmarks** (MacBook Pro M1, 16GB RAM, SSD):

| Metric | KoruDelta | SQLite (fsync) | Notes |
|--------|-----------|----------------|-------|
| **Writes (single)** | ~201 ops/sec | ~3,700 ops/sec | KoruDelta: WAL + versioning |
| **Writes (batch)** | ~3,500 ops/sec* | N/A | 16x faster with `put_batch` |
| **Reads** | ~134K ops/sec | ~267K ops/sec | Hot memory cache |
| **Binary size** | 11MB | 1MB | Single static binary |
| **Memory** | Configurable | Configurable | 512MB default |

\* Batch write: 1000 items in 280ms vs 4.66s for individual writes

**Why slower writes?** KoruDelta does more:
- Every write creates a version (immutable history)
- Content-addressed deduplication (Blake3 hashes)
- Causal graph tracking
- Automatic memory tier promotion

**Batch writes** (`put_batch`) amortize fsync cost across many items, delivering 10-50x speedups for bulk operations.

**Trade-off:** Speed for superpowers. If you need audit trails, time travel, or causal consistency, KoruDelta eliminates weeks of application development.

## Core Features

### 🕰️ Time Travel (Built-in)

```rust
// Every change is versioned forever
let history = db.history("users", "alice").await?;

// Query past state
let past = db.get_at("users", "alice", yesterday).await?;

// Compare versions
kdelta diff users/alice
```

**Use cases:** Audit trails, compliance (HIPAA/GDPR), debugging, undo/redo.

### 📊 Materialized Views (v2.0.0)

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
let results = db.query_view("active_users").await?;  // Instant
```

Views persist across restarts and auto-refresh on writes.

### 🔍 Vector Search (v2.0.0)

```rust
use koru_delta::vector::Vector;

// Store embedding
let embedding = Vector::new(vec![0.1, 0.2, 0.3, ...], "text-embedding-3-small");
db.embed("docs", "doc1", embedding, None).await?;

// Semantic search
let results = db.embed_search(
    Some("docs"), 
    &query_vector,
    VectorSearchOptions { top_k: 10, threshold: 0.0, model_filter: None }
).await?;
```

Build RAG applications, semantic document search, recommendation engines.

### 🔔 Real-time Subscriptions

```rust
let (sub_id, mut rx) = db.subscribe(Subscription {
    collection: Some("orders".to_string()),
    key: None,
    filter: None,
    change_types: vec![ChangeType::Insert, ChangeType::Update],
    name: Some("order-monitor".to_string()),
}).await;

while let Ok(event) = rx.recv().await {
    println!("New order: {}", event.key);
}
```

### 🔐 Self-Sovereign Auth

```rust
use koru_delta::auth::{mine_identity, IdentityUserData};

// Mine an identity (proof-of-work prevents spam)
let identity = mine_identity(
    IdentityUserData::new("alice"),
    4  // difficulty: 4 leading hex zeros
).await;

// identity.id = public key
// identity.secret_key = private key (keep secure!)
```

No central authority. Users own their keys.

### 🌐 Browser/WASM

```javascript
import init, { KoruDeltaWasm } from 'koru-delta';

await init();

// Persistent database (IndexedDB)
const db = await KoruDeltaWasm.newPersistent();

await db.put('users', 'alice', { name: 'Alice', age: 30 });
const user = await db.get('users', 'alice');

// Batch writes for better performance (10-50x faster)
await db.putBatch([
    { namespace: 'users', key: 'alice', value: { name: 'Alice' } },
    { namespace: 'users', key: 'bob', value: { name: 'Bob' } },
]);

// Data survives page refreshes!
```

## When to Use KoruDelta

### ✅ Perfect For

| Use Case | Why KoruDelta Wins |
|----------|-------------------|
| **Audit-heavy apps** | Built-in versioning, no schema changes |
| **Local-first software** | Works offline, syncs when online |
| **Edge/IoT** | 11MB binary, survives power loss |
| **AI agents** | Vector search + memory tiering |
| **Config management** | Time travel, easy rollbacks |
| **Compliance** | Immutable history, cryptographic proofs |

### ⚠️ Not For

| Use Case | Use Instead |
|----------|-------------|
| 100K+ writes/sec analytics | ClickHouse, TimescaleDB |
| Complex SQL JOINs | PostgreSQL |
| Multi-region active-active | CockroachDB, Spanner |
| Pure caching | Redis |

## Architecture

### The Secret: Distinction Calculus

KoruDelta is built on [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core)—a minimal axiomatic system for distributed computation:

- **Mathematical guarantees** - Safety from formal foundations
- **Structural integrity** - Can't corrupt by design  
- **Deterministic operations** - Same inputs → same results
- **Natural distribution** - Consensus emerges from axioms

### Storage: WAL + Content-Addressed

```
~/.korudelta/db/
├── wal/000001.wal          # Append-only log (immutable)
└── values/ab/cd1234...     # Deduplicated by content hash
```

**Benefits:**
- O(1) writes (append, not rewrite)
- Crash-safe (never overwrites data)
- Automatic deduplication
- The log IS the history

### Memory: Brain-Inspired Tiering

| Tier | Capacity | Access | Eviction |
|------|----------|--------|----------|
| **Hot** | 10K items | ~400ns | LRU |
| **Warm** | Recent chronicle | ~1µs | Age-based |
| **Cold** | Consolidated epochs | ~10µs | Fitness score |
| **Deep** | Genomic (1KB) | Load on demand | Manual |

Like human memory: frequently used items stay hot, patterns consolidate to deep storage.

## CLI Reference

```bash
# Basic CRUD
kdelta set users/alice '{"name": "Alice"}'
kdelta get users/alice
kdelta delete users/alice

# Time travel
kdelta get users/alice --at "2026-02-01T00:00:00Z"
kdelta log users/alice              # History
kdelta diff users/alice             # Compare versions

# Views
kdelta view create active_users users --filter 'status = "active"'
kdelta view list
kdelta view query active_users
kdelta view refresh active_users

# Queries
kdelta query users --filter 'age > 30' --sort name --limit 10
kdelta query sales --sum amount      # Aggregation

# HTTP API
kdelta serve --port 8080
kdelta --url http://localhost:8080 get users/alice

# Cluster (experimental)
kdelta start --join 192.168.1.100
```

## Examples

```bash
# Full feature showcase
cargo run --example crisis_coordination_demo

# Distributed cluster validation  
cargo run --example cluster_e2e_test

# Stress testing
cargo run --example stress_test --release

# Original demos
cargo run --example ecommerce_demo
```

## Project Stats

- **15,000+ lines** Rust code
- **424 tests** (all passing)
- **0 compiler warnings**
- **11MB** binary size
- Cross-platform: Linux, macOS, Windows, WASM

## Distributed Status

**Current (v2.0.0):** Single-node production ready

**Multi-node clustering:** Infrastructure exists, HTTP broadcast gap. Planned for v2.1.0.

| Feature | Status |
|---------|--------|
| Node discovery | ✅ Working |
| Initial sync on join | ✅ Working |
| Live replication | ⚠️ Gap (v2.1.0) |
| Gossip protocol | ✅ Working |

## Security

- **Auth:** Proof-of-work identity mining (prevents spam)
- **Crypto:** Ed25519 signatures, Blake3 hashing
- **Model:** Self-sovereign (users own keys, no central auth server)
- **TLS:** Recommended for HTTP API (use reverse proxy)

## Operations

```bash
# Resource limits (via code)
let config = CoreConfig {
    memory: MemoryConfig {
        hot_capacity: 10000,
        max_memory_mb: 512,
        ..Default::default()
    },
    resource_limits: ResourceLimits {
        max_disk_mb: 10 * 1024,  // 10GB
        max_open_files: 256,
        max_connections: 100,
        ..Default::default()
    },
    ..Default::default()
};

# Logging
export KORU_LOG=info  # error, warn, info, debug, trace
```

**Monitoring:** Structured logs via `tracing`. Prometheus metrics planned for v2.1.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [ARCHITECTURE.md](ARCHITECTURE.md).

## License

MIT OR Apache-2.0

## Links

- [GitHub](https://github.com/swyrknt/koru-delta)
- [Design](DESIGN.md) - Philosophy and decisions
- [Architecture](ARCHITECTURE.md) - Technical deep dive
- [CLI Guide](CLI_GUIDE.md) - Complete command reference

---

*KoruDelta: Where data meets history, and simplicity meets power.*
