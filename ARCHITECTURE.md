# KoruDelta Architecture

This document describes the internal architecture, design decisions, and coding standards for KoruDelta.

## Overview

KoruDelta is a zero-configuration causal database built on top of [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core). It provides Git-like versioning, Redis-like simplicity, and mathematical guarantees from distinction calculus.

## Architecture Layers

KoruDelta is architected in layers that enable distinction-driven operations:

```
┌─────────────────────────────────────────┐
│         KoruDelta Public API            │  ← Simple, async interface
│    (put, get, history, get_at)          │
├─────────────────────────────────────────┤
│      Vector Search Layer (v2)           │  ← AI/ML embeddings
│  (SNSW, HNSW, CausalIndex)              │
├─────────────────────────────────────────┤
│       Auth Layer (v2)                   │  ← Self-sovereign identity
│  (Identity, Session, Capability)        │
├─────────────────────────────────────────┤
│      Reconciliation Layer (v2)          │  ← Distributed sync
│  (MerkleTree, BloomFilter, WorldRec)    │
├─────────────────────────────────────────┤
│      Evolutionary Processes (v2)        │  ← Automated management
│  (Consolidation, Distillation, Genome)  │
├─────────────────────────────────────────┤
│       Memory Tiering (v2)               │  ← Hot/Warm/Cold/Deep
│  (LRU cache, chronicle, epochs, DNA)    │
├─────────────────────────────────────────┤
│        Causal Storage Layer             │  ← Versioning & history
│  (CausalGraph, ReferenceGraph)          │
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

### Layer 2: Vector Search (`src/vector/`)

Native vector storage and similarity search for AI embeddings. **Fully implemented in Phase 4.**

**Key Components:**
- `Vector` - Embedding with model metadata and serialization
- `VectorStorage` trait - Extension trait for `KoruDelta` adding `embed()` and `embed_search()`
- `SNSW` (Synthesis-Navigable Small World) - Distinction-based ANN search
- `HNSW` (Hierarchical Navigable Small World) - Traditional ANN implementation
- `CausalVectorIndex` - Time-travel vector search (query historical embeddings)

**Design Principles:**
- **Content-addressed**: Blake3 hash = vector identity (automatic deduplication)
- **Causal-aware**: Vector history preserved like any data
- **Time-travel search**: `similar_at()` queries embeddings at past timestamps
- **Model-agnostic**: Works with OpenAI, local models, any embedding

**Search Tiers (SNSW):**
```
🔥 Hot (Exact Cache)    → O(1) hash lookup
🌤️ Warm-Fast           → Beam search, low ef
🌤️ Warm-Thorough       → Deep synthesis navigation
❄️ Cold (Exact)        → Linear scan with proximity
```

**API Example:**
```rust
// Store embedding
let embedding = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
db.embed("documents", "doc1", embedding, Some(json!({"title": "AI"}))).await?;

// Search similar vectors
let query = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
let results = db.embed_search(Some("documents"), &query, VectorSearchOptions::new().top_k(5)).await?;

// Time-travel search (what was similar yesterday?)
let past_results = db.similar_at(Some("documents"), &query, "2026-02-01T00:00:00Z", opts).await?;
```

### Layer 3: Causal Storage (`src/storage.rs`)

Manages versioned key-value storage with complete causal history.

**Key Components:**
- `CausalStorage` - Storage engine with causal graph
- `VersionedValue` - Value + dual IDs (write_id, distinction_id)
- `CausalGraph` - Tracks all writes and their causal relationships

**Design Principles:**
- Immutable history (append-only, never overwrite)
- **Dual identification**: `write_id` (unique per write) + `distinction_id` (content hash)
- Thread-safe concurrent access via `DashMap`
- Time-travel queries by traversing causal graph

**Data Structures:**
```rust
current_state: DashMap<FullKey, VersionedValue>   // Latest version per key
version_store: DashMap<WriteId, VersionedValue>   // All versions by write_id
causal_graph: CausalGraph                         // Causal relationships
value_store: DashMap<DistinctionId, Arc<Value>>   // Deduplicated values
```

**Version Linking:**
```
write_1 ← write_2 ← write_3 ← write_4 (current)
   ↑         ↑         ↑
distinction_id: hash(content)
write_id: hash + timestamp_nanos
previous_version: links via write_id
```

### Layer 4: Document Mapping (`src/mapper.rs`)

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

### Layer 5: Memory Tiering ✅ (`src/memory/`)

Brain-like memory hierarchy for efficient resource usage. **Fully implemented in Phase 7.**

**Components:**
- `HotMemory` - LRU cache for frequently accessed distinctions (ACTIVE)
- `WarmMemory` - Recent chronicle with idle detection (ACTIVE)
- `ColdMemory` - Consolidated epochs with fitness filtering (ACTIVE)
- `DeepMemory` - Genomic storage for 1KB portable backups (ACTIVE)

**GET Cascade with Promotion:**
```
User → get(key)
          ↓
     HotMemory? → Return (<1ms)
          ↓ No
     WarmMemory? → Promote to Hot → Return
          ↓ No
     ColdMemory? → Promote through tiers → Return
          ↓ No
     CausalStorage → Promote to Hot → Return
