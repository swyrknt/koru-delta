# KoruDelta Design Philosophy

This document describes the design philosophy, core principles, and architectural decisions behind KoruDelta.

## Vision

KoruDelta is a **zero-configuration causal database** that combines:

- **Git-like versioning** - Every change is tracked in an immutable history
- **Redis-like simplicity** - Simple key-value API with no setup required
- **Distributed consistency** - Multi-node clusters that sync automatically

The goal is to make distributed, versioned data storage as easy as using a local hash map.

## Core Principles

### 1. Invisible Complexity

The underlying mathematical foundation (distinction calculus via koru-lambda-core) provides strong guarantees but should never be exposed to users.

**What users see:**
```rust
let db = KoruDelta::start().await?;
db.put("users", "alice", json!({"name": "Alice"})).await?;
```

**What happens internally:**
- JSON is serialized to bytes
- Bytes are mapped to distinctions
- Distinctions are synthesized into a content-addressed version ID
- Causal chains track the relationship between versions

Users don't need to understand this. They just store and retrieve data.

### 2. History as a First-Class Citizen

Unlike traditional databases where history is an afterthought (if available at all), KoruDelta treats history as fundamental:

- Every write creates a new version
- All versions are retained
- Time-travel queries are built-in
- Diffs between versions are trivial

This enables:
- Complete audit trails
- Easy debugging ("how did we get here?")
- Rollback capabilities
- Causal consistency guarantees

### 3. Zero Configuration

Starting a database should require exactly zero configuration:

```bash
kdelta start  # That's it
```

Joining a cluster should require exactly one piece of information:

```bash
kdelta start --join 192.168.1.100  # Join existing cluster
```

No config files, no schema definitions, no consensus tuning, no port configurations (unless you want them).

### 4. Universal Runtime

The same code runs everywhere:
- Linux, macOS, Windows
- Server, laptop, edge device
- Browser (via WASM)

This is achieved through Rust's cross-compilation and WASM support.

## API Design Principles

### Simplicity Over Power

Prefer simple APIs that cover 90% of use cases over complex APIs that cover 100%.

**Good:**
```rust
db.get("users", "alice").await?
```

**Avoided:**
```rust
db.get_with_options("users", "alice", GetOptions::builder()
    .consistency(ConsistencyLevel::Strong)
    .timeout(Duration::from_secs(5))
    .build())
```

### Async by Default

All public APIs are async, even if the current implementation is synchronous. This ensures:
- Future-proof for network operations
- Consistent API regardless of deployment mode
- Integration with async ecosystems (Tokio)

### Explicit Errors

All fallible operations return `Result<T, DeltaError>`. No silent failures, no panics in library code.

```rust
pub enum DeltaError {
    KeyNotFound { namespace: String, key: String },
    SerializationError(serde_json::Error),
    StorageError(String),
    // ...
}
```

## Architecture Decisions

### Why Content-Addressed Versioning?

Each version is identified by a SHA256 hash of its content. Benefits:
- **Deduplication**: Identical values share the same version ID
- **Integrity**: Corruption is detectable
- **Distribution**: Natural merge semantics for sync

### Why Immutable History?

All history is append-only. Benefits:
- **Audit**: Complete provenance of all changes
- **Time travel**: Query any historical state
- **Concurrency**: No locks needed for reads

### Why DashMap?

We use `DashMap` (lock-free concurrent hash map) for:
- Thread-safe concurrent access
- Better performance than `RwLock<HashMap>`
- Simpler code than manual locking

### Why JSON?

JSON as the data format because:
- Universal (every language has JSON support)
- Human-readable (easy debugging)
- Flexible (no schema required)
- Good enough performance for most use cases

## Development Phases

### Phase 1: Single Node (Complete)

Foundation with all core features:
- Put/Get/History operations
- Time travel queries
- Visual diffs
- CLI tool
- Disk persistence

### Phase 2: Distribution (Complete)

Multi-node clustering:
- Peer discovery via gossip
- Automatic data sync
- Cluster health monitoring
- Join/leave operations

### Phase 3: Advanced Features (Complete)

Query engine and real-time features:
- Filter, sort, project, aggregate
- Materialized views
- Real-time subscriptions
- History queries

### Future Considerations

Potential future enhancements:
- Pluggable storage backends (RocksDB, SQLite, S3)
- Conflict resolution strategies
- Schema validation (optional)
- Web dashboard
- Cloud-managed service

## Trade-offs

### Consistency vs Availability

KoruDelta prioritizes **consistency**. In a network partition:
- Writes may fail if sync cannot be verified
- Reads return the last known consistent state

### Memory vs Disk

Current implementation keeps working set in memory:
- Pros: Fast reads, simple implementation
- Cons: Limited by available RAM
- Future: Tiered storage with hot/cold separation

### Simplicity vs Features

We consciously limit features to maintain simplicity:
- No SQL (use the query API instead)
- No transactions (use versioning for consistency)
- No triggers (use subscriptions instead)

## References

- [ARCHITECTURE.md](ARCHITECTURE.md) - Technical implementation details
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core) - Underlying distinction engine
