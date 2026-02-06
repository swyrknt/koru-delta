# Phase 7: Unified Core Integration

**Status:** Planning Complete  
**Date:** 2026-02-04  
**Previous:** Phases 1-6 Complete (221 tests passing)  
**Goal:** Wire all layers into cohesive unified system

---

## Overview

Phase 7 integrates all six previous phases into a unified `KoruDeltaCore` that:
- Automatically manages memory across Hot/Warm/Cold/Deep tiers
- Runs evolutionary processes in background
- Syncs with peers via reconciliation
- Protects access with capability-based auth
- Maintains backward compatibility with v1 API

---

## Architecture

### Unified Core Structure

```rust
pub struct KoruDeltaCore {
    // Configuration
    config: CoreConfig,
    
    // Layer 2: Storage (foundation)
    storage: Arc<CausalStorage>,
    
    // Layer 3: Memory Tiering
    hot: Arc<HotMemory>,
    warm: Arc<WarmMemory>,
    cold: Arc<ColdMemory>,
    deep: Arc<DeepMemory>,
    
    // Layer 4: Processes
    process_runner: Arc<ProcessRunner>,
    
    // Layer 5: Reconciliation
    reconciliation: Arc<ReconciliationManager>,
    
    // Layer 6: Auth
    auth: Arc<AuthManager>,
    
    // Background tasks
    shutdown: tokio::sync::watch::Sender<bool>,
}
```

### Data Flow

```
User API Call
     ↓
KoruDeltaCore
     ↓
┌─────────────────────────────────────────┐
│  Auth Check (Capability verification)   │ ← Layer 6
└─────────────────────────────────────────┘
     ↓
┌─────────────────────────────────────────┐
│  Hot Memory (try first)                 │ ← Layer 3
└─────────────────────────────────────────┘
     ↓ (miss)
┌─────────────────────────────────────────┐
│  Warm/Cold/Deep (promote if found)      │ ← Layer 3
└─────────────────────────────────────────┘
     ↓
┌─────────────────────────────────────────┐
│  CausalStorage (source of truth)        │ ← Layer 2
│  - CausalGraph                          │ ← Layer 1
│  - ReferenceGraph                       │ ← Layer 1
└─────────────────────────────────────────┘
     ↓
┌─────────────────────────────────────────┐
│  Background:                            │
│  - ConsolidationProcess (move tiers)    │ ← Layer 4
│  - DistillationProcess (fitness)        │ ← Layer 4
│  - GenomeUpdateProcess (backup)         │ ← Layer 4
│  - Reconciliation (sync peers)          │ ← Layer 5
└─────────────────────────────────────────┘
```

---

## Implementation Plan

### Week 1: Core Structure & Memory Integration

#### Day 1-2: Create `KoruDeltaCore` Shell
```rust
// src/core_v2.rs
pub struct KoruDeltaCore { ... }

impl KoruDeltaCore {
    pub async fn new(config: CoreConfig) -> Result<Self>;
    pub async fn shutdown(self) -> Result<()>;
}
```

**Tasks:**
- [ ] Create `src/core_v2.rs` module
- [ ] Define `CoreConfig` struct
- [ ] Implement constructor with all layers
- [ ] Implement graceful shutdown
- [ ] Add basic health check method

#### Day 3-4: Unified `put()` Implementation
```rust
pub async fn put(
    &self,
    namespace: &str,
    key: &str,
    value: JsonValue,
) -> Result<VersionedValue> {
    // 1. Check auth (if enabled)
    // 2. Store in CausalStorage
    // 3. Add to Hot memory
    // 4. Notify reconciliation (for sync)
    // 5. Return versioned value
}
```

**Tasks:**
- [ ] Implement auth-gated put
- [ ] Wire storage → hot memory
- [ ] Add reconciliation notification
- [ ] Write tests (10 tests)

#### Day 5: Unified `get()` Implementation
```rust
pub async fn get(
    &self,
    namespace: &str,
    key: &str,
) -> Result<VersionedValue> {
    // 1. Check Hot memory (fast path)
    // 2. Check Warm (promote if found)
    // 3. Check Cold (promote if found)
    // 4. Check Deep (express if needed)
    // 5. Fallback to storage
}
```

**Tasks:**
- [ ] Implement tiered get with promotion
- [ ] Add LRU tracking
- [ ] Write tests (10 tests)

#### Day 6-7: Query & History
```rust
pub async fn query(&self, namespace: &str, query: Query) -> Result<Vec<...>>;
pub async fn history(&self, namespace: &str, key: &str) -> Result<Vec<...>>;
pub async fn get_at(&self, namespace: &str, key: &str, timestamp: DateTime) -> Result<...>;
```