```

**Background Processes:**
- **Consolidation** (5 min): Hot ↔ Warm ↔ Cold ↔ Deep
- **Distillation** (1 hour): Fitness-based selection
- **Genome Update** (daily): Extract causal topology

### Layer 6: Evolutionary Processes ✅ (`src/processes/`)

Automated memory management through natural selection. **Running in Phase 7.**

**Components:**
- `ConsolidationProcess` - Rhythmic movement between memory layers (5 min interval)
- `DistillationProcess` - Fitness-based natural selection (1 hour interval)
- `GenomeUpdateProcess` - DNA maintenance and disaster recovery (daily interval)

**Analogy:** Like sleep consolidating memories—unfit distinctions are archived, essence is preserved.

**Implementation:**
```rust
// Spawned on KoruDelta initialization
tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = interval.tick() => run_consolidation(),
            _ = shutdown.changed() => break,
        }
    }
});
```

### Layer 7: Reconciliation (`src/reconciliation/`)

Efficient distributed sync via set reconciliation.

**Components:**
- `MerkleTree` - Hash tree for O(log n) set comparison
- `BloomFilter` - Probabilistic membership testing
- `WorldReconciliation` - Protocol for merging causal graphs

**Protocol:**
```
1. Exchange Merkle roots
2. If different, drill down to find differences
3. Send only missing distinctions
4. Merge causal graphs (conflicts become branches)
```

### Layer 8: Auth Layer (`src/auth/`)

Self-sovereign identity and capability-based authorization using distinctions.

**Key Components:**
- `Identity` - Mined identity with Ed25519 keys and proof-of-work
- `Session` - Authenticated session with derived encryption keys
- `Capability` - Signed permission grants (granter → grantee)
- `AuthManager` - High-level authentication coordinator

**Design Principles:**
- **Self-sovereign**: Users generate and own their keys
- **Distinction-based**: Auth state stored as `_auth:*` distinctions
- **Capability-based**: No roles, only explicit permission grants
- **Reconcilable**: Auth state syncs between nodes like any data

**Storage Layout:**
```
_auth:identity:{pubkey}      → Identity (mined, proof-of-work)
_auth:capability:{id}        → Capability (signed grant)
_auth:revocation:{cap_id}    → Revocation (tombstone)
```

**Authentication Flow:**
```
1. Mine identity (proof-of-work, ~1s)
2. Store identity as distinction
3. Request challenge (ephemeral, 5min TTL)
4. Sign challenge with private key
5. Verify signature, create session
6. Session keys derived via HKDF-SHA256
```

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

Every write creates a versioned entry with dual identification:

```rust
VersionedValue {
    value: JsonValue,               // The actual data (Arc-wrapped)
    timestamp: DateTime<Utc>,       // When written (nanosecond precision)
    write_id: String,               // Unique per write: "{hash}_{timestamp_nanos}"
    distinction_id: String,         // Content hash (SHA256)
    previous_version: Option<String>, // Causal link via write_id
}
```

**Dual ID Design:**
- `write_id` enables **complete history**—writing the same value 100 times = 100 unique writes
- `distinction_id` enables **deduplication**—same content shares storage in value_store
- `version_id()` returns `distinction_id` for content-addressing compatibility

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
├── lib.rs              # Public API exports, crate docs
├── core.rs             # KoruDelta main implementation
├── storage.rs          # CausalStorage implementation
├── causal_graph.rs     # Causal graph tracking
├── reference_graph.rs  # Reference tracking for GC
├── mapper.rs           # DocumentMapper implementation
├── types.rs            # Shared data structures
├── error.rs            # Error types
├── memory/             # Memory tiering (v2)
│   ├── hot.rs          # Hot memory (LRU cache)
│   ├── warm.rs         # Warm memory (chronicle)
│   ├── cold.rs         # Cold memory (epochs)
│   └── deep.rs         # Deep memory (genomic)
├── processes/          # Evolutionary processes (v2)
│   ├── consolidation.rs
│   ├── distillation.rs
│   └── genome_update.rs
├── reconciliation/     # Set reconciliation (v2)
│   ├── mod.rs          # ReconciliationManager
│   ├── merkle.rs       # Merkle trees
│   ├── bloom.rs        # Bloom filters
│   └── world.rs        # World reconciliation
└── auth/               # Self-sovereign authentication (v2)
    ├── mod.rs          # Public API exports
    ├── types.rs        # Identity, Session, Capability
    ├── identity.rs     # Proof-of-work mining
    ├── verification.rs # Challenge-response
    ├── session.rs      # Session management
    ├── capability.rs   # Permission grants
    ├── storage.rs      # Storage adapter
    └── manager.rs      # High-level API
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
| `get_at()` | O(h) | h = history size (causal graph traversal) |
| `history()` | O(h) | h = history size |
| `contains()` | O(1) | HashMap lookup |
| **Reconciliation** |||
| `MerkleTree::diff()` | O(log n) | n = distinctions (best case) |
| `BloomFilter::might_contain()` | O(1) | k hash functions |
| `WorldReconciliation::reconcile()` | O(d) | d = differences |
| **Memory Tiering** |||
| `HotMemory::get()` | O(1) | LRU cache |
| `WarmMemory::get()` | O(1) | HashMap + disk |
| `ColdMemory::consolidate()` | O(n) | n = distinctions to consolidate |

### Space Complexity

- **Per key**: O(h) where h = number of versions
- **Total**: O(k × h̄) where k = keys, h̄ = average history size
- **Bloom filter**: O(-n·ln(p)/ln²(2)) bits for n items at FP rate p
- **Merkle tree**: O(n) nodes for n distinctions

### Sync Efficiency

| Scenario | Data Transferred | Efficiency |
|----------|-----------------|------------|
| Identical sets | 32 bytes (root hash) | 100% |
| 1% difference | ~1% of data + tree overhead | 99% |
| 50% difference | ~50% of data | 50% |
| Bloom filter (1% FP) | ~1KB for 10K items | ~99% |

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
