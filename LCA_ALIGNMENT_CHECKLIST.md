# LCA Architecture Alignment Checklist

**Goal:** All KoruDelta components must implement `LocalCausalAgent`. No exceptions.  
**Philosophy:** The trait is the soul contract - every interaction goes through synthesis.  
**Status:** [ ] Not Started | [~] In Progress | [x] Complete | [!] Blocked

---

## Overview

This checklist aligns all remaining components to follow the LCA (Local Causal Agent) architecture.  
**The Law:** `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`

### Current State
- **8 agents** fully implement `LocalCausalAgent` trait ✅
- **7 agents** follow pattern but don't implement trait ⚠️
- **6 managers** have NO LCA at all 🔴
- **Goal:** 100% trait implementation across all interactive components

### Alignment Strategy
1. Add `local_root: Distinction` field
2. Create `{Component}Action` enum
3. Implement `LocalCausalAgent` trait
4. Refactor all mutations to use `synthesize_action()`
5. Zero regressions - all existing tests must pass

---

## Phase A: Critical Core Components (6 components)

### A.1 CausalStorage → StorageAgent

**File:** `src/storage.rs` (refactor), `src/storage_agent.rs` (new)

**Current State:**
- Has `engine: Arc<DistinctionEngine>`
- Direct `DashMap` mutations (current_state, version_store, tombstones)
- NO local_root, NO synthesis

**Target State:**
```rust
pub struct StorageAgent {
    local_root: Distinction,  // NEW
    engine: Arc<DistinctionEngine>,
    causal_graph: CausalGraph,
    reference_graph: ReferenceGraph,
    // State becomes synthesized distinctions, not direct maps
}

impl LocalCausalAgent for StorageAgent {
    type ActionData = StorageAction;
    
    fn get_current_root(&self) -> &Distinction { &self.local_root }
    fn update_local_root(&mut self, new_root: Distinction) { self.local_root = new_root; }
    
    fn synthesize_action(&mut self, action: StorageAction, engine: &Arc<DistinctionEngine>) 
        -> Distinction {
        // All storage operations become synthesis
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        
        // Apply action to state (only after synthesis)
        self.apply_action(&action);
        
        new_root
    }
}
```

**Actions to Implement:**
- [ ] `StorageAction::Put { namespace, key, value }`
- [ ] `StorageAction::Get { namespace, key }`
- [ ] `StorageAction::Delete { namespace, key }`
- [ ] `StorageAction::History { namespace, key }`
- [ ] `StorageAction::Query { pattern }`

**Refactoring Steps:**
- [ ] Add `local_root` field to `StorageAgent`
- [ ] Create `StorageAction` enum in `src/actions/mod.rs` (if not exists)
- [ ] Implement `LocalCausalAgent` trait
- [ ] Refactor `put()` to use `synthesize_action(StorageAction::Put)`
- [ ] Refactor `get()` to use `synthesize_action(StorageAction::Get)`
- [ ] Refactor `delete()` to use `synthesize_action(StorageAction::Delete)`
- [ ] Refactor `history()` to use `synthesize_action(StorageAction::History)`
- [ ] Refactor `query()` to use `synthesize_action(StorageAction::Query)`
- [ ] Remove direct `DashMap` mutations from public API
- [ ] Update `CausalStorage` references to `StorageAgent` throughout codebase
- [ ] Ensure all existing tests pass without modification

**Verification:**
- [ ] `LocalCausalAgent` trait implemented
- [ ] All storage operations synthesize
- [ ] State mutations only occur inside `apply_action()`
- [ ] 421+ tests passing

---

### A.2 LifecycleManager → LifecycleAgent

**File:** `src/lifecycle/mod.rs` (refactor)

**Current State:**
- Coordinates memory tier transitions
- Direct state management (AccessTracker, ImportanceScorer, TransitionPlanner)
- NO local_root, NO synthesis