**Tasks:**
- [ ] Implement query across all tiers
- [ ] Implement history (use causal graph)
- [ ] Implement time travel
- [ ] Write tests (15 tests)

**Week 1 Deliverables:**
- Core structure with all layers wired
- Working put/get with memory tiering
- 35 new tests

---

### Week 2: Background Processes

#### Day 1-2: Process Runner Integration
```rust
impl KoruDeltaCore {
    async fn start_background_tasks(&self) {
        // Spawn consolidation process
        // Spawn distillation process
        // Spawn genome update process
        // Spawn reconciliation listener
    }
}
```

**Tasks:**
- [ ] Start ProcessRunner on init
- [ ] Wire consolidation to storage
- [ ] Add config for process intervals
- [ ] Write tests (5 tests)

#### Day 3-4: Consolidation Integration
```rust
// In consolidation process:
1. Find demotion candidates in Hot
2. Move to Warm
3. Find idle in Warm
4. Consolidate to Cold
5. Rotate Cold epochs
6. Archive old epochs to Deep
```

**Tasks:**
- [ ] Wire hot → warm eviction
- [ ] Wire warm → cold consolidation
- [ ] Wire cold → deep archiving
- [ ] Write tests (8 tests)

#### Day 5-6: Distillation Integration
```rust
// In distillation process:
1. Calculate fitness for cold distinctions
2. Classify as fit/unfit
3. Keep fit in cold
4. Archive unfit to deep
```

**Tasks:**
- [ ] Integrate fitness calculation
- [ ] Wire natural selection
- [ ] Test with simulated data
- [ ] Write tests (8 tests)

#### Day 7: Genome Integration
```rust
// In genome update process:
1. Extract genome from causal graph
2. Store in deep memory
3. Schedule periodic updates
```

**Tasks:**
- [ ] Wire genome extraction
- [ ] Add genome persistence
- [ ] Test recovery from genome
- [ ] Write tests (5 tests)

**Week 2 Deliverables:**
- All 4 processes running in background
- Automatic memory management working
- 26 new tests

---

### Week 3: Reconciliation & Sync

#### Day 1-2: Reconciliation Manager Integration
```rust
impl KoruDeltaCore {
    pub async fn add_peer(&self, peer_addr: SocketAddr) -> Result<()>;
    pub async fn remove_peer(&self, peer_addr: SocketAddr) -> Result<()>;
    pub async fn sync_with(&self, peer: SocketAddr) -> Result<SyncReport>;
}
```

**Tasks:**
- [ ] Wire ReconciliationManager
- [ ] Implement peer management
- [ ] Add sync triggers on write
- [ ] Write tests (8 tests)

#### Day 3-4: Automatic Sync
```rust
// Background task:
1. Listen for storage changes
2. Notify reconciliation
3. Sync with peers
4. Handle conflicts
```

**Tasks:**
- [ ] Add change notification channel
- [ ] Implement eager sync
- [ ] Handle network failures
- [ ] Write tests (10 tests)

#### Day 5-6: Multi-Node Testing
```rust
#[tokio::test]
async fn test_three_node_sync() {
    // Start 3 nodes
    // Write to node 1
    // Verify sync to nodes 2 & 3
    // Handle conflicts
}
```

**Tasks:**
- [ ] Create multi-node test harness
- [ ] Test eventual consistency
- [ ] Test conflict resolution
- [ ] Write tests (5 tests)

#### Day 7: Sync Performance
```rust
// Benchmarks:
- Sync 1K distinctions between nodes
- Measure bandwidth usage
- Verify O(log n) behavior
```

**Tasks:**
- [ ] Add sync benchmarks
- [ ] Optimize hot paths
- [ ] Document performance

**Week 3 Deliverables:**
- Automatic sync working
- Multi-node tests passing
- 23 new tests

---

### Week 4: Auth Integration

#### Day 1-2: HTTP Server with Auth
```rust
// Mount auth routes
let app = Router::new()
    .merge(auth_routes(auth.clone()))
    .merge(protected_routes(auth.clone()))
    .merge(data_routes(core.clone()))
    .layer(auth_middleware(auth));
```

**Tasks:**
- [ ] Create unified HTTP router
- [ ] Mount auth endpoints
- [ ] Add auth middleware
- [ ] Write tests (8 tests)

#### Day 3-4: Protected Data Endpoints
```rust
// Protect existing endpoints:
GET /api/v1/:namespace/:key      ← requires Read capability
PUT /api/v1/:namespace/:key      ← requires Write capability
POST /api/v1/:namespace/query    ← requires Read capability
```

**Tasks:**
- [ ] Add capability checks to endpoints
- [ ] Return 403 for unauthorized
- [ ] Test with/without capabilities
- [ ] Write tests (10 tests)

