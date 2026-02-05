# KoruDelta Distinction Extension Implementation

> **Status:** Refactoring to capture emergent behavior
> **Goal:** Clean integration of causal/reference tracking
> **Approach:** Evolve existing code, remove unused patterns
> **Timeline:** 8 weeks to complete

## The Clean Integration Approach

### What We're Doing

**NOT:** Building v2 alongside v1  
**NOT:** Keeping deprecated code  
**YES:** Evolving the existing codebase  
**YES:** Removing patterns that don't serve the distinction model  
**YES:** Clean, unified architecture

### Code Changes

**Remove:**
- Unused abstraction layers
- Redundant storage patterns  
- Ad-hoc compaction (replace with distillation)
- Manual retention policies (replace with natural selection)

**Evolve:**
- `CausalStorage` → integrates causal graph
- `put()` → becomes synthesis capture
- `sync()` → becomes set reconciliation
- `auth()` → becomes capability traversal

**Keep (Respected):**
- `koru_lambda_core::DistinctionEngine` (unchanged)
- Public API surface (`put`, `get`, `query`)
- All existing tests (must pass)

### What Users Will Feel

**Before (Current):**
- "My database file is 10GB and growing"
- "Sync is slow, sends everything"
- "Why is auth so complicated?"
- "I ran out of memory"

**After (Evolved):**
- "It just... stays small?" (distillation)
- "Sync is instant" (set reconciliation)
- "Auth just works" (capability graph)
- "Runs on my Raspberry Pi" (layered memory)

### The "It Just Works" Factor

| Feature | User Experience | Technical Mechanism |
|---------|----------------|---------------------|
| **Auto-Compaction** | Database never grows unbounded | Distillation removes noise, keeps essence |
| **Fast Sync** | Near-instant reconciliation | Set reconciliation sends only missing distinctions |
| **Zero-Config Auth** | `kdelta auth init` and done | Capability graph, no JWT/secrets management |
| **Unbounded Scale** | Millions of keys, same RAM | Hot/Warm/Cold/Deep tiers |
| **Time Travel** | `get_at()` just works | Causal graph traversal |
| **Backup** | `kdelta export-genome` → 1KB file | Genome extraction |

---

## Phase 1: Foundation Modules ✅ COMPLETE

New modules (not replacements, additions):
- `causal_graph.rs` - tracks how distinctions cause each other
- `reference_graph.rs` - tracks what points to what

### Week 1: CausalGraph Core ✅
- [x] Create `src/causal_graph.rs` module
- [x] Implement `CausalGraph` struct:
  - [x] `parents: DashMap<DistinctionId, Vec<DistinctionId>>`
  - [x] `children: DashMap<DistinctionId, Vec<DistinctionId>>`
  - [x] `nodes: DashSet<DistinctionId>`
- [x] Implement `add_node()` - add distinction to graph
- [x] Implement `add_edge()` - add causal link
- [x] Implement `ancestors()` - BFS to find all ancestors
- [x] Implement `descendants()` - BFS to find all descendants
- [x] Implement `lca()` - Least Common Ancestor for merge
- [x] Implement `frontier()` - find leaf nodes (current state)
- [x] Implement `roots()` - find genesis distinctions
- [x] Write tests (9 tests, all passing)
- **User Benefit:** Time travel queries, proper causal understanding

### Week 1b: ReferenceGraph ✅
- [x] Create `src/reference_graph.rs` module
- [x] Implement `ReferenceGraph` struct:
  - [x] `outgoing: DashMap<DistinctionId, Vec<DistinctionId>>`
  - [x] `incoming: DashMap<DistinctionId, Vec<DistinctionId>>`
- [x] Implement `add_reference()` - track what points to what
- [x] Implement `reference_count()` - for GC
- [x] Implement `is_reachable()` - check if distinction is live
- [x] Implement `find_garbage()` - find unreachable distinctions
- [x] Implement `find_hot_candidates()` - for hot memory promotion
- [x] Write tests (7 tests, all passing)
- **User Benefit:** Intelligent memory management, automatic cleanup