**Target State:**
```rust
pub struct LifecycleAgent {
    local_root: Distinction,  // NEW
    config: LifecycleConfig,
    // Internal components become agents or synthesized state
}

impl LocalCausalAgent for LifecycleAgent {
    type ActionData = LifecycleAction;
    
    fn synthesize_action(&mut self, action: LifecycleAction, engine: &Arc<DistinctionEngine>) 
        -> Distinction {
        // Lifecycle decisions become synthesis
        let action_distinction = action.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_distinction);
        self.local_root = new_root.clone();
        
        self.apply_lifecycle_action(&action);
        new_root
    }
}
```

**Actions to Implement:**
- [ ] `LifecycleAction::EvaluateAccess { distinction_id }`
- [ ] `LifecycleAction::Promote { distinction_id, from_tier, to_tier }`
- [ ] `LifecycleAction::Demote { distinction_id, from_tier, to_tier }`
- [ ] `LifecycleAction::Transition { transitions: Vec<Transition> }`
- [ ] `LifecycleAction::UpdateThresholds { new_thresholds }`

**Refactoring Steps:**
- [ ] Add `local_root` field
- [ ] Create `LifecycleAction` enum
- [ ] Implement `LocalCausalAgent` trait
- [ ] Convert `AccessTracker` to synthesize `EvaluateAccess` actions
- [ ] Convert `TransitionPlanner` to synthesize `Transition` actions
- [ ] Refactor `run_lifecycle()` to use synthesis loop
- [ ] Ensure background process still works

**Verification:**
- [ ] Trait implemented
- [ ] Memory tier transitions synthesize
- [ ] Background lifecycle process uses LCA

---

### A.3 SessionManager → SessionAgent

**File:** `src/auth/session.rs` (refactor)

**Current State:**
- Direct `DashMap` for session storage
- NO local_root, NO synthesis

**Target State:**
```rust
pub struct SessionAgent {
    local_root: Distinction,  // NEW
    // Sessions become synthesized distinctions
}

impl LocalCausalAgent for SessionAgent {
    type ActionData = SessionAction;
    
    fn synthesize_action(&mut self, action: SessionAction, engine: &Arc<DistinctionEngine>) 
        -> Distinction {
        // Session operations become synthesis
    }
}
```

**Actions to Implement:**
- [ ] `SessionAction::CreateSession { identity_key, capabilities }`
- [ ] `SessionAction::ValidateSession { session_token }`
- [ ] `SessionAction::RefreshSession { session_token }`
- [ ] `SessionAction::InvalidateSession { session_token }`
- [ ] `SessionAction::RotateKeys { session_token }`

**Verification:**
- [ ] All session operations synthesize
- [ ] Auth flow still works

---

### A.4 SubscriptionManager → SubscriptionAgent

**File:** `src/subscriptions.rs` (refactor)

**Current State:**
- Direct subscriber registry mutations
- NO local_root, NO synthesis

**Target State:**
```rust
pub struct SubscriptionAgent {
    local_root: Distinction,  // NEW
}

impl LocalCausalAgent for SubscriptionAgent {
    type ActionData = SubscriptionAction;
}
```

**Actions to Implement:**
- [ ] `SubscriptionAction::Subscribe { query, subscriber_id }`
- [ ] `SubscriptionAction::Unsubscribe { subscription_id }`
- [ ] `SubscriptionAction::Notify { event }`
- [ ] `SubscriptionAction::UpdateQuery { subscription_id, new_query }`

**Verification:**
- [ ] Pub/sub operations synthesize
- [ ] Event notifications still work

---

### A.5 ProcessRunner → ProcessAgent

**File:** `src/processes/mod.rs` (refactor)

**Current State:**
- Direct process spawning and management
- NO local_root, NO synthesis

**Target State:**
```rust
pub struct ProcessAgent {
    local_root: Distinction,  // NEW
}

impl LocalCausalAgent for ProcessAgent {
    type ActionData = ProcessAction;
}
```

