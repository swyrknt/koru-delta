# The Causal Database

**A New Category of Data Storage**

KoruDelta is not just another database. It's a fundamentally different way of thinking about data—one that treats time, provenance, and relationships as first-class citizens rather than afterthoughts.

> *"Traditional databases store the current state. KoruDelta stores causality."*

---

## Table of Contents

1. [What is a Causal Database?](#what-is-a-causal-database)
2. [Core Concepts](#core-concepts)
3. [Why Causality Matters](#why-causality-matters)
4. [Getting Started](#getting-started)
5. [Use Cases](#use-cases)
6. [Architecture Deep Dive](#architecture-deep-dive)
7. [Comparison with Other Databases](#comparison)
8. [Best Practices](#best-practices)

---

## What is a Causal Database?

A causal database is a storage system that:

1. **Tracks provenance** — Where did this data come from?
2. **Preserves history** — What was the state at any point in time?
3. **Understands relationships** — How does this data connect to other data?
4. **Manages lifecycle** — When should data be forgotten?

Traditional databases optimize for the **present**. Causal databases optimize for **understanding**.

### The Git Analogy

Think of KoruDelta like Git, but for structured data:

| Git | KoruDelta |
|-----|-----------|
| Commits | Versions |
| Branches | Namespaces |
| Diff | History queries |
| Blame | Provenance tracking |
| Merge | Conflict resolution |
| .git directory | Built-in storage |

Just as Git revolutionized code by making history and collaboration first-class, KoruDelta revolutionizes data by making causality first-class.

---

## Core Concepts

### 1. Distinctions (Identity through Content)

In KoruDelta, identity comes from **content**, not location.

```rust
// Two values with the same content have the same identity
let a = json!({"name": "Alice", "age": 30});
let b = json!({"name": "Alice", "age": 30});

// a and b are the same distinction
assert_eq!(distinction_id(&a), distinction_id(&b));
```

This content-addressing enables:
- **Automatic deduplication** — Same content stored once
- **Immutable history** — Content can't change, only be superseded
- **Verifiable provenance** — Hash chains guarantee integrity

### 2. Synthesis (Relationships through Combination)

Data doesn't exist in isolation. Synthesis tracks how distinctions combine:

```rust
// When we combine user + order, we create a synthesis
let user = db.get("users", "alice").await?;
let order = db.get("orders", "ord-123").await?;

// The relationship is preserved
db.synthesize("user_order", &user, &order).await?;
```

### 3. Causal Graph (Time with Meaning)

Every change records not just *when* it happened, but *why*:

```rust
// Each version links to its predecessor
Version {
    value: {...},
    timestamp: "2026-02-07T12:00:00Z",
    previous_version: Some("hash-of-previous"),
    caused_by: Some("transaction-id"),
}
```

This creates an immutable causal chain that can be traversed in either direction.

### 4. Memory Tiers (Natural Lifecycle)

Data has a natural lifecycle. KoruDelta manages this automatically:

| Tier | Capacity | Access Time | Use Case |
|------|----------|-------------|----------|
| **Hot** | ~10K items | ~400ns | Active working set |
| **Warm** | ~1M items | ~1ms | Recent history |
| **Cold** | Unlimited | ~10ms | Archive |
| **Deep** | Unlimited | ~100ms+ | Genomic/epoch data |

```rust
// Data flows naturally between tiers
Hot → Warm → Cold → Deep
  ↑_________________|
  (reactivation on access)
```

---

## Why Causality Matters

### Scenario 1: The Production Incident

**Traditional Database:**
```
PM: "What was the config during the outage?"
Dev: "I don't know, we changed it 20 times yesterday."
PM: "Can we roll back?"
Dev: "To which version?"
```

**Causal Database:**
```rust
// Query the exact state at incident time
let config = db.get_at("config", "api", "2026-02-07T14:30:00Z").await?;

// See what changed and why
let history = db.history("config", "api").await?;
// [{"value": {...}, "timestamp": "...", "caused_by": "deploy-123"}]
```

### Scenario 2: The AI Agent

**Traditional Database:**
```
User: "What did I ask you to remember last Tuesday?"
AI: "I don't have a record of that conversation."
```

**Causal Database:**
```rust
// Time-travel to any conversation
let memories = db.query_at(
    "agent_memory",
    timestamp,
    Filter::new().field("agent_id").eq("assistant-42")
).await?;

// Understand the agent's reasoning
let decision_chain = db.causal_chain("decision", "choice-123").await?;
// Shows all data that influenced the decision
```

### Scenario 3: The Audit Request

**Traditional Database:**
```
Auditor: "Show me all changes to user data in Q4."
Dev: "We'll need to parse 500GB of logs..."
Auditor: "Who authorized each change?"
Dev: "That information isn't in the logs."
```

**Causal Database:**
```rust
// Query all changes with full provenance
let changes = db.query(
    namespace: "users",
    filter: Filter::new()
        .timestamp().between("2026-10-01", "2026-12-31")
        .field("operation").eq("update"),
    include_history: true
).await?;

// Each result includes:
// - What changed
// - When it changed
// - Who made the change
// - What caused the change
// - Previous values
```

---

## Getting Started

### Installation

```bash
# Rust
cargo add koru-delta

# Python
pip install koru-delta

# With LLM framework support
pip install koru-delta[frameworks]
```

### Quick Start (Rust)

```rust
use koru_delta::KoruDelta;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database (zero configuration)
    let db = KoruDelta::start().await?;
    
    // Store data (automatically versioned)
    db.put("users", "alice", json!({
        "name": "Alice",
        "email": "alice@example.com"
    })).await?;
    
    // Retrieve current value
    let user = db.get("users", "alice").await?;
    println!("Current: {:?}", user);
    
    // Update (creates new version)
    db.put("users", "alice", json!({
        "name": "Alice",
        "email": "alice.smith@example.com"
    })).await?;
    
    // Query history
    let history = db.history("users", "alice").await?;
    for entry in history {
        println!("{}: {:?}", entry.timestamp, entry.value);
    }
    
    // Time-travel query
    let old_user = db.get_at(
        "users", "alice", "2026-02-01T00:00:00Z"
    ).await?;
    println!("User on Feb 1: {:?}", old_user);
    
    Ok(())
}
```

### Quick Start (Python)

```python
import asyncio
from koru_delta import Database

async def main():
    # Create database
    async with Database() as db:
        # Store data
        await db.put("users", "alice", {
            "name": "Alice",
            "email": "alice@example.com"
        })
        
        # Retrieve
        user = await db.get("users", "alice")
        print(f"User: {user}")
        
        # Update
        await db.put("users", "alice", {
            "name": "Alice",
            "email": "alice.smith@example.com"
        })
        
        # Get history
        history = await db.history("users", "alice")
        for entry in history:
            print(f"{entry['timestamp']}: {entry['value']}")
        
        # Time-travel query
        old_user = await db.get_at(
            "users", "alice", "2026-02-01T00:00:00Z"
        )
        print(f"User on Feb 1: {old_user}")

asyncio.run(main())
```

### Vector Search

```python
from koru_delta import Database

async with Database() as db:
    # Store embedding
    await db.embed(
        "documents", "doc1",
        embedding=[0.1, 0.2, 0.3, ...],  # 1536-dim vector
        model="text-embedding-3-small",
        metadata={"title": "Introduction to Causality"}
    )
    
    # Semantic search
    results = await db.similar(
        "documents",
        query=[0.1, 0.15, 0.25, ...],
        top_k=5
    )
    
    for result in results:
        print(f"{result['key']}: {result['score']:.3f}")
```

---

## Use Cases

### AI Agents

AI agents need memory that:
- Persists across sessions
- Supports semantic search
- Maintains provenance (why did I remember this?)
- Forgets naturally (lifecycle management)

```python
from koru_delta import Database
from koru_delta.integrations import chunk_document

async with Database() as db:
    # Store episodic memory
    await db.put("episodes", "conv-123", {
        "content": "User asked about Python async",
        "embedding": embedding,
        "importance": 0.8
    })
    
    # Semantic recall
    similar = await db.similar("episodes", query_embedding, top_k=5)
    
    # Natural forgetting (automatic tiering)
    # Hot → Warm → Cold → Deep based on access patterns
```

### Audit & Compliance

Regulatory requirements often demand:
- Immutable records
- Complete provenance
- Point-in-time queries
- Tamper-evident history

```rust
// Every change is automatically logged
let tx = db.transaction()
    .with_principal("user-alice")
    .with_reason("customer-request")
    .build();

db.put_with_context("records", "cust-123", data, tx).await?;

// Immutable audit trail
let audit_log = db.history("records", "cust-123").await?;
// Cannot be modified, only appended
```

### Edge Computing

Edge deployments need:
- Small binary size (~8MB)
- Zero configuration
- Offline operation
- Automatic sync

```rust
// Runs anywhere, no setup required
let db = KoruDelta::start().await?;  // That's it

// Offline-capable
// Changes queue locally, sync when connected
```

### Configuration Management

Configuration needs:
- Versioning
- Rollback capability
- Environment promotion
- Change tracking

```rust
// Versioned config
db.put("config", "api", new_config).await?;

// Instant rollback
let previous = db.get_at("config", "api", before_incident).await?;
db.put("config", "api", previous).await?;  // New version, old value

// Compare environments
let prod = db.get("config", "api").await?;
let staging = db.get_at("config", "api", "2026-02-07T12:00:00Z").await?;
let diff = compute_diff(&prod, &staging);
```

---

## Architecture Deep Dive

### Content-Addressed Storage

```
┌─────────────────────────────────────────┐
│  Value: {"name": "Alice", "age": 30}   │
│  Hash:  blake3(value) → "a3f2..."      │
│  Identity: "a3f2..." (not location!)   │
└─────────────────────────────────────────┘
```

Benefits:
- Deduplication: Same content = same identity
- Integrity: Hash verifies content
- Immutability: Changing content changes identity

### Causal Graph

```
Version 1          Version 2          Version 3
┌─────────┐       ┌─────────┐       ┌─────────┐
│ Value A │──────→│ Value B │──────→│ Value C │
│ TS: t1  │       │ TS: t2  │       │ TS: t3  │
│ Prev: ∅ │       │ Prev: 1 │       │ Prev: 2 │
└─────────┘       └─────────┘       └─────────┘
```

Properties:
- Immutable nodes
- Directed edges (previous → next)
- Can branch (divergent histories)
- Can merge (convergent histories)

### Memory Tiering

```
┌─────────────────────────────────────────────────────┐
│ HOT (RAM)                                           │
│ • LRU cache of recent/frequent items               │
│ • ~10K items, ~400ns access                        │
│ • Prefrontal cortex equivalent                     │
├─────────────────────────────────────────────────────┤
│ WARM (Disk - Chronicle)                            │
│ • Compressed embeddings + metadata                 │
│ • ~1M items, ~1ms access                           │
│ • Hippocampus equivalent                           │
├─────────────────────────────────────────────────────┤
│ COLD (Disk - Consolidated)                         │
│ • Summaries, reduced dimensionality                │
│ • Unlimited, ~10ms access                          │
│ • Cerebral cortex equivalent                       │
├─────────────────────────────────────────────────────┤
│ DEEP (Archive - Genomic)                           │
│ • Epoch-level abstractions                         │
│ • Unlimited, ~100ms+ access                        │
│ • DNA equivalent                                   │
└─────────────────────────────────────────────────────┘
```

Transitions are automatic based on:
- Access frequency
- Access recency
- Time of day patterns
- Learned importance model

---

## Comparison

### vs Traditional Databases

| Feature | PostgreSQL | MongoDB | KoruDelta |
|---------|-----------|---------|-----------|
| Schema | Rigid | Flexible | Flexible |
| History | Manual (auditing) | Manual (oplog) | Built-in |
| Time Travel | No | No | Yes |
| Provenance | No | No | Yes |
| Deduplication | No | No | Automatic |
| Vector Search | Extension | Extension | Native |
| Lifecycle | Manual | Manual | Automatic |

### vs Vector Databases

| Feature | Pinecone | Chroma | KoruDelta |
|---------|----------|--------|-----------|
| Vector Search | Yes | Yes | Yes |
| Metadata Filtering | Yes | Yes | Yes |
| Causal History | No | No | Yes |
| Time-Travel Queries | No | No | Yes |
| Content-Addressing | No | No | Yes |
| Lifecycle Management | No | No | Yes |
| Edge Deployment | No | Limited | Yes (~8MB) |

### vs Event Sourcing

| Feature | Event Sourcing | KoruDelta |
|---------|---------------|-----------|
| History | Yes | Yes |
| Replay | Yes | Yes |
| Snapshots | Manual | Automatic |
| Vector Search | No | Yes |
| Automatic Tiering | No | Yes |
| Content-Addressing | No | Yes |

---

## Best Practices

### 1. Namespace Design

Namespaces are isolation boundaries. Design them around access patterns:

```rust
// Good: Separate by domain
users/          // User profiles
sessions/       // Session state
episodes/       // Agent memories
config/         // Configuration
audit/          // Audit trail

// Good: Separate by lifecycle
hot/            // Active working set
warm/           // Recent history
metrics/        // Time-series data (different tiering)
```

### 2. Key Design

Keys should be:
- Unique within namespace
- Meaningful (for debugging)
- Hierarchical when appropriate

```rust
// Good
users:alice
users:bob
sessions:2026-02-07:abc123
orders:2026:Q1:ord-456

// Avoid
key-1
key-2
data
```

### 3. Versioning Strategy

Every `put` creates a new version. Plan for this:

```rust
// For frequent updates, consider batching
let batch = db.batch();
for item in items {
    batch.put("queue", &item.id, item);
}
batch.commit().await?;

// Or use incremental updates
let mut user = db.get("users", "alice").await?;
user["last_seen"] = now;
db.put("users", "alice", user).await?;  // New version
```

### 4. Query Patterns

Use the right query for the job:

```rust
// Current state
db.get("users", "alice").await?;

// Specific point in time
db.get_at("users", "alice", timestamp).await?;

// Full history
db.history("users", "alice").await?;

// Semantic similarity
db.similar("documents", query_embedding, top_k).await?;

// Filtered query
db.query("users")
    .filter(Filter::new().field("status").eq("active"))
    .execute().await?;
```

### 5. Lifecycle Awareness

Design for automatic tiering:

```rust
// Hot data: Small, frequently accessed
// → Stays in Hot tier automatically

// Warm data: Accessed occasionally
// → Moves to Warm after inactivity

// Cold data: Rarely accessed
// → Moves to Cold, compressed

// Deep data: Almost never accessed
// → Moves to Deep, highly compressed
```

---

## The Philosophy

KoruDelta is built on a few core principles:

1. **Time is not an afterthought.** Every piece of data exists in time, and that context matters.

2. **Identity comes from content.** Not from location, not from name, but from what the data *is*.

3. **Relationships matter.** Data is connected, and those connections should be preserved.

4. **Forgetting is natural.** Not all data deserves equal attention forever.

5. **Simplicity enables power.** Zero configuration doesn't mean zero capability.

---

## Next Steps

- **Quick Start**: Try the [examples](../bindings/python/examples/)
- **API Reference**: See the [API documentation](./API_REFERENCE.md)
- **Architecture**: Read [ARCHITECTURE.md](../ARCHITECTURE.md)
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md)

---

*Built on [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core) — distinction calculus for the real world.*