---

## Phase 2: Clean Integration ✅ COMPLETE

### Refactor CausalStorage ✅

**Evolved pattern (implemented):**
```rust
pub struct CausalStorage {
    engine: Arc<DistinctionEngine>,           // Respect core
    causal_graph: CausalGraph,                // NEW: capture causality
    reference_graph: ReferenceGraph,          // NEW: capture references
    current_state: DashMap<...>,              // Keep
    version_store: DashMap<...>,              // NEW: content-addressed versions
    value_store: DashMap<...>,                // Keep
}
```

**Removed:** `history_log` - The causal graph + version_store IS the history.

### Completed Tasks ✅
- [x] Refactor `CausalStorage` to use causal graph for history
- [x] Add `causal_graph` field (tracks how distinctions cause each other)
- [x] Add `reference_graph` field (for GC and hot memory tracking)
- [x] Add `version_store` (content-addressed version storage)
- [x] Update `put()` to populate causal graph on each write
- [x] Update `history()` to use version_store + causal graph
- [x] Update `get_at()` to traverse causal graph for time travel
- [x] Update `from_snapshot()` to rebuild causal graph
- [x] Update tests for content-addressing behavior
- [x] All 99 lib tests passing

### Architecture Benefits
- **Unified history**: Causal graph provides history + causality
- **Deduplication**: version_store is content-addressed
- **Emergence captured**: Every put() adds to causal graph
- **Respects core**: koru-lambda-core unchanged
- **Clean code**: Removed redundant history_log pattern

### Week 2: Extended Engine
- [ ] Extend `DistinctionEngine` with:
  - [ ] `causal_graph: CausalGraph`
  - [ ] `reference_graph: ReferenceGraph`
  - [ ] `epoch: AtomicU64`
- [ ] Modify `distinguish()` to:
  - [ ] Add to causal graph
  - [ ] Track causal parents
  - [ ] Track references
- [ ] Implement `synthesize()` - create new distinction with context
- [ ] Implement `find_roots()` - distinctions with no parents
- [ ] Implement `find_frontier()` - current leaves
- [ ] Implement `capture_topology()` - for genome
- [ ] Write integration tests
- [ ] Ensure backward compatibility
- **User Benefit:** Everything is now tracked causally

---

## Phase 3: Memory Architecture Evolution 🎯

### Current Pattern (Simplify)

**Current:** All data in RAM (`current_state`, `history_log`)

**Problem:** Doesn't scale, unbounded RAM

**Evolution:** Tiered memory (not new v2, evolution of storage)

### Hot Layer (Evolve current_state)
- Keep frequently accessed in RAM
- Use reference_graph to identify "hot" distinctions
- Move cold to disk automatically