**Actions to Implement:**
- [ ] `ProcessAction::SpawnProcess { process_type, config }`
- [ ] `ProcessAction::PauseProcess { process_id }`
- [ ] `ProcessAction::ResumeProcess { process_id }`
- [ ] `ProcessAction::TerminateProcess { process_id }`
- [ ] `ProcessAction::Heartbeat { process_id }`

**Verification:**
- [ ] Background processes synthesize
- [ ] Process lifecycle managed through LCA

---

### A.6 ReconciliationManager → ReconciliationAgent

**File:** `src/reconciliation/mod.rs` (refactor)

**Current State:**
- Direct sync state management
- `strategy` field is dead_code
- NO local_root, NO synthesis

**Target State:**
```rust
pub struct ReconciliationAgent {
    local_root: Distinction,  // NEW
    network_root: Distinction,
    // Sync becomes synthesis
}

impl LocalCausalAgent for ReconciliationAgent {
    type ActionData = ReconciliationAction;
}
```

**Actions to Implement:**
- [ ] `ReconciliationAction::StartSync { peer_id }`
- [ ] `ReconciliationAction::ExchangeRoots { peer_frontier }`
- [ ] `ReconciliationAction::RequestDifferences { divergence_point }`
- [ ] `ReconciliationAction::ApplyDelta { changes }`
- [ ] `ReconciliationAction::ResolveConflict { conflict_id, resolution }`
- [ ] `ReconciliationAction::CompleteSync { peer_id }`

**Verification:**
- [ ] Distributed sync synthesizes
- [ ] Conflict resolution uses LCA

---

## Phase B: Partial Components - Add Trait (6 components)

These have `local_root` and `synthesize_action` but don't implement the trait.
They need to be formalized.

### B.1 WorkspaceAgent - Implement Trait

**File:** `src/workspace_agent.rs`

**Current:** Has `local_root`, internal synthesis, NO trait
**Target:** `impl LocalCausalAgent for WorkspaceAgent`

**Tasks:**
- [ ] Make `synthesize_workspace()` public as `synthesize_action()`
- [ ] Implement `LocalCausalAgent` trait
- [ ] Ensure `WorkspaceAction` is the `ActionData` type
- [ ] All 11 tests still pass

---

### B.2 VectorAgent - Implement Trait

**File:** `src/vector_agent.rs`

**Current:** Has `local_root`, internal synthesis, NO trait
**Target:** `impl LocalCausalAgent for VectorAgent`

**Tasks:**
- [ ] Make `index()` use public `synthesize_action()`
- [ ] Implement `LocalCausalAgent` trait
- [ ] Ensure `VectorAction` is the `ActionData` type
- [ ] All 13 tests still pass

---

### B.3 NetworkProcess - Implement Trait

**File:** `src/network_process.rs`

**Current:** Has `local_root`, `synthesize()` with different signature
**Target:** `impl LocalCausalAgent for NetworkProcess`

**Tasks:**
- [ ] Rename/refactor `synthesize()` to `synthesize_action()`
- [ ] Implement `LocalCausalAgent` trait
- [ ] Ensure `NetworkAction` is the `ActionData` type
- [ ] All 16 falsification tests still pass

---

### B.4 IdentityAgent - Implement Trait

**File:** `src/auth/manager.rs`

**Current:** Has `local_root`, private `synthesize_action()`
**Target:** `impl LocalCausalAgent for IdentityAgent`

**Tasks:**
- [ ] Make `synthesize_action()` and `update_local_root()` public
- [ ] Implement `LocalCausalAgent` trait
- [ ] Ensure `IdentityAction` is the `ActionData` type

---

### B.5 KoruDelta Core - Implement Trait

**File:** `src/core.rs`

