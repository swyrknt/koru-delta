# KoruDelta Architecture

This document describes the internal architecture, design decisions, and coding standards for KoruDelta.

## Overview

KoruDelta is a zero-configuration causal database built on top of [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core). It provides Git-like versioning, Redis-like simplicity, and mathematical guarantees from distinction calculus.

## Architecture Layers

KoruDelta is architected in three clean layers:

```
┌─────────────────────────────────────────┐
│         KoruDelta Public API            │  ← Simple, async interface
│    (put, get, history, get_at)          │
├─────────────────────────────────────────┤
│        Causal Storage Layer             │  ← Versioning & history
│  (VersionedValue, causal chains)        │
├─────────────────────────────────────────┤
│      Distinction Engine (core)          │  ← Mathematical foundation
│  (DistinctionEngine, synthesis)         │
└─────────────────────────────────────────┘
```

### Layer 1: Public API (`src/core.rs`)

The user-facing interface that abstracts away all internal complexity.

**Key Components:**
- `KoruDelta` - Main database handle
- `DatabaseStats` - Metrics and monitoring

**Design Principles:**
- Async-first (future-proof for distributed operations)
- Simple method signatures (namespace, key, value)
- Hide mathematical concepts from users
- Thread-safe via `Arc` cloning

**Example:**
```rust
let db = KoruDelta::start().await?;
db.put("users", "alice", json!({"name": "Alice"})).await?;
let value = db.get("users", "alice").await?;
```

### Layer 2: Causal Storage (`src/storage.rs`)

Manages versioned key-value storage with complete causal history.

**Key Components:**
- `CausalStorage` - Storage engine
- `VersionedValue` - Value + metadata (timestamp, version ID, previous version)
- History log (append-only, per key)

**Design Principles:**
- Immutable history (append-only, never overwrite)
- Content-addressed versioning via distinctions
- Thread-safe concurrent access via `DashMap`
- Time-travel queries by traversing causal chains

**Data Structures:**
```rust
current_state: DashMap<FullKey, VersionedValue>  // Latest version per key
history_log: DashMap<FullKey, Vec<VersionedValue>> // All versions chronologically
```

**Version Linking:**
```
v1 ← v2 ← v3 ← v4 (current)
     ↑
     previous_version links
```

### Layer 3: Document Mapping (`src/mapper.rs`)

Bridge between JSON data and distinction structures.

**Key Components:**
- `DocumentMapper` - Stateless conversion utility

**Algorithm:**
1. Serialize JSON → canonical bytes
2. Map each byte → distinction (cached O(1) lookup)
3. Fold distinctions → single root distinction (deterministic)

**Properties:**
- Same JSON → same distinction ID (content-addressed)
- Deterministic (order-independent for objects, order-dependent for arrays)
- Efficient via koru-lambda-core's byte caching

### Foundation: Distinction Engine

Provided by `koru-lambda-core`, this gives us:

- **Deterministic synthesis**: `synthesize(a, b)` always produces same result
- **Content addressing**: Distinction IDs are SHA256 hashes
- **Thread-safety**: Lock-free concurrent operations via `DashMap`
- **Mathematical guarantees**: Five core axioms ensure consistency

## Key Data Types

### `FullKey` (`src/types.rs`)

Combines namespace + key into a single identifier.

```rust
FullKey {
    namespace: "users",
    key: "alice"
}
// Canonical: "users:alice"
```

### `VersionedValue` (`src/types.rs`)

Every write creates a versioned entry:

```rust
VersionedValue {
    value: JsonValue,           // The actual data
    timestamp: DateTime<Utc>,   // When written
    version_id: String,         // Content-addressed ID (distinction)
    previous_version: Option<String>, // Causal link
}
```

### `HistoryEntry` (`src/types.rs`)

Simplified view for history queries (omits previous_version link).

## Concurrency Model

### Thread Safety

All core structures are thread-safe:

- `DistinctionEngine` - Uses `DashMap` for lock-free synthesis
- `CausalStorage` - Uses `DashMap` for lock-free state updates
- `KoruDelta` - Uses `Arc` for cheap cloning across threads

### Concurrent Writes

**Same Key:**
- Multiple threads can write to the same key concurrently
- Each write appends to the history log (thread-safe)
- Causal chain is maintained correctly via `DashMap` atomic operations
- All writes are recorded, none are lost

**Different Keys:**
- Fully parallel, no contention
- Each key has independent history

### Memory Model

- **Shared engine**: Single `DistinctionEngine` shared via `Arc`
- **Shared storage**: Single `CausalStorage` shared via `Arc`
- **Clone semantics**: `KoruDelta::clone()` is cheap (Arc increment)

