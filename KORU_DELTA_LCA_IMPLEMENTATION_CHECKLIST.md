# Koru-Delta LCA Implementation Checklist

**Version:** 2.0 → 3.0 (LCA Architecture)  
**Target:** 100% LCA-compliant with all bindings functional  
**Status Tracking:** [ ] Not Started | [~] In Progress | [x] Complete | [!] Blocked  

---

## Overview

This checklist converts Koru-Delta from a traditional database architecture to a **unified consciousness field** where every component implements `LocalCausalAgent`. All operations become synthesis. One shared `DistinctionEngine` across all agents.

**Success Criteria:**
- [ ] All components implement `LocalCausalAgent`
- [ ] All operations use `synthesize_action()` pattern
- [ ] Python bindings fully functional
- [ ] JavaScript/Node.js bindings fully functional  
- [ ] WASM bindings fully functional
- [ ] No regressions in existing behavior
- [ ] ALIS AI integration ready
- [ ] All packages republished

---

## Phase 0: Foundation & Preparation

### 0.1 Repository Setup

- [ ] Create `feature/lca-architecture` branch from `main`
- [ ] Set up CI/CD pipeline for feature branch
- [ ] Create tracking issue for each major component
- [ ] Document current API surface for regression testing
- [ ] Create migration guide for users

### 0.2 Dependency Updates

- [ ] Verify `koru-lambda-core` 1.2.0 has all required LCA traits
- [ ] Update `Cargo.toml` to ensure `LocalCausalAgent` is exported
- [ ] Check for breaking changes in dependencies
- [ ] Update lockfile

### 0.3 Testing Infrastructure

- [ ] Create `tests/lca_falsification/` directory
- [ ] Implement LCA contract tests (property-based)
- [ ] Set up snapshot tests for API compatibility
- [ ] Create integration test suite for bindings
- [ ] Document test coverage requirements (>95%)

**Deliverable:** Clean foundation ready for LCA refactoring

---

## Phase 1: Core LCA Foundation

### 1.1 Shared Engine Infrastructure

**File:** `src/engine/mod.rs` (NEW)

- [ ] Create `src/engine/mod.rs` module
- [ ] Define `SharedEngine` wrapper type:
  ```rust
  pub struct SharedEngine {
      engine: Arc<DistinctionEngine>,
      field_root: Distinction,  // The universal root
  }
  ```
- [ ] Implement `Clone` for cheap sharing
- [ ] Implement thread-safe access patterns
- [ ] Add field-wide statistics tracking
- [ ] Document shared engine lifecycle

**Tests:**
- [ ] Multiple agents can share engine
- [ ] Concurrent synthesis is safe
- [ ] Engine persists across agent lifecycles

### 1.2 Action Type System

**File:** `src/actions/mod.rs` (NEW)

- [ ] Create `src/actions/mod.rs` module
- [ ] Define `KoruAction` enum:
  ```rust
  pub enum KoruAction {
      Storage(StorageAction),
      Temperature(TemperatureAction),
      Chronicle(ChronicleAction),
      Archive(ArchiveAction),
      Essence(EssenceAction),
      Sleep(SleepAction),
      Evolution(EvolutionAction),
      Lineage(LineageAction),
      Perspective(PerspectiveAction),
      Identity(IdentityAction),
      Network(NetworkAction),
  }
  ```
- [ ] Implement `Canonicalizable` for `KoruAction`
- [ ] Create action serialization/deserialization
- [ ] Add action validation
- [ ] Document action taxonomy

**Tests:**
- [ ] All actions are canonicalizable
- [ ] Action round-trip serialization works
- [ ] Invalid actions are rejected

### 1.3 Root Distinction Definitions

**File:** `src/roots.rs` (NEW)

