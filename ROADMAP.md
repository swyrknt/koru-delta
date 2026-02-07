# KoruDelta Roadmap

**Version:** 2.0.0 (Current)  
**Last Updated:** 2026-02-06  
**Status:** Production Ready → Scaling Up

---

## Overview

KoruDelta v2.0.0 is a **production-ready, lightweight causal database** optimized for:
- ✅ Zero-configuration deployments
- ✅ Versioned data with full audit trails
- ✅ Edge computing and IoT
- ✅ Local-first applications

This roadmap addresses the current limitations and charts a path to v3.0.0.

---

## Current Limitations (v2.0.0)

| # | Limitation | Impact | Priority |
|---|------------|--------|----------|
| 1 | **Write throughput** ~20K/sec via API | Not suitable for high-frequency analytics | High |
| 2 | **Query language** Basic filters only | No JOINs, limited aggregations | Medium |
| 3 | **Multi-region** Single-region clustering | No geo-distribution, split-brain risk | High |
| 4 | **Client libraries** Rust only | Python/JS/Go developers can't use it easily | Medium |
| 5 | **Hosted service** Self-hosted only | Teams need managed option | Low |

---

## Phase 1: Performance & Scale (v2.1.0)

**Goal:** 10x write throughput, bulk operations

### 1.1 Batch Operations API
```rust
// Bulk insert for high-throughput scenarios
db.batch_put([
    ("metrics/cpu", json!({"val": 45, "ts": 123456})),
    ("metrics/mem", json!({"val": 82, "ts": 123456})),
    // ... 1000s of items
]).await?;
```

**Use Case:** Time-series data, metrics ingestion, IoT sensor batches

**Target:** 200K+ writes/sec for batches (10x improvement)

### 1.2 Connection Pooling
- Keep TCP connections open between client and server
- Currently: new connection per request (HTTP overhead)
- After: persistent connections with multiplexing

**Target:** Reduce per-write latency from 50ms → 5ms (CLI)

### 1.3 Async WAL with fsync Batching
- Current: fsync on every write (durable but slow)
- v2.1: Async WAL with configurable fsync interval
  - `fsync=always` - Current behavior (safest)
  - `fsync=1s` - fsync every second (fast + safe)
  - `fsync=off` - OS decides (fastest, some risk)

**Use Case:** Metrics where losing 1s of data is acceptable

### 1.4 Memory-Mapped Hot Storage
- Current: HashMap + serialization
- v2.1: Memory-mapped files for hot tier
- Zero-copy reads for large values

**Target:** 2-3x read performance improvement

**Deliverables:**
- [ ] `batch_put()` / `batch_get()` API
- [ ] Connection pooling in client
- [ ] Configurable fsync modes
- [ ] Memory-mapped hot storage
- [ ] Benchmark: 200K+ writes/sec (batches)

**Timeline:** 6-8 weeks  
**Breaking Changes:** None (additive)

---

## Phase 2: Query Engine v2 (v2.2.0)

**Goal:** SQL-like query capabilities without SQL complexity

### 2.1 JOIN Support (Limited)
```rust
// Cross-namespace lookups
let query = Query::new()
    .join("orders", "user_id", "users", "id")
    .filter(Filter::eq("users:status", "premium"))
    .filter(Filter::gt("orders:amount", 100));
```

**Limitation:** Only equi-JOINs on keys, no arbitrary cross-products

**Use Case:** "Find all orders from premium users"

### 2.2 Aggregation Pipeline
```rust
let results = db.query("sales", Query::new()
    .group_by("region")
    .aggregate(Aggregation::pipeline()
        .sum("amount")
        .avg("discount")
        .count()
        .min("timestamp")
        .max("timestamp")
    )
    .sort_by("sum_amount", false)
).await?;
```

**Use Case:** Sales reports, analytics dashboards

### 2.3 Secondary Indexes
- Current: Primary key only (namespace/key)
- v2.2: Optional secondary indexes on fields
- Trade-off: Faster queries, slower writes