#### Day 5: Default Capabilities
```rust
// On identity creation, grant:
- Read on own identity
- Write on own data namespace
```

**Tasks:**
- [ ] Implement default grants
- [ ] Test self-access
- [ ] Document capability model
- [ ] Write tests (5 tests)

#### Day 6-7: End-to-End Auth Flow
```rust
#[tokio::test]
async fn test_full_auth_flow() {
    // 1. Register identity
    // 2. Authenticate
    // 3. Access protected resource
    // 4. Grant capability to another
    // 5. Verify access control
}
```

**Tasks:**
- [ ] Create E2E auth test
    - [ ] Test all auth scenarios
    - [ ] Document auth flow
    - [ ] Write tests (7 tests)

**Week 4 Deliverables:**
- HTTP API with auth protection
- Default capability grants
- 30 new tests

---

### Week 5: Migration & Polish

#### Day 1-2: v1 Compatibility Layer
```rust
// src/core.rs (existing)
impl KoruDelta {
    // Keep existing API
    // Delegate to KoruDeltaCore internally
}
```

**Tasks:**
- [ ] Create compatibility shim
- [ ] Test v1 API still works
- [ ] Add deprecation notices
- [ ] Write tests (10 tests)

#### Day 3-4: Configuration
```rust
pub struct CoreConfig {
    // Memory
    pub hot_capacity: usize,
    pub warm_capacity: usize,
    pub cold_epochs: usize,
    
    // Processes
    pub consolidation_interval: Duration,
    pub distillation_interval: Duration,
    
    // Sync
    pub sync_enabled: bool,
    pub sync_interval: Duration,
    
    // Auth
    pub auth_enabled: bool,
    pub session_ttl: Duration,
}
```

**Tasks:**
- [ ] Add comprehensive config
- [ ] Validate config on startup
- [ ] Document all options
- [ ] Write tests (5 tests)

#### Day 5: Performance Benchmarks
```rust
#[bench]
fn bench_put_with_tiering(b: &mut Bencher) {
    // Benchmark put with memory tiering
}

#[bench]
fn bench_get_with_promotion(b: &mut Bencher) {
    // Benchmark get with tier promotion
}
```

**Tasks:**
- [ ] Add put benchmark
- [ ] Add get benchmark
- [ ] Add sync benchmark
- [ ] Document results

#### Day 6-7: Documentation
- [ ] Update ARCHITECTURE.md with unified core
- [ ] Write migration guide (v1 → v2)
- [ ] Document new configuration options
- [ ] Create "Getting Started" for v2

**Week 5 Deliverables:**
- v1 compatibility maintained
- Configuration system
- Benchmarks
- Complete documentation
- 15 new tests

---

## Success Criteria

### Functional
- [ ] All 6 layers wired together
- [ ] Automatic memory tiering works
- [ ] Background processes run continuously
- [ ] Automatic sync between nodes
- [ ] Auth protects HTTP endpoints
- [ ] v1 API still works

### Performance
- [ ] Put < 5ms (with tiering)
- [ ] Get < 2ms (hot hit)
- [ ] Get < 10ms (warm promotion)
- [ ] Sync 1K items < 1s

### Quality
- [ ] 120+ new tests (total > 340)
- [ ] 0 compiler warnings
- [ ] 0 clippy warnings
- [ ] 100% doc coverage for public API

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking v1 API | Low | High | Compatibility shim |
| Performance regression | Medium | High | Benchmarks + optimization |
| Complex integration bugs | Medium | High | Comprehensive tests |
| Memory leaks in background tasks | Low | High | Proper shutdown + tests |

---

## Open Questions

1. **Should v2 be opt-in or default?**
   - Proposal: Opt-in via `KoruDelta::start_v2()`

2. **How to handle v1 → v2 migration?**
   - Proposal: Automatic on first v2 start

3. **Should auth be required or optional?**
   - Proposal: Optional, enabled via config

4. **How aggressive should sync be?**
   - Proposal: Configurable (eager vs lazy)

---

## Timeline Summary

| Week | Focus | Tests | Deliverable |
|------|-------|-------|-------------|
| 1 | Core + Memory | 35 | Working put/get with tiering |
| 2 | Processes | 26 | Background tasks running |
| 3 | Sync | 23 | Multi-node reconciliation |
| 4 | Auth | 30 | Protected HTTP API |
| 5 | Polish | 15 | v1 compat + docs |
| **Total** | | **129** | **Unified Core** |

---

## Next Steps

1. ✅ Architecture review complete
2. ✅ Phase 7 plan approved
3. 🎯 Begin Week 1 implementation
4. 📊 Track progress daily
5. 🧪 Test continuously

---

*Phase 7 Planning Complete*  
*Ready to begin implementation*