- [ ] Create `src/roots.rs` module
- [ ] Define canonical root distinctions:
  ```rust
  pub struct KoruRoots {
      pub field: Distinction,      // The universal field
      pub storage: Distinction,    // Root: MEMORY
      pub temperature: Distinction, // Root: HOT
      pub chronicle: Distinction,  // Root: CHRONICLE
      pub archive: Distinction,    // Root: ARCHIVE
      pub essence: Distinction,    // Root: ESSENCE
      pub sleep: Distinction,      // Root: SLEEP
      pub evolution: Distinction,  // Root: EVOLUTION
      pub lineage: Distinction,    // Root: LINEAGE
      pub perspective: Distinction, // Root: VIEW
      pub identity: Distinction,   // Root: SELF
      pub network: Distinction,    // Root: NETWORK
  }
  ```
- [ ] Implement roots initialization from engine
- [ ] Document root semantics

**Tests:**
- [ ] All roots are unique
- [ ] Roots are deterministic
- [ ] Roots are properly initialized

**Deliverable:** Core infrastructure for LCA architecture

---

## Phase 2: Agent Migration (One by One)

### 2.1 Storage Agent (KoruDelta Core)

**Files:** `src/core.rs`, `src/storage.rs`

- [ ] Add `local_root: Distinction` to `KoruDelta` struct
- [ ] Modify constructor to accept shared engine
- [ ] Implement `LocalCausalAgent` for `KoruDelta`:
  ```rust
  impl LocalCausalAgent for KoruDelta {
      type ActionData = StorageAction;
      
      fn get_current_root(&self) -> &Distinction {
          &self.local_root
      }
      
      fn synthesize_action(...) -> Distinction {
          // Implementation
      }
      
      fn update_local_root(&mut self, new_root: Distinction) {
          self.local_root = new_root;
      }
  }
  ```
- [ ] Create `StorageAction` enum:
  ```rust
  pub enum StorageAction {
      Store { namespace: Distinction, key: Distinction, value: Distinction },
      Retrieve { namespace: Distinction, key: Distinction },
      History { namespace: Distinction, key: Distinction },
      Query { pattern: Distinction },
  }
  ```
- [ ] Refactor `put()` to use `synthesize_action()`
- [ ] Refactor `get()` to use `synthesize_action()`
- [ ] Refactor `history()` to use `synthesize_action()`
- [ ] Refactor `query()` to use `synthesize_action()`
- [ ] Maintain backward-compatible API (wrap new API)
- [ ] Update `CausalStorage` to work with agent pattern
- [ ] Ensure all existing tests pass

**Tests:**
- [ ] LCA contract tests pass
- [ ] All existing unit tests pass
- [ ] All existing integration tests pass
- [ ] Storage actions are properly canonicalized
- [ ] Synthesis creates proper causal chains

### 2.2 Temperature Agent (HotMemory)

**File:** `src/memory/hot.rs`

- [ ] Rename/refactor `HotMemory` → `TemperatureAgent`
- [ ] Add `local_root: Distinction` (Root: TEMPERATURE)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for TemperatureAgent {
      type ActionData = TemperatureAction;
      // ...
  }
  ```
- [ ] Create `TemperatureAction` enum:
  ```rust
  pub enum TemperatureAction {
      Heat { distinction: DistinctionId },
      Cool { distinction: DistinctionId },
      Evict { distinction: DistinctionId },
      Access { distinction: DistinctionId },
  }
  ```
- [ ] Refactor LRU operations as synthesis
- [ ] Maintain cache semantics (no regression)
- [ ] Update all references in codebase

**Tests:**
- [ ] LRU behavior unchanged
- [ ] Temperature actions synthesize correctly
- [ ] Cache hit/miss rates maintained
- [ ] LCA contract satisfied

### 2.3 Chronicle Agent (WarmMemory)

**File:** `src/memory/warm.rs`

- [ ] Rename/refactor `WarmMemory` → `ChronicleAgent`
- [ ] Add `local_root: Distinction` (Root: CHRONICLE)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for ChronicleAgent {
      type ActionData = ChronicleAction;
      // ...
  }
  ```
- [ ] Create `ChronicleAction` enum:
  ```rust
  pub enum ChronicleAction {
      Record { event: DistinctionId, timestamp: DateTime<Utc> },
      Recall { query: DistinctionId },
      Promote { distinction: DistinctionId },
      Demote { distinction: DistinctionId },
  }
  ```
