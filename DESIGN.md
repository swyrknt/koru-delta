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

Each version has two identifiers:
- **`distinction_id`**: SHA256 hash of content (content-addressed)
- **`write_id`**: `{distinction_id}_{timestamp_nanos}` (write-addressed)

Benefits:
- **Deduplication**: Identical values share the same `distinction_id` (value store)
- **Complete History**: Every write has unique `write_id` (version store)
- **Integrity**: Corruption is detectable
- **Distribution**: Natural merge semantics for sync
- **Causal Chains**: `previous_version` links via `write_id` (not content hash)

### Why Immutable History?

All history is append-only with dual identification:
- **Value Store**: Maps `distinction_id` → value (deduplication)
- **Version Store**: Maps `write_id` → VersionedValue (complete history)

Benefits:
- **Audit**: Complete provenance of all changes (even rewrites of same value)
- **Time travel**: Query any historical state via causal graph traversal
- **Concurrency**: No locks needed for reads
- **Deduplication**: Same content stored once, referenced many times

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

## Distinction-Driven Architecture

KoruDelta is evolving toward a **distinction calculus system** that captures the emergent behavior of distinctions:

### Core Insight

The system doesn't just store data—it tracks the **becoming** of distinctions:
- **Synthesis**: New distinctions emerge from prior ones (causal graph)
- **Reference**: Distinctions point to other distinctions (reference graph)
- **Memory**: Like a brain, distinctions flow through layers (Hot → Warm → Cold → Deep)
- **Evolution**: Unfit distinctions are archived, essence is preserved (distillation)

### Two IDs, Two Purposes

```rust
struct VersionedValue {
    write_id: String,        // Unique per write: "{hash}_{timestamp_nanos}"
    distinction_id: String,  // Content hash: SHA256(value)
    previous_version: Option<String>, // Links via write_id
    // ...
}
```

- **`write_id`**: Enables complete history—even writing the same value 100 times creates 100 unique writes
- **`distinction_id`**: Enables deduplication—identical values share storage

### The Causal Graph

The causal graph is the **source of truth** for history:
- Nodes are `write_id`s (every write)
- Edges represent causality (parent → child)
- Traversal yields complete history
- Time travel queries navigate this graph

## Local Causal Agent (LCA) Design

KoruDelta implements the **Local Causal Agent** pattern, where every component is an agent with a local causal perspective in a unified field.

### The Core Formula

All operations follow:
```
ΔNew = ΔLocal_Root ⊕ ΔAction
```

This is not just documentation—it's the actual implementation pattern:

```rust
// Every agent has a local root (its causal perspective)
local_root: Distinction,

// Every operation synthesizes action with local root
let action_distinction = action.to_canonical_structure(engine);
let new_root = engine.synthesize(&local_root, &action_distinction);
self.local_root = new_root.clone();
```

### Why LCA?

**1. Deterministic Identity**
- Same action + same root = same distinction ID
- Content-addressed (Blake3 hash)
- No UUIDs, no randomness

**2. Complete Audit Trail**
- Every operation leaves a causal trace
- Query: "How did we get here?"
- Answer: Follow the synthesis chain

**3. Composable Agents**
- Agents combine through synthesis
- Cross-agent causality is natural
- `orchestrator.synthesize_cross_agent(&["agent1", "agent2"], action)`

**4. Universal Addressing**
- Distinction IDs are universal
- Same data = same ID on any node
- Natural for distributed systems

### Interior Mutability Pattern

For ergonomic APIs, agents use interior mutability:

```rust
// Internal: RwLock for local_root
local_root: RwLock<Distinction>,

// Public: &self API
pub fn do_something(&self, data: Data) -> Result<Distinction> {
    // Synthesize internally
    let new_root = self.synthesize_action(data)?;
    *self.local_root.write().unwrap() = new_root;
    Ok(new_root)
}
```

This preserves the simple `&self` API while following LCA internally.

### The Unified Field

All 21 agents share one `DistinctionEngine` (the "field"):

```
┌─────────────────────────────────────┐
│       DistinctionEngine             │  ← The unified field
│  (single instance, shared by all)   │
└─────────────────────────────────────┘
         │         │         │
    ┌────┘    ┌────┘    ┌────┘
┌───┴───┐ ┌───┴───┐ ┌───┴───┐
│Storage│ │Vector │ │Identity│  ← Agents with local roots
│ Agent │ │ Agent │ │ Agent  │
└───────┘ └───────┘ └────────┘
```

Each agent has its own `local_root` (perspective), but they all synthesize into the same field.

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