## Error Handling

All errors use the `DeltaError` enum for type-safe matching.

### Error Types

```rust
pub enum DeltaError {
    KeyNotFound { namespace, key },
    NoValueAtTimestamp { namespace, key, timestamp },
    SerializationError(serde_json::Error),
    InvalidData { reason },
    EngineError(String),
    StorageError(String),
    TimeError(String),
}
```

### Error Philosophy

- **Explicit errors**: No silent failures
- **Rich context**: Errors include relevant metadata (namespace, key, etc.)
- **Pattern matching**: Users can match on specific error variants
- **No panics**: Public API never panics (except for unrecoverable bugs)

## Code Style Guidelines

### General Principles

1. **Simplicity over cleverness** - Straightforward code beats clever code
2. **Documentation first** - Every public item has docs
3. **Test everything** - Comprehensive unit + integration tests
4. **Hide complexity** - Mathematical concepts stay internal
5. **Thread-safe by default** - All structures support concurrent access

### Documentation Standards

```rust
/// One-line summary (ends with period).
///
/// Detailed explanation with:
/// - Use cases
/// - Examples
/// - Thread safety notes
/// - Performance characteristics
///
/// # Example
///
/// ```ignore
/// let result = function_name(args);
/// ```
pub fn function_name() { }
```

### Naming Conventions

- **Modules**: `snake_case` (e.g., `causal_storage`)
- **Types**: `PascalCase` (e.g., `VersionedValue`)
- **Functions**: `snake_case` (e.g., `get_at`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_HISTORY`)

### Module Organization

```
src/
├── lib.rs          # Public API exports, crate docs
├── core.rs         # KoruDelta main implementation
├── storage.rs      # CausalStorage implementation
├── mapper.rs       # DocumentMapper implementation
├── types.rs        # Shared data structures
└── error.rs        # Error types
```

### Testing Strategy

**Unit Tests:**
- Located in same file as implementation (`#[cfg(test)] mod tests`)
- Test individual functions and edge cases
- Fast, isolated, deterministic

**Integration Tests:**
- Located in `tests/` directory
- Test end-to-end workflows
- Test concurrency scenarios
- Test error conditions

**Test Naming:**
```rust
#[test]
fn test_<feature>_<scenario>() {
    // Arrange
    let db = setup();

    // Act
    let result = db.operation();

    // Assert
    assert_eq!(result, expected);
}
```

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `put()` | O(n) | n = bytes in value (JSON serialization + mapping) |
| `get()` | O(1) | HashMap lookup |
| `get_at()` | O(h) | h = history size (linear scan backward) |
| `history()` | O(h) | h = history size (copy all versions) |
| `contains()` | O(1) | HashMap lookup |

### Space Complexity

- **Per key**: O(h) where h = number of versions
- **Total**: O(k × h̄) where k = keys, h̄ = average history size

### Optimization Opportunities

Future optimizations (not yet implemented):
- **History indexing**: Binary search for time-travel queries → O(log h)
- **Compaction**: Merge old versions to reduce history size
- **Lazy loading**: Stream history instead of loading all versions

## Phase 2: Distribution (Complete)

Multi-node clustering is now implemented:

### Architecture

```
┌─────────────────────────────────────────┐
│         KoruDelta Public API            │  ← Unchanged
├─────────────────────────────────────────┤
│           Cluster Layer                 │  ← NEW
│   (ClusterNode, PeerManager, Sync)      │
├─────────────────────────────────────────┤
│        Causal Storage Layer             │  ← Minor updates
├─────────────────────────────────────────┤
│      Distinction Engine (core)          │  ← Unchanged
└─────────────────────────────────────────┘
```

### New Modules

- `network.rs` - TCP communication, message protocol
- `cluster.rs` - ClusterNode, peer management, gossip

### Features

1. **Join**: `kdelta start --join <ip>` joins an existing cluster
2. **Discovery**: Gossip protocol shares peer information
3. **Sync**: Full snapshot sync on join, incremental broadcast for writes
4. **Health**: Heartbeat pings track peer status

### Usage

```bash
# Start first node
kdelta start --port 7878

# Join from another machine
kdelta start --join 192.168.1.100:7878
```

## Phase 3: Queries, Views, and Subscriptions (Complete)

Phase 3 adds powerful query capabilities:

### Architecture