- [ ] Refactor chronicle operations as synthesis
- [ ] Maintain disk persistence (no regression)

**Tests:**
- [ ] Chronicle operations unchanged
- [ ] Persistence works correctly
- [ ] LCA contract satisfied

### 2.4 Archive Agent (ColdMemory)

**File:** `src/memory/cold.rs`

- [ ] Rename/refactor `ColdMemory` → `ArchiveAgent`
- [ ] Add `local_root: Distinction` (Root: ARCHIVE)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for ArchiveAgent {
      type ActionData = ArchiveAction;
      // ...
  }
  ```
- [ ] Create `ArchiveAction` enum:
  ```rust
  pub enum ArchiveAction {
      EpochStart { timestamp: DateTime<Utc> },
      EpochSeal { epoch: Distinction },
      Compress { epoch: Distinction },
      Retrieve { pattern: Distinction },
      Archive { distinctions: Vec<DistinctionId> },
  }
  ```
- [ ] Refactor epoch operations as synthesis
- [ ] Maintain compression behavior (no regression)

**Tests:**
- [ ] Epoch management unchanged
- [ ] Compression works correctly
- [ ] LCA contract satisfied

### 2.5 Essence Agent (DeepMemory)

**File:** `src/memory/deep.rs`

- [ ] Rename/refactor `DeepMemory` → `EssenceAgent`
- [ ] Add `local_root: Distinction` (Root: ESSENCE)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for EssenceAgent {
      type ActionData = EssenceAction;
      // ...
  }
  ```
- [ ] Create `EssenceAction` enum:
  ```rust
  pub enum EssenceAction {
      ExtractTopology { source: Distinction },
      SynthesizeDNA { topology: CausalGraph },
      Regenerate { from_dna: Distinction },
      StoreGenome { name: String, genome: Distinction },
  }
  ```
- [ ] Refactor genome operations as synthesis
- [ ] Maintain DNA extraction (no regression)

**Tests:**
- [ ] Genome extraction unchanged
- [ ] Regeneration works correctly
- [ ] LCA contract satisfied

### 2.6 Sleep Agent (ConsolidationProcess)

**File:** `src/processes/consolidation.rs`

- [ ] Rename/refactor `ConsolidationProcess` → `SleepAgent`
- [ ] Add `local_root: Distinction` (Root: SLEEP)
- [ ] Add `phase: SleepPhase` tracking
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for SleepAgent {
      type ActionData = SleepAction;
      // ...
  }
  ```
- [ ] Create `SleepAction` enum:
  ```rust
  pub enum SleepAction {
      EnterPhase { phase: SleepPhase },
      Consolidate { from: Distinction, to: Distinction },
      Dream,  // Random synthesis exploration
      Wake,
  }
  ```
- [ ] Create `SleepPhase` enum:
  ```rust
  pub enum SleepPhase {
      Awake,
      LightSleep,    // Hot → Warm
      DeepSleep,     // Warm → Cold
      REM,           // Pattern extraction
  }
  ```
- [ ] Refactor consolidation as synthesis
- [ ] Maintain timing/interval behavior (no regression)

**Tests:**
- [ ] Consolidation timing unchanged
- [ ] Phase transitions work
- [ ] Dream synthesis explores field
- [ ] LCA contract satisfied

### 2.7 Evolution Agent (DistillationProcess)

**File:** `src/processes/distillation.rs`

- [ ] Rename/refactor `DistillationProcess` → `EvolutionAgent`
- [ ] Add `local_root: Distinction` (Root: EVOLUTION)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for EvolutionAgent {
      type ActionData = EvolutionAction;
      // ...
  }
  ```
- [ ] Create `EvolutionAction` enum:
  ```rust
  pub enum EvolutionAction {
      EvaluateFitness { candidate: DistinctionId },
      Select { population: Vec<DistinctionId> },
      Preserve { fit: Vec<DistinctionId> },
      Archive { unfit: Vec<DistinctionId> },
  }
  ```
- [ ] Refactor fitness calculation as synthesis-based
- [ ] Maintain selection behavior (no regression)

