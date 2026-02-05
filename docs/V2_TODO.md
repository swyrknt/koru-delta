# KoruDelta v2.0 Implementation TODO

> **Status:** Committed to full v2.0 distinction-driven architecture
> **Goal:** Revolutionary simplicity through principled design
> **Timeline:** 8 weeks to complete

## The User Experience Vision

### What Users Will Feel

**Before (v1.0):**
- "My database file is 10GB and growing"
- "Sync is slow, sends everything"
- "Why is auth so complicated?"
- "I ran out of memory"

**After (v2.0):**
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

## Phase 1: Foundation ✅ COMPLETE

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

## Phase 2: Distinction Engine Integration 🎯 IN PROGRESS

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

## Phase 3: Layered Memory 🎯

### The Logic
Users should never think about memory. The system should just work at any scale.

### Week 3a: HotMemory (Working Memory)
- [ ] Create `src/memory/hot.rs`
- [ ] Implement `HotMemory` with LRU cache:
  - [ ] `cache: LruCache<DistinctionId, Distinction>`
  - [ ] `current_state: DashMap<Context, DistinctionId>`
- [ ] Implement `get()` - access distinction
- [ ] Implement `put()` - add to hot
- [ ] Implement `evict()` - move to warm
- [ ] Write tests
- **User Benefit:** Fast access to recent data, bounded RAM

### Week 3b: WarmMemory (Recent Chronicle)
- [ ] Create `src/memory/warm.rs`
- [ ] Implement `WarmMemory`:
  - [ ] `chronicle: ProcessChronicle` (disk-based)
  - [ ] `index: DashMap<DistinctionId, FileOffset>`
  - [ ] `recent_window: VecDeque<DistinctionId>`
- [ ] Implement `append()` - add distinction
- [ ] Implement `get()` - fetch from disk
- [ ] Implement `should_distill()` - check if needs consolidation
- [ ] Write tests
- **User Benefit:** Full history available, but not in RAM

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
- [ ] Create "Why v2.0" explainer

---

## Progress Log

### 2026-02-05 - Foundation Complete
- ✅ CausalGraph: 9 tests passing
- ✅ ReferenceGraph: 7 tests passing
- ✅ 16 new tests total, all green
- 🎯 Next: DistinctionEngine integration

### Success Metrics

By v2.0 completion:
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