```rust
// Create index
db.create_index("users", "email", IndexType::Unique).await?;
db.create_index("orders", "status", IndexType::NonUnique).await?;

// Query uses index automatically
let user = db.get_by_index("users", "email", "alice@example.com").await?;
```

### 2.4 Full-Text Search
```rust
// Index text fields
let query = Query::new()
    .text_search("content", "database +performance -slow")
    .sort_by("_relevance", false);
```

**Implementation:** Integrate with tantivy or similar

**Deliverables:**
- [ ] Cross-namespace JOIN (limited)
- [ ] Aggregation pipeline
- [ ] Secondary index API
- [ ] Full-text search integration
- [ ] Query optimizer (choose index vs scan)

**Timeline:** 10-12 weeks  
**Breaking Changes:** None (additive)

---

## Phase 3: Multi-Region & Enterprise (v2.3.0)

**Goal:** Production-grade distributed database

### 3.1 CRDT-Based Conflict Resolution
- Current: Last-write-wins (simpler but loses data)
- v2.3: CRDTs for automatic conflict merging

```rust
// Counters (auto-merge additions)
db.put("stats/views", json!({"count": Counter(1)})).await?;
// Node A: +5, Node B: +3 → Result: 9 (not 3 or 5)

// Sets (auto-merge unions)
db.put("tags/item", json!({"tags": Set(["a", "b"])})).await?;
// Node A: add "c", Node B: add "d" → Result: ["a", "b", "c", "d"]
```

**Use Case:** Shopping carts, counters, sets in multi-region

### 3.2 Consensus for Cluster Leadership
- Current: Gossip (eventually consistent)
- v2.3: Optional Raft consensus for strong consistency

```rust
// For data that needs strong consistency
let config = ClusterConfig::new()
    .consistency(Consistency::Raft)  // vs Gossip
    .replication_factor(3);
```

**Use Case:** Financial transactions, inventory, leader election

### 3.3 Geo-Distribution
```rust
// Pin data to regions
let config = ClusterConfig::new()
    .region("us-west")
    .replicate_to(["us-east", "eu-west"]);

// Read from nearest
db.get("users/alice").await?;  // Auto-routes to us-west
```

**Use Case:** Global apps with data residency requirements

### 3.4 Backup & Point-in-Time Recovery
```rust
// Snapshot at specific time
let snapshot = db.snapshot_at("2026-02-06T12:00:00Z").await?;

// Incremental backup
let delta = db.changes_since(last_backup_time).await?;
```

**Deliverables:**
- [ ] CRDT data types (Counter, Set, Map, Register)
- [ ] Raft consensus option
- [ ] Multi-region awareness
- [ ] Point-in-time recovery
- [ ] Cross-region replication

**Timeline:** 16-20 weeks  
**Breaking Changes:** None (additive, opt-in)

---

## Phase 4: Language Bindings (v2.4.0)

**Goal:** Python, JavaScript, Go support

### 4.1 Python Client
```python
from korudelta import KoruDelta
import asyncio

async def main():
    db = await KoruDelta.start_with_path("./data")
    await db.put("users", "alice", {"name": "Alice", "age": 30})
    user = await db.get("users", "alice")
    print(user.value())

asyncio.run(main())
```

**Implementation:** PyO3 bindings

### 4.2 JavaScript/TypeScript Client
```typescript
import { KoruDelta } from 'koru-delta';

const db = await KoruDelta.startWithPath('./data');
await db.put('users', 'alice', { name: 'Alice', age: 30 });
const user = await db.get('users', 'alice');
console.log(user.value());
```

**Implementation:** wasm-bindgen for Node.js, WASM for browser

### 4.3 Go Client
```go
import "github.com/koru-delta/go-client"

db, _ := koru.Connect("http://localhost:7878")
db.Put("users", "alice", map[string]interface{}{
    "name": "Alice",
    "age": 30,
})
```

**Implementation:** C FFI bindings