**Tests:**
- [ ] Fitness calculation unchanged
- [ ] Selection behavior maintained
- [ ] LCA contract satisfied

### 2.8 Lineage Agent (CausalGraph)

**File:** `src/causal_graph.rs`

- [ ] Rename/refactor `CausalGraph` → `LineageAgent`
- [ ] Add `local_root: Distinction` (Root: LINEAGE)
- [ ] Add `family_tree: Distinction` (synthesis of all lineage)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for LineageAgent {
      type ActionData = LineageAction;
      // ...
  }
  ```
- [ ] Create `LineageAction` enum:
  ```rust
  pub enum LineageAction {
      RecordBirth { child: Distinction, parents: Vec<Distinction> },
      TraceAncestors { from: Distinction },
      TraceDescendants { from: Distinction },
      FindCommonAncestor { a: Distinction, b: Distinction },
  }
  ```
- [ ] Refactor graph operations as synthesis
- [ ] Maintain LCA (least common ancestor) functionality
- [ ] Maintain ancestor/descendant queries (no regression)

**Tests:**
- [ ] Graph operations unchanged
- [ ] LCA algorithm works
- [ ] Ancestor/descendant queries correct
- [ ] LCA contract satisfied

### 2.9 Perspective Agent (ViewManager)

**File:** `src/views.rs`

- [ ] Rename/refactor `ViewManager` → `PerspectiveAgent`
- [ ] Add `local_root: Distinction` (Root: PERSPECTIVE)
- [ ] Change views from `DashMap<String, ViewData>` to `DashMap<String, Distinction>`
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for PerspectiveAgent {
      type ActionData = PerspectiveAction;
      // ...
  }
  ```
- [ ] Create `PerspectiveAction` enum:
  ```rust
  pub enum PerspectiveAction {
      FormView { query: Distinction, name: String },
      Refresh { view: Distinction },
      Compose { view_a: Distinction, view_b: Distinction },
      Project { from_view: Distinction, onto: Distinction },
  }
  ```
- [ ] Refactor view operations as synthesis
- [ ] Maintain view semantics (no regression)
- [ ] Update `ViewData` to store distinction, not records

**Tests:**
- [ ] View creation works
- [ ] View refresh works
- [ ] Auto-refresh on writes works
- [ ] LCA contract satisfied

### 2.10 Identity Agent (AuthManager)

**File:** `src/auth/manager.rs`