### Chronicle Layer (Evolve persistence)
- Current WAL is good
- Keep it (don't replace, enhance)
- Add index for fast causal traversal

### Remove These Patterns:
- [ ] Unbounded in-memory history
- [ ] Full database snapshots (replace with genome)
- [ ] Manual compaction triggers

### Add These Capabilities:
- [ ] Automatic hot/cold separation
- [ ] Reference-counted GC
- [ ] Causal frontier tracking

### The Logic
Users should never think about memory. The system should just work at any scale.

### Week 3a: HotMemory (Working Memory) ✅ COMPLETE
- [x] Create `src/memory/hot.rs`
- [x] Implement `HotMemory` with LRU cache:
  - [x] `cache: DashMap<DistinctionId, VersionedValue>`
  - [x] `current_state: DashMap<FullKey, DistinctionId>`
  - [x] `access_order: VecDeque<DistinctionId>` for LRU
- [x] Implement `get()` - access with LRU update
- [x] Implement `put()` - add with eviction
- [x] Implement `evict_lru()` - evict to warm
- [x] Write tests (7 tests, all passing)
- **User Benefit:** Fast access to recent data, bounded RAM

**HotMemory Features:**
- LRU (Least Recently Used) eviction policy
- Configurable capacity (default: 1000 items)
- Statistics tracking (hits, misses, hit rate)
- Handles updates (replaces old version)
- Clear operation (evict all to warm)

### Week 3b: WarmMemory (Recent Chronicle) ✅ COMPLETE
- [x] Create `src/memory/warm.rs`
- [x] Implement `WarmMemory`:
  - [x] `index: DashMap<DistinctionId, IndexEntry>` - in-memory index
  - [x] `recent_window: VecDeque` - for promotion candidates
  - [x] `current_mappings: DashMap<FullKey, DistinctionId>`
- [x] Implement `put()` - add to warm (from Hot eviction)
- [x] Implement `get()` - fetch with access tracking
- [x] Implement `find_promotion_candidates()` - for Hot promotion
- [x] Implement `find_demotion_candidates()` - for Cold demotion
- [x] Write tests (8 tests, all passing)
- **User Benefit:** Full history available, but not in RAM

**WarmMemory Features:**
- Index capacity: 10K distinctions (configurable)
- Idle threshold: 1 hour (Cold demotion candidate)
- Promotion tracking based on recent window
- Statistics: hits, misses, promotions, demotions

### Week 4a: ColdMemory (Consolidated Epochs)
- [ ] Create `src/memory/cold.rs`
- [ ] Implement `ColdMemory`:
  - [ ] `epochs: Vec<Epoch>`
  - [ ] `patterns: PatternIndex`
- [ ] Implement `consolidate()` - compress warm into cold
- [ ] Implement `extract_patterns()` - find common patterns
- [ ] Implement `compress()` - write compressed epoch
- [ ] Write tests
- **User Benefit:** Database stays small, old data compressed

### Week 4b: DeepMemory (Genomic Storage)
- [ ] Create `src/memory/deep.rs`
- [ ] Implement `DeepMemory`:
  - [ ] `genome: Genome`
  - [ ] `archive: GenomicArchive`
- [ ] Implement `extract_genome()` - minimal recreation info
- [ ] Implement `express_genome()` - recreate from genome
- [ ] Implement `archive_epoch()` - move to deep storage
- [ ] Write tests
- **User Benefit:** Portable backups, system can self-restore

---

## Phase 4: Evolutionary Processes 🎯

### The Logic
The system should manage itself. Users shouldn't think about compaction or retention.

### Week 5a: ConsolidationProcess (Sleep Cycle)
- [ ] Create `src/processes/consolidation.rs`
- [ ] Implement `ConsolidationProcess`:
  - [ ] Move hot → warm
  - [ ] Move warm → cold
  - [ ] Update indices
- [ ] Implement rhythm (timer-based, configurable)
- [ ] Write tests
- **User Benefit:** Automatic memory management

### Week 5b: DistillationProcess (Natural Selection)
- [ ] Create `src/processes/distillation.rs`
- [ ] Implement `DistillationProcess`:
  - [ ] `fitness()` - score distinctions
  - [ ] `classify()` - fit vs unfit
  - [ ] `distill()` - keep fit, archive unfit
- [ ] Implement natural selection logic
- [ ] Write tests
- **User Benefit:** Database never grows unbounded

### Week 5c: GenomeUpdateProcess (DNA Update)
- [ ] Create `src/processes/genome_update.rs`
- [ ] Implement `GenomeUpdateProcess`:
  - [ ] Extract essential structure
  - [ ] Update genome
  - [ ] Store in deep memory
- [ ] Write tests
- **User Benefit:** Always have minimal backup

---

## Phase 5: World Reconciliation 🎯

### The Logic
Sync should be instant and optimal. Send only what the other side doesn't have.

### Week 6a: Set Reconciliation
- [ ] Create `src/reconciliation/mod.rs`
- [ ] Implement Merkle tree for distinctions
- [ ] Implement Bloom filter exchange
- [ ] Implement `find_missing()` - what distinctions to sync
- [ ] Write tests
- **User Benefit:** Fast, efficient sync

### Week 6b: World Reconciliation
- [ ] Create `src/reconciliation/world.rs`
- [ ] Implement `WorldReconciliation`:
  - [ ] `exchange_roots()` - share frontier
  - [ ] `reconcile()` - full sync
  - [ ] `merge_graphs()` - combine causal graphs
- [ ] Handle conflicts as causal branches
- [ ] Write tests
- **User Benefit:** Distributed truth, automatic convergence

---

## Phase 6: Auth via Distinctions 🎯

### The Logic
Auth should be simple. No JWT, no sessions table, no complexity.

### Week 7: Distinction-Based Auth
- [ ] Create `src/auth/mod.rs`
- [ ] Implement `User` distinction type
- [ ] Implement `Credential` distinction type
- [ ] Implement `Capability` distinction type
- [ ] Implement `authorize()` - graph path verification
  ```rust
  // Auth check: Does path exist?
  Request -> Session -> User -> Capability -> Resource
  ```
- [ ] Implement `create_session()` - synthesize session
- [ ] Implement `revoke()` - synthesize revocation
- [ ] Write tests
- **User Benefit:** Zero-config auth, automatic audit trail

---

## Phase 7: Integration 🎯

### Week 8: KoruDelta v2 Integration
- [ ] Create `src/v2/mod.rs`
- [ ] Integrate all layers:
  - [ ] Hot → Warm → Cold → Deep (automatic flow)
  - [ ] Causal graph for all operations
  - [ ] Reference graph for GC
- [ ] Implement `KoruDeltaV2` struct
- [ ] Port existing API to v2
- [ ] Maintain backward compatibility (v1 mode)
- [ ] Write comprehensive tests
- **User Benefit:** Seamless upgrade, same API, better everything

---

## Phase 8: Polish & Documentation 🎯

### Week 9-10: Final Integration
- [ ] End-to-end tests
- [ ] Performance benchmarks
- [ ] Memory usage validation
- [ ] Compaction correctness tests
- [ ] Sync correctness tests
- [ ] Auth security tests

### Documentation
- [ ] Update ARCHITECTURE.md
- [ ] Document causal graph operations
- [ ] Document memory tiering
- [ ] Document auth system
- [ ] Write migration guide
- [ ] Update CLI guide
- [ ] Create "Why Evolve" explainer

---

## Progress Log

### 2026-02-05 - Foundation Complete
- ✅ CausalGraph: 9 tests passing
- ✅ ReferenceGraph: 7 tests passing
- ✅ 16 new tests total, all green
- 🎯 Next: DistinctionEngine integration

### Success Metrics

By completion:
- [ ] Database size stays bounded under load
- [ ] Sync is 10x faster (set reconciliation)
- [ ] Auth setup is 1 command
- [ ] Runs on Raspberry Pi with 512MB RAM
- [ ] Time travel queries < 10ms
- [ ] Genome export < 1KB for any DB size

---

## Design Principles (Airtight Logic)

### 1. Everything is a Distinction
- Data = distinction
- Auth = distinction  
- Sync = distinction exchange
- Config = distinction

### 2. Causality is Primary
- Every distinction has causal parents
- Time travel = causal graph traversal
- Merge = LCA computation

### 3. Memory is Layered (Like Brain)
- Hot: Working (fast, bounded)
- Warm: Recent (full detail, disk)
- Cold: Consolidated (compressed)
- Deep: Genomic (minimal, portable)

### 4. System is Self-Managing
- Compaction = natural selection
- Sync = set reconciliation
- Auth = graph traversal
- No manual tuning needed

### 5. Simplicity Through Depth
- Complex internals → Simple UX
- User sees: `put()`, `get()`, `sync()`
- System handles: distinctions, causality, memory, auth

---

*Building the future, one distinction at a time.*