**Current:** Has `local_root`, private `synthesize_action()`
**Target:** `impl<R: Runtime> LocalCausalAgent for KoruDeltaGeneric<R>`

**Tasks:**
- [ ] Make `synthesize_action()` public
- [ ] Implement `LocalCausalAgent` trait
- [ ] Ensure `StorageAction` is the `ActionData` type

---

### B.6 KoruOrchestrator - Implement Trait

**File:** `src/orchestrator.rs`

**Current:** Has `local_root`, private `synthesize_action()`
**Target:** `impl LocalCausalAgent for KoruOrchestrator`

**Tasks:**
- [ ] Make `synthesize_action()` public
- [ ] Implement `LocalCausalAgent` trait
- [ ] Ensure `PulseAction` is the `ActionData` type

---

## Phase C: Data Structures (No Changes Required)

These are passive data structures that don't need LCA:

- [ ] **AgentRegistry** - Registry metadata (already DONE via AgentInfo)
- [ ] **Config structs** - Pure configuration
- [ ] **Index types** (FlatIndex, HnswIndex) - Passive storage structures
- [ ] **Snapshot types** - Immutable data containers

---

## Phase D: Integration & Verification

### D.1 Unified Agent Registry

**File:** `src/orchestrator.rs` (enhance)

- [ ] All 20 agents registered in `AgentRegistry`
- [ ] Each agent exposes its `local_root` via trait
- [ ] Orchestrator can query any agent's root
- [ ] Cross-agent synthesis enabled

### D.2 Action Type Consolidation

**File:** `src/actions/mod.rs`

- [ ] `LifecycleAction` added to `KoruAction` enum
- [ ] `SessionAction` added to `KoruAction` enum
- [ ] `SubscriptionAction` added to `KoruAction` enum
- [ ] `ProcessAction` added to `KoruAction` enum
- [ ] `ReconciliationAction` added to `KoruAction` enum

Total: 14 → 19 action types

### D.3 Root Type Expansion

**File:** `src/roots.rs`

- [ ] `RootType::Lifecycle` added
- [ ] `RootType::Session` added
- [ ] `RootType::Subscription` added
- [ ] `RootType::Process` added
- [ ] `RootType::Reconciliation` added

Total: 14 → 19 root types

### D.4 Testing

- [ ] All 421+ existing tests pass
- [ ] New LCA contract tests for each converted component
- [ ] Cross-agent synthesis integration tests
- [ ] Zero regressions in any behavior

### D.5 Documentation

- [ ] Update ARCHITECTURE.md with 100% LCA coverage
- [ ] Document each agent's `ActionData` type
- [ ] Update examples to show trait usage

---

## Success Criteria

✅ **All interactive components implement `LocalCausalAgent`**
- 20 agents total (8 existing + 6 new + 6 trait additions)

✅ **No direct state mutations outside synthesis**
- All `DashMap`/`HashMap` mutations happen inside `apply_action()`

✅ **Unified formula everywhere**
- `ΔNew = ΔLocal_Root ⊕ ΔAction_Data`

✅ **Zero regressions**
- All existing tests pass
- All existing behavior preserved

✅ **Complete action coverage**
- 19 action types covering all operations

✅ **Complete root coverage**
- 19 canonical roots for all agents

---

## Timeline Estimate

| Phase | Components | Complexity | Est. Time |
|-------|-----------|------------|-----------|
| A.1 | StorageAgent | High (core) | 2-3 days |
| A.2-A.6 | 5 other agents | Medium | 3-4 days |
| B.1-B.6 | 6 trait implementations | Low | 1-2 days |
| D.1-D.5 | Integration & testing | Medium | 2-3 days |
| **Total** | **20 components** | | **8-12 days** |

---

**Owner:** AI Agent Team  
**Branch:** `lca-architecture`  
**Target:** 100% LCA compliance  
**Blockers:** None

---

**Last Updated:** 2026-02-14  
**Status:** Ready to begin Phase A