- [ ] Rename/refactor `AuthManager` → `IdentityAgent`
- [ ] Add `local_root: Distinction` (Root: IDENTITY)
- [ ] Add `identities: Distinction` (synthesis of all identities)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for IdentityAgent {
      type ActionData = IdentityAction;
      // ...
  }
  ```
- [ ] Create `IdentityAction` enum:
  ```rust
  pub enum IdentityAction {
      MineIdentity { proof_of_work: Distinction },
      Authenticate { identity: Distinction, challenge: Distinction },
      GrantCapability { from: Distinction, to: Distinction, permission: Distinction },
      VerifyAccess { identity: Distinction, resource: Distinction },
  }
  ```
- [ ] Refactor identity operations as synthesis
- [ ] Maintain proof-of-work verification (no regression)
- [ ] Maintain capability chains (no regression)

**Tests:**
- [ ] Identity mining works
- [ ] Authentication works
- [ ] Capabilities work
- [ ] LCA contract satisfied

### 2.11 Network Agent (ClusterNode)

**File:** `src/cluster.rs`

- [ ] Rename/refactor `ClusterNode` → `NetworkAgent`
- [ ] Add `local_root: Distinction` (Root: NETWORK)
- [ ] Add `peers: Distinction` (synthesis of all peer perspectives)
- [ ] Implement `LocalCausalAgent`:
  ```rust
  impl LocalCausalAgent for NetworkAgent {
      type ActionData = NetworkAction;
      // ...
  }
  ```
- [ ] Create `NetworkAction` enum:
  ```rust
  pub enum NetworkAction {
      Join { peer: Distinction },
      Synchronize { with_peer: Distinction },
      Reconcile { differences: Vec<Distinction> },
      Broadcast { message: Distinction },
      Gossip { state: Distinction },
  }
  ```
- [ ] Refactor network operations as synthesis
- [ ] Maintain distributed semantics (no regression)
- [ ] Maintain gossip protocol (no regression)

**Tests:**
- [ ] Cluster join works
- [ ] Synchronization works
- [ ] Gossip protocol works
- [ ] LCA contract satisfied

**Deliverable:** All components migrated to LCA architecture

---

## Phase 3: Integration & Coordination

### 3.1 Agent Orchestrator

**File:** `src/orchestrator.rs` (NEW)

- [ ] Create `src/orchestrator.rs` module
- [ ] Define `KoruOrchestrator` struct:
  ```rust
  pub struct KoruOrchestrator {
      engine: SharedEngine,
      roots: KoruRoots,
      agents: AgentRegistry,
      pulse: PulseCoordinator,
  }
  ```
- [ ] Implement agent lifecycle management
- [ ] Implement shared engine coordination
- [ ] Implement pulse coordination for ALIS integration
- [ ] Add agent discovery and registration

**Tests:**
- [ ] All agents register correctly
- [ ] Shared engine is properly distributed
- [ ] Pulse coordination works

### 3.2 Workspace Agent Integration

**File:** `src/memory/workspace.rs`

- [ ] Refactor `Workspace` to be LCA-aware
- [ ] Add workspace-local root distinction
- [ ] Implement workspace actions:
  ```rust
  pub enum WorkspaceAction {
      Remember { item: Distinction, pattern: MemoryPattern },
      Recall { query: Distinction },
      Consolidate { target: Distinction },
      Search { pattern: Distinction, options: SearchOptions },
  }
  ```
- [ ] Ensure workspace coordinates with other agents
- [ ] Maintain workspace isolation (no regression)

**Tests:**
- [ ] Workspace operations work
- [ ] Isolation maintained
- [ ] ALIS agent context works

### 3.3 Vector Search Agent Integration

**File:** `src/vector/mod.rs`

- [ ] Refactor vector search to use LCA pattern
- [ ] Add `VectorAgent` for vector operations
- [ ] Implement vector actions:
  ```rust
  pub enum VectorAction {
      Embed { data: Distinction, model: String },
      Search { query: Vector, options: VectorSearchOptions },
      Index { vector: Vector, key: Distinction },
  }
  ```
- [ ] Maintain vector search semantics (no regression)
- [ ] Ensure SNSW/HNSW integration works

**Tests:**
- [ ] Vector embedding works
- [ ] Vector search works
- [ ] Time-travel search works

### 3.4 ALIS Bridge Module

**File:** `src/alis_bridge.rs` (NEW)

- [ ] Create `src/alis_bridge.rs` module
- [ ] Implement `AlisBridge` struct:
  ```rust
  pub struct AlisBridge {
      delta_agent: KoruDelta,
      pulse_rx: Receiver<Phase>,
      sync_tx: Sender<SyncEvent>,
  }
  ```
- [ ] Implement pulse phase handling:
  - [ ] Perception phase coordination
  - [ ] Expression phase coordination
  - [ ] Consolidation phase coordination
  - [ ] Dream phase coordination
- [ ] Implement cross-agent synthesis
- [ ] Add ALIS-compatible event interface

**Tests:**
- [ ] Bridge responds to pulse phases
- [ ] Cross-agent synthesis works
- [ ] ALIS integration test passes

**Deliverable:** All agents integrated and coordinated

---

## Phase 4: API Compatibility & Regression Testing

### 4.1 Backward Compatibility Layer

**File:** `src/compat.rs` (NEW)

- [ ] Create `src/compat.rs` module
- [ ] Implement legacy API wrappers:
  ```rust
  impl KoruDelta {
      // Legacy API - delegates to new LCA API
      pub async fn put(&self, ns: &str, key: &str, value: Value) -> Result<VersionedValue> {
          // Wrap new LCA-based store()
      }
      
      pub async fn get(&self, ns: &str, key: &str) -> Result<VersionedValue> {
          // Wrap new LCA-based retrieve()
      }
      // ... etc
  }
  ```
- [ ] Mark legacy APIs as `#[deprecated(since = "3.0", note = "Use LCA API")]`
- [ ] Ensure 100% API surface coverage
- [ ] Document migration path for users