```
┌─────────────────────────────────────────┐
│         KoruDelta Public API            │  ← Extended with query/view/subscription
│    (query, create_view, subscribe)      │
├─────────────────────────────────────────┤
│          Query Layer                    │  ← NEW
│   (Filter, Query, Aggregation)          │
├─────────────────────────────────────────┤
│          Views Layer                    │  ← NEW
│   (ViewManager, ViewDefinition)         │
├─────────────────────────────────────────┤
│       Subscriptions Layer               │  ← NEW
│   (SubscriptionManager, ChangeEvent)    │
├─────────────────────────────────────────┤
│           Cluster Layer                 │  ← Unchanged
├─────────────────────────────────────────┤
│        Causal Storage Layer             │  ← Extended with scan_collection
├─────────────────────────────────────────┤
│      Distinction Engine (core)          │  ← Unchanged
└─────────────────────────────────────────┘
```

### New Modules

- `query.rs` - Filter, Query, Aggregation, QueryExecutor
- `views.rs` - ViewDefinition, ViewManager, ViewData
- `subscriptions.rs` - Subscription, SubscriptionManager, ChangeEvent

### Features

1. **Query Engine**
   - Filter: Eq, Ne, Gt, Gte, Lt, Lte, Contains, Exists, Matches, And, Or, Not
   - Projection: Select specific fields
   - Sorting: Ascending/descending by field
   - Limiting: Offset and limit results
   - Aggregation: Count, Sum, Avg, Min, Max, Distinct, GroupBy

2. **Materialized Views**
   - Create views with query definitions
   - Auto-refresh on writes (optional)
   - Manual refresh on demand
   - List and delete views

3. **Subscriptions**
   - Subscribe to all changes
   - Subscribe to specific collection/key
   - Filter by change type (insert, update, delete)
   - Filter by value conditions
   - `put_notify()` for writes with notifications

### Usage

```rust
// Query
let results = db.query("users", Query::new()
    .filter(Filter::gt("age", 30))
    .sort_by("name", true)
    .limit(10)
).await?;

// Views
db.create_view(ViewDefinition::new("active_users", "users")
    .with_query(Query::new().filter(Filter::eq("status", "active")))
).await?;

// Subscriptions
let (id, mut rx) = db.subscribe(Subscription::collection("users")).await;
```

## Future Extensions

### Phase 4: Cloud & Deployment

Potential features:
- Managed cloud service
- Kubernetes operator
- Auto-scaling clusters
- Multi-region replication

### Phase 5: Storage Backends

Current: In-memory only

Future: Pluggable backends
- Disk persistence (RocksDB, SQLite)
- Cloud storage (S3, etc.)
- Maintain same API regardless of backend

## Design Decisions

### Why Async?

Even though Phase 1 is synchronous under the hood, we use async APIs because:

1. **Future-proof**: Distribution (Phase 2) will be naturally async
2. **Consistency**: Same API from development to production
3. **Ecosystem**: Integrates with Tokio ecosystem

### Why Content-Addressed Versioning?

Distinction IDs (SHA256 hashes) provide:

1. **Deduplication**: Identical values share the same distinction
2. **Integrity**: Can verify data hasn't been corrupted
3. **Distribution**: Natural merge semantics for distributed sync

### Why Immutable History?

Append-only history gives us:

1. **Audit trails**: Complete provenance of all changes
2. **Time travel**: Query any point in the past
3. **Debugging**: Understand how state evolved
4. **Causal consistency**: Clear ordering of events

### Why Hide the Math?

The distinction calculus is powerful but abstract. We hide it because:

1. **Accessibility**: More developers can use the database
2. **Simplicity**: Users think in terms of keys and values
3. **Flexibility**: Can change internal implementation
4. **Marketing**: "Just works" is better than "read this paper"

The math is our **secret weapon**, not our pitch.

## Getting Started (Development)

### Build

```bash
cargo build
```

### Test

```bash
cargo test
cargo test --release  # Optimized
```

### Documentation

```bash
cargo doc --open
```

### Benchmarks

```bash
cargo bench
```

Performance characteristics (benchmarked):
- **Read latency**: ~340ns (2.9M ops/sec)
- **Write throughput**: ~27K ops/sec
- **History query**: 4.3M elements/sec

## Contributing Guidelines

When adding new features:

1. **Design first**: Update this document with your design
2. **Test first**: Write tests before implementation
3. **Document**: Add comprehensive docs
4. **Simplify**: Can this API be simpler?
5. **Verify**: All tests must pass

For questions, see [DESIGN.md](DESIGN.md) for the product vision.

## References

- [DESIGN.md](DESIGN.md) - Product vision and roadmap
- [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core) - Underlying engine
- [README.md](README.md) - User-facing introduction