**Deliverables:**
- [ ] Python bindings (PyO3)
- [ ] JavaScript/TypeScript bindings
- [ ] Go bindings
- [ ] Language-specific idioms
- [ ] Examples and documentation

**Timeline:** 12-16 weeks (can parallelize)  
**Breaking Changes:** None

---

## Phase 5: Hosted Service (v3.0.0)

**Goal:** Managed KoruDelta for teams

### 5.1 KoruDelta Cloud
- Fully managed clusters
- Auto-scaling
- Backup/restore
- Monitoring dashboard
- Multi-tenant isolation

### 5.2 Pricing Tiers
| Tier | Storage | Throughput | Use Case |
|------|---------|------------|----------|
| Free | 1GB | 100 ops/sec | Side projects |
| Pro | 100GB | 10K ops/sec | Production apps |
| Enterprise | Unlimited | Custom | Large scale |

### 5.3 Enterprise Features
- SSO/SAML integration
- Audit logging
- VPC peering
- Dedicated support
- SLA guarantees

**Deliverables:**
- [ ] Managed cloud service
- [ ] Web dashboard
- [ ] Terraform provider
- [ ] Kubernetes operator
- [ ] Enterprise features

**Timeline:** 20-24 weeks  
**Business Model:** Freemium SaaS

---

## Technical Debt & Maintenance

### Continuous (Ongoing)
- [ ] Security audits (quarterly)
- [ ] Dependency updates (monthly)
- [ ] Performance regression tests (per PR)
- [ ] Documentation updates (per release)

### v2.x Maintenance
- [ ] Windows CI/CD
- [ ] ARM64 builds (Apple Silicon, Graviton)
- [ ] WASM browser compatibility
- [ ] Fuzz testing

---

## Prioritization Framework

**How we decide what to build:**

1. **User Pain** - Are users blocked by this limitation?
2. **Use Case Fit** - Does it serve our target use cases?
3. **Complexity** - Can we maintain it long-term?
4. **Differentiation** - Does it reinforce our unique value?

**Not Building:**
- ❌ Full SQL support (becomes PostgreSQL)
- ❌ Graph queries (use Neo4j)
- ❌ Columnar storage (use ClickHouse)
- ❌ Stored procedures (keeps it simple)

**Building:**
- ✅ Better batch operations (time-series use case)
- ✅ Limited JOINs (common need, fits model)
- ✅ CRDTs (causal consistency is our differentiator)
- ✅ Language bindings (developer adoption)

---

## Success Metrics

| Metric | v2.0.0 | v2.3.0 Target | v3.0.0 Target |
|--------|--------|---------------|---------------|
| Write throughput | 20K/sec | 200K/sec | 1M/sec (clustered) |
| Query complexity | Filters | JOINs + Aggregations | SQL-like |
| Regions | 1 | 3+ | Global |
| Languages | Rust | Rust + Python + JS | All major |
| GitHub stars | 100 | 1,000 | 10,000 |
| Production users | 10 | 100 | 1,000 |

---

## Contributing to the Roadmap

**We prioritize based on:**
1. GitHub issues with 👍 reactions
2. Real-world usage feedback
3. Sponsor requests
4. Core team vision

**How to influence:**
- Open an issue explaining your use case
- Vote on existing issues
- Sponsor the project
- Contribute PRs

---

## Timeline Summary

| Version | Focus | ETA | Key Features |
|---------|-------|-----|--------------|
| 2.0.0 | ✅ Production | Now | Zero config, versioning, clustering |
| 2.1.0 | Performance | +2mo | Batches, 200K writes/sec |
| 2.2.0 | Query Engine | +5mo | JOINs, aggregations, indexes |
| 2.3.0 | Distribution | +9mo | CRDTs, multi-region, consensus |
| 2.4.0 | Languages | +12mo | Python, JS, Go bindings |
| 3.0.0 | Cloud | +18mo | Managed service, enterprise |

---

*This roadmap is a living document. Priorities shift based on user feedback and real-world usage.*

**Current Status:** 2.0.0 is production-ready. 2.1.0 development starts now.