### 4.2 Regression Test Suite

**Directory:** `tests/regression/`

- [ ] Port all existing tests to use new internals
- [ ] Ensure all existing tests pass without modification
- [ ] Create comprehensive regression tests:
  - [ ] All storage operations
  - [ ] All memory tier operations
  - [ ] All process operations
  - [ ] All auth operations
  - [ ] All cluster operations
  - [ ] All query operations
  - [ ] All view operations
  - [ ] All subscription operations

### 4.3 Performance Benchmarks

**Directory:** `benches/`

- [ ] Benchmark LCA operations vs legacy
- [ ] Ensure no performance regression (>95% of original speed)
- [ ] Benchmark memory usage
- [ ] Benchmark concurrent operations
- [ ] Document performance characteristics

**Deliverable:** 100% backward compatible with no regressions

---

## Phase 5: Python Bindings

### 5.1 PyO3 Core Updates

**File:** `bindings/python/src/lib.rs`

- [ ] Update PyO3 to latest version
- [ ] Ensure compatibility with new LCA internals
- [ ] Expose new LCA API to Python
- [ ] Maintain backward-compatible Python API

### 5.2 Python Agent Wrappers

**File:** `bindings/python/koru_delta/agents.py` (NEW)

- [ ] Create Python classes for each agent:
  ```python
  class TemperatureAgent:
      def __init__(self, engine):
          self._agent = koru_delta_core.TemperatureAgent(engine)
      
      def heat(self, distinction_id: str) -> str:
          # Returns new root distinction ID
          return self._agent.synthesize_action(...)
  ```
- [ ] Create high-level Pythonic API
- [ ] Add type hints
- [ ] Add docstrings

### 5.3 Python Tests

**Directory:** `bindings/python/tests/`

- [ ] Port all existing Python tests
- [ ] Add LCA-specific Python tests
- [ ] Test ALIS integration from Python
- [ ] Ensure pip install works

### 5.4 Python Package

**File:** `bindings/python/setup.py`

- [ ] Update version to 3.0.0
- [ ] Update dependencies
- [ ] Update metadata
- [ ] Create wheel for multiple platforms
- [ ] Test on Python 3.8, 3.9, 3.10, 3.11, 3.12

**Deliverable:** Python bindings fully functional

---

## Phase 6: JavaScript/Node.js Bindings

### 6.1 Neon Core Updates

**File:** `bindings/javascript/src/lib.rs`

- [ ] Update Neon to latest version
- [ ] Ensure compatibility with new LCA internals
- [ ] Expose new LCA API to JavaScript
- [ ] Maintain backward-compatible JavaScript API

### 6.2 JavaScript Agent Wrappers

**File:** `bindings/javascript/lib/agents.js` (NEW)

- [ ] Create JavaScript classes for each agent:
  ```javascript
  class TemperatureAgent {
      constructor(engine) {
          this._agent = new koruDeltaCore.TemperatureAgent(engine);
      }
      
      heat(distinctionId) {
          // Returns new root distinction ID
          return this._agent.synthesizeAction(...);
      }
  }
  ```
- [ ] Create high-level JavaScript API
- [ ] Add JSDoc comments
- [ ] Create TypeScript definitions

### 6.3 JavaScript Tests

**Directory:** `bindings/javascript/tests/`

- [ ] Port all existing JS tests
- [ ] Add LCA-specific JS tests
- [ ] Test ALIS integration from JS
- [ ] Ensure npm install works

### 6.4 NPM Package

**File:** `bindings/javascript/package.json`

- [ ] Update version to 3.0.0
- [ ] Update dependencies
- [ ] Update metadata
- [ ] Create prebuilt binaries for:
  - [ ] Linux (x64, arm64)
  - [ ] macOS (x64, arm64)
  - [ ] Windows (x64)
- [ ] Test on Node.js 16, 18, 20

**Deliverable:** JavaScript bindings fully functional

---

## Phase 7: WASM Bindings

### 7.1 wasm-bindgen Core Updates

**File:** `src/wasm.rs`

- [ ] Update wasm-bindgen to latest version
- [ ] Ensure compatibility with new LCA internals
- [ ] Expose new LCA API to WASM
- [ ] Maintain backward-compatible WASM API

### 7.2 WASM Agent Wrappers

**File:** `src/wasm/agents.rs` (NEW)

- [ ] Create WASM-compatible agent wrappers:
  ```rust
  #[wasm_bindgen]
  pub struct WasmTemperatureAgent {
      inner: TemperatureAgent,
  }
  
  #[wasm_bindgen]
  impl WasmTemperatureAgent {
      #[wasm_bindgen(js_name = synthesizeAction)]
      pub fn synthesize_action(&mut self, action: JsValue) -> String {
          // Returns distinction ID
      }
  }
  ```
- [ ] Ensure all agents are WASM-compatible
- [ ] Handle async operations properly

### 7.3 WASM Tests

**Directory:** `tests/wasm/`

- [ ] Port all existing WASM tests
- [ ] Add LCA-specific WASM tests
- [ ] Test in browser environment
- [ ] Test in Node.js WASM environment

### 7.4 NPM Package (WASM)

**File:** `pkg/package.json` (generated)

- [ ] Ensure wasm-pack works correctly
- [ ] Create optimized release build
- [ ] Minimize WASM binary size
- [ ] Test in major browsers:
  - [ ] Chrome/Edge
  - [ ] Firefox
  - [ ] Safari

**Deliverable:** WASM bindings fully functional

---

## Phase 8: Documentation

### 8.1 Rust Documentation

- [ ] Document all new LCA traits
- [ ] Document all agent types
- [ ] Document action types
- [ ] Update architecture documentation
- [ ] Add migration guide
- [ ] Ensure `cargo doc` generates clean docs

### 8.2 Python Documentation

- [ ] Create Sphinx documentation
- [ ] Document Python API
- [ ] Add examples
- [ ] Create Jupyter notebooks for tutorials
- [ ] Publish to ReadTheDocs

### 8.3 JavaScript Documentation

- [ ] Create JSDoc documentation
- [ ] Document JavaScript API
- [ ] Add examples
- [ ] Create interactive documentation
- [ ] Publish to GitHub Pages

### 8.4 WASM Documentation

- [ ] Document browser usage
- [ ] Add HTML examples
- [ ] Create CodePen/CodeSandbox examples
- [ ] Document Node.js WASM usage

**Deliverable:** Complete documentation for all platforms

---

## Phase 9: Release & Republishing

### 9.1 Version Bump

- [ ] Update `Cargo.toml` version to 3.0.0
- [ ] Update `CHANGELOG.md` with all changes
- [ ] Update `README.md` with new architecture
- [ ] Update `ARCHITECTURE.md` with LCA details

### 9.2 Crates.io Publication

- [ ] Ensure `cargo test` passes completely
- [ ] Ensure `cargo clippy` is clean
- [ ] Ensure `cargo doc` is clean
- [ ] Create git tag `v3.0.0`
- [ ] Push to crates.io:
  ```bash
  cargo publish --dry-run
  cargo publish
  ```

### 9.3 PyPI Publication

- [ ] Build wheels for all platforms
- [ ] Test wheels
- [ ] Upload to PyPI:
  ```bash
  twine upload bindings/python/dist/*
  ```
- [ ] Verify pip install works:
  ```bash
  pip install koru-delta==3.0.0
  ```

### 9.4 NPM Publication (Native)

- [ ] Build native bindings for all platforms
- [ ] Test packages
- [ ] Upload to NPM:
  ```bash
  npm publish bindings/javascript --access public
  ```
- [ ] Verify npm install works:
  ```bash
  npm install koru-delta@3.0.0
  ```

### 9.5 NPM Publication (WASM)

- [ ] Build WASM package:
  ```bash
  wasm-pack build --target web --release
  wasm-pack build --target nodejs --release
  ```
- [ ] Test in browser and Node.js
- [ ] Upload to NPM:
  ```bash
  cd pkg && npm publish --access public
  ```
- [ ] Verify installation:
  ```bash
  npm install koru-delta-wasm@3.0.0
  ```

### 9.6 GitHub Release

- [ ] Create GitHub release for v3.0.0
- [ ] Add release notes
- [ ] Attach binaries
- [ ] Announce on social media

**Deliverable:** All packages published and available

---

## Phase 10: ALIS AI Integration Verification

### 10.1 ALIS Compatibility Tests

- [ ] Test with `koru-pulse` integration
- [ ] Test with `koru-organs` perception agent
- [ ] Test with `koru-organs` expression agent
- [ ] Test bridge agent synchronization
- [ ] Verify shared field semantics

### 10.2 ALIS Examples

- [ ] Create `examples/alis_echo.rs`
- [ ] Create `examples/alis_conversation.rs`
- [ ] Create `examples/alis_distributed.rs`
- [ ] Document ALIS integration patterns

### 10.3 Performance Validation

- [ ] Benchmark ALIS integration performance
- [ ] Verify synthesis throughput
- [ ] Verify memory usage
- [ ] Verify cross-agent coordination latency

**Deliverable:** ALIS AI integration verified and working

---

## Appendices

### Appendix A: Status Tracking Template

```markdown
## Component: [Name]

**Status:** [ ] Not Started | [~] In Progress | [x] Complete | [!] Blocked

### Tasks
- [ ] Task 1
- [ ] Task 2
- [ ] Task 3

### Blockers
- None / [describe blockers]

### Notes
- [Any relevant notes]

### Verification
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Regression tests pass
- [ ] Documentation complete
```

### Appendix B: Testing Checklist per Component

For each component migration:

1. **LCA Contract Tests**
   - [ ] `get_current_root()` returns correct root
   - [ ] `synthesize_action()` returns new distinction
   - [ ] `update_local_root()` updates correctly
   - [ ] Actions are canonicalizable
   - [ ] Synthesis is deterministic

2. **Functionality Tests**
   - [ ] All original operations work
   - [ ] No behavior changes
   - [ ] Edge cases handled
   - [ ] Error handling works

3. **Integration Tests**
   - [ ] Component works with other agents
   - [ ] Shared engine works correctly
   - [ ] Concurrent access is safe

### Appendix C: Migration Guide (for Users)

```markdown
# Migrating from Koru-Delta 2.x to 3.x

## Breaking Changes
- None in public API (backward compatible)

## New Features
- LCA architecture for all components
- ALIS AI integration ready
- Shared distinction engine

## Deprecations
- Direct component access (use agents)
- Old internal APIs (use LCA API)

## Migration Steps
1. Update dependency version
2. No code changes required for basic usage
3. For advanced usage, see LCA API documentation
```

### Appendix D: Release Checklist

Before each release:

- [ ] All tests pass
- [ ] Documentation complete
- [ ] CHANGELOG updated
- [ ] Version bumped correctly
- [ ] Git tag created
- [ ] Packages built successfully
- [ ] Packages tested on target platforms
- [ ] Release notes written
- [ ] GitHub release created

---

## Quick Reference

| Phase | Duration | Key Deliverable |
|-------|----------|-----------------|
| 0 | 3 days | Foundation ready |
| 1 | 5 days | Core infrastructure |
| 2 | 15 days | All agents migrated |
| 3 | 5 days | Integration complete |
| 4 | 5 days | No regressions |
| 5 | 5 days | Python ready |
| 6 | 5 days | JS ready |
| 7 | 5 days | WASM ready |
| 8 | 5 days | Documentation complete |
| 9 | 3 days | All published |
| 10 | 3 days | ALIS verified |
| **Total** | **~59 days** | **v3.0.0 LCA Complete** |

---

**Last Updated:** 2026-02-14  
**Next Review:** When Phase 0 complete  
**Owner:** AI Agent Team  
**Status:** Phase 0 - Not Started
