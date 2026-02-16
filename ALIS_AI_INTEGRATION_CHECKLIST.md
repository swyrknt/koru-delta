# ALIS AI Integration Checklist

**Purpose:** Track implementation of ALIS AI-specific features in KoruDelta  
**Target:** Complete Delta Agent API for ALIS AI Memory Consciousness  
**Architecture:** Local Causal Agent (LCA) pattern throughout  
**Estimated Duration:** 3-5 days  
**Status:** [ ] Not Started | [~] In Progress | [x] Complete

---

## Overview

This checklist tracks the implementation of graph-aware APIs needed for ALIS AI's Delta Agent (Memory Consciousness). These features extend KoruDelta's foundation (storage + similarity search) with causal graph operations.

### Current State (KoruDelta 3.0.0)

✅ **Already Working:**
- `put_similar()` / `find_similar()` - Semantic storage/search
- `embed()` / `embed_search()` - Vector operations
- `history()` / `get_at()` - Time travel
- Namespaces, metadata, statistics

❌ **Missing for ALIS:**
- TTL (Time-To-Live) with automatic expiration
- Graph connectivity queries
- Similar unconnected pair finding
- Random walk for dream phase

---

## Phase 1: TTL (Time-To-Live) Support

**Purpose:** Predictions and temporary distinctions need automatic expiration  
**Use Case:** Expression agent's active inference loop

### 1.1 Core TTL Storage

**File:** `src/core.rs`

- [ ] Add `expires_at` field to internal storage metadata
- [ ] Implement `put_with_ttl()` method:
  ```rust
  pub async fn put_with_ttl(
      &self,
      namespace: impl Into<String>,
      key: impl Into<String>,
      value: impl Into<serde_json::Value>,
      ttl_ticks: u64,
  ) -> Result<VersionedValue, DeltaError>
  ```
- [ ] Implement `put_similar_with_ttl()` method:
  ```rust
  pub async fn put_similar_with_ttl(
      &self,
      namespace: impl Into<String>,
      key: impl Into<String>,
      content: impl Into<serde_json::Value>,
      metadata: Option<serde_json::Value>,
      ttl_ticks: u64,
  ) -> Result<(), DeltaError>
  ```

### 1.2 TTL Cleanup

**File:** `src/core.rs`

- [ ] Implement `cleanup_expired()` method:
  ```rust
  pub async fn cleanup_expired(&self) -> Result<usize, DeltaError>
  ```
- [ ] Returns count of expired items removed
- [ ] Automatic cleanup on every Nth operation (configurable)
- [ ] LCA Pattern: Cleanup is a `ConsolidationAction`

### 1.3 TTL Queries

**File:** `src/core.rs`

- [ ] Implement `get_ttl_remaining()`:
  ```rust
  pub async fn get_ttl_remaining(&self, namespace: &str, key: &str) -> Result<Option<u64>, DeltaError>
  ```
- [ ] Implement `list_expiring_soon()`:
  ```rust
  pub async fn list_expiring_soon(&self, within_ticks: u64) -> Vec<(String, String, u64)>
  ```

### 1.4 LCA Architecture Compliance

- [ ] Create `ConsolidationAction::CleanupExpired` variant
- [ ] TTL operations synthesize through local root
- [ ] Expiration events are content-addressed

**Tests:**
- [ ] TTL items expire correctly
- [ ] Cleanup removes expired items
- [ ] Non-expired items remain
- [ ] LCA synthesis advances root

---

## Phase 2: Graph Connectivity Queries

**Purpose:** Query causal relationships between distinctions  
**Use Case:** Expression agent needs highly-connected distinctions

### 2.1 Causal Graph Index

**File:** `src/causal_graph.rs` (extend existing)

- [ ] Add graph traversal cache
- [ ] Optimize BFS/DFS for repeated queries
- [ ] Track connectivity scores per distinction

### 2.2 Connectivity API

**File:** `src/core.rs`

- [ ] Implement `are_connected()`:
  ```rust
  pub async fn are_connected(
      &self,
      namespace_a: &str,
      key_a: &str,
      namespace_b: &str,
      key_b: &str,
  ) -> Result<bool, DeltaError>
  ```
- [ ] Uses BFS through causal graph
- [ ] Returns true if path exists between two distinctions

- [ ] Implement `get_connection_path()`:
  ```rust
  pub async fn get_connection_path(
      &self,
      namespace_a: &str,
      key_a: &str,
      namespace_b: &str,
      key_b: &str,
  ) -> Result<Option<Vec<String>>, DeltaError>
  ```
- [ ] Returns path of distinction IDs if connected

### 2.3 Highly-Connected Query

**File:** `src/core.rs`

- [ ] Implement `get_highly_connected()`:
  ```rust
  pub async fn get_highly_connected(
      &self,
      namespace: Option<&str>,
      k: usize,
  ) -> Result<Vec<ConnectedDistinction>, DeltaError>
  ```

- [ ] Define `ConnectedDistinction` struct:
  ```rust
  pub struct ConnectedDistinction {
      pub namespace: String,
      pub key: String,
      pub connection_score: u32,  // parents + children + neighbors
      pub parents: Vec<String>,
      pub children: Vec<String>,
  }
  ```

- [ ] Rank by: `parents.len() + children.len() + synthesis_events.len()`

### 2.4 LCA Architecture Compliance

- [ ] Create `LineageAction::QueryConnectivity` variant
- [ ] Create `LineageAction::QueryHighlyConnected` variant
- [ ] Graph queries synthesize through LineageAgent

**Tests:**
- [ ] Connected distinctions return true
- [ ] Unconnected distinctions return false
- [ ] Connection paths are correct
- [ ] Highly-connected ranking is accurate
- [ ] LCA synthesis advances root

---

## Phase 3: Similar Unconnected Pairs

**Purpose:** Find distinctions that are similar but not causally connected  
**Use Case:** Consolidation agent's proactive synthesis

### 3.1 Core Algorithm

**File:** `src/core.rs`

- [ ] Implement `find_similar_unconnected_pairs()`:
  ```rust
  pub async fn find_similar_unconnected_pairs(
      &self,
      namespace: Option<&str>,
      k: usize,
      similarity_threshold: f32,  // e.g., 0.7
  ) -> Result<Vec<UnconnectedPair>, DeltaError>
  ```

- [ ] Define `UnconnectedPair` struct:
  ```rust
  pub struct UnconnectedPair {
      pub namespace_a: String,
      pub key_a: String,
      pub namespace_b: String,
      pub key_b: String,
      pub similarity_score: f32,
  }
  ```

### 3.2 Algorithm Steps

1. Get all keys in namespace (or all namespaces)
2. For each key, compute embedding
3. Find similar keys (cosine similarity > threshold)
4. Filter out pairs that are already connected (use `are_connected()`)
5. Sort by similarity score
6. Return top k pairs

### 3.3 Performance Optimization

- [ ] Use embedding index for fast similarity search
- [ ] Batch connectivity checks
- [ ] Cache results for configurable duration

### 3.4 LCA Architecture Compliance

- [ ] Create `ConsolidationAction::FindUnconnectedPairs` variant
- [ ] Pair finding synthesizes through ConsolidationAgent
- [ ] Results are content-addressed and cached

**Tests:**
- [ ] Returns only unconnected pairs
- [ ] Similarity threshold filters correctly
- [ ] Results sorted by score
- [ ] Performance acceptable (< 100ms for 10k items)
- [ ] LCA synthesis advances root

---

## Phase 4: Random Walk for Dream Phase

**Purpose:** Creative synthesis through random distinction combinations  
**Use Case:** Sleep agent's dream phase

### 4.1 Random Walk API

**File:** `src/core.rs`

- [ ] Implement `random_walk_combinations()`:
  ```rust
  pub async fn random_walk_combinations(
      &self,
      n: usize,           // Number of combinations to return
      steps: usize,       // Random walk steps per combination
  ) -> Result<Vec<RandomCombination>, DeltaError>
  ```

- [ ] Define `RandomCombination` struct:
  ```rust
  pub struct RandomCombination {
      pub start_namespace: String,
      pub start_key: String,
      pub end_namespace: String,
      pub end_key: String,
      pub path: Vec<String>,  // Intermediate distinctions
      pub novelty_score: f32,  // Distance metric
  }
  ```

### 4.2 Random Walk Algorithm

1. Pick random starting distinction
2. Follow random causal link (parent or child)
3. Repeat for `steps` iterations
4. Record end distinction
5. Compute novelty score (path length / connectivity)
6. Return start→end combinations

### 4.3 Dream Event Storage

**File:** `src/core.rs`

- [ ] Implement `record_dream_synthesis()`:
  ```rust
  pub async fn record_dream_synthesis(
      &self,
      combination: &RandomCombination,
  ) -> Result<(), DeltaError>
  ```
- [ ] Stores in special `dream_synthesis` namespace

### 4.4 LCA Architecture Compliance

- [ ] Create `SleepAction::DreamSynthesis` variant
- [ ] Random walks synthesize through SleepAgent
- [ ] Dream events are content-addressed

**Tests:**
- [ ] Random walks produce varied results
- [ ] Paths are valid causal chains
- [ ] Novelty scores are reasonable
- [ ] Dream events stored correctly
- [ ] LCA synthesis advances root

---

## Phase 5: Python Bindings Updates

**File:** `bindings/python/src/database.rs`

### 5.1 TTL Methods

- [ ] `put_with_ttl()` - Python wrapper
- [ ] `put_similar_with_ttl()` - Python wrapper
- [ ] `cleanup_expired()` - Python wrapper
- [ ] `get_ttl_remaining()` - Python wrapper
- [ ] `list_expiring_soon()` - Python wrapper

### 5.2 Graph Connectivity Methods

- [ ] `are_connected()` - Python wrapper
- [ ] `get_connection_path()` - Python wrapper
- [ ] `get_highly_connected()` - Python wrapper with result conversion

### 5.3 Similar Unconnected Pairs

- [ ] `find_similar_unconnected_pairs()` - Python wrapper

### 5.4 Random Walk

- [ ] `random_walk_combinations()` - Python wrapper
- [ ] `record_dream_synthesis()` - Python wrapper

### 5.5 Type Definitions

- [ ] Add Python classes: `ConnectedDistinction`, `UnconnectedPair`, `RandomCombination`
- [ ] Update `__init__.py` exports

**Tests:**
- [ ] All new methods work from Python
- [ ] Type conversions correct
- [ ] Async/await works properly

---

## Phase 6: WASM/JavaScript Bindings Updates

**File:** `src/wasm.rs`

### 6.1 TTL Methods

- [ ] `put_with_ttl_js()` - WASM export
- [ ] `put_similar_with_ttl_js()` - WASM export
- [ ] `cleanup_expired_js()` - WASM export
- [ ] `get_ttl_remaining_js()` - WASM export
- [ ] `list_expiring_soon_js()` - WASM export

### 6.2 Graph Connectivity Methods

- [ ] `are_connected_js()` - WASM export
- [ ] `get_connection_path_js()` - WASM export
- [ ] `get_highly_connected_js()` - WASM export

### 6.3 Similar Unconnected Pairs

- [ ] `find_similar_unconnected_pairs_js()` - WASM export

### 6.4 Random Walk

- [ ] `random_walk_combinations_js()` - WASM export
- [ ] `record_dream_synthesis_js()` - WASM export

### 6.5 TypeScript Definitions

**File:** `bindings/javascript/index.d.ts`

- [ ] Add interfaces: `ConnectedDistinction`, `UnconnectedPair`, `RandomCombination`
- [ ] Add all new method signatures
- [ ] Add JSDoc documentation

### 6.6 Package Update

**File:** `bindings/javascript/package.json`

- [ ] Update version to 3.1.0 (new features)

**Tests:**
- [ ] All methods work from JavaScript
- [ ] TypeScript types correct
- [ ] Browser and Node.js both work

---

## Phase 7: Integration & Validation

### 7.1 ALIS AI Example Update

**File:** `examples/alis_ai_integration.rs`

- [ ] Add TTL demonstration
- [ ] Add graph connectivity queries
- [ ] Add similar unconnected pairs finding
- [ ] Add dream phase with random walks
- [ ] Verify all ALIS requirements met

### 7.2 Documentation

- [ ] Document TTL usage patterns
- [ ] Document graph query API
- [ ] Add ALIS AI integration guide
- [ ] Update ARCHITECTURE.md with new features

### 7.3 Final Validation

Run validation checklist:

| Feature | Status | Verified |
|---------|--------|----------|
| TTL storage | [ ] | |
| TTL cleanup | [ ] | |
| TTL queries | [ ] | |
| are_connected() | [ ] | |
| get_connection_path() | [ ] | |
| get_highly_connected() | [ ] | |
| find_similar_unconnected_pairs() | [ ] | |
| random_walk_combinations() | [ ] | |
| Python bindings | [ ] | |
| WASM/JS bindings | [ ] | |
| Zero warnings | [ ] | |
| All tests pass | [ ] | |

---

## LCA Architecture Alignment

Every new feature must follow the Local Causal Agent pattern:

```rust
// Formula: ΔNew = ΔLocal_Root ⊕ ΔAction_Data

// Example: TTL cleanup
impl LocalCausalAgent for ConsolidationAgent {
    fn synthesize_action(&mut self, action: ConsolidationAction, engine: &Arc<DistinctionEngine>) 
        -> Distinction 
    {
        match action {
            ConsolidationAction::CleanupExpired => {
                // Perform cleanup
                let cleaned_count = cleanup_expired_items();
                
                // Create action distinction
                let action_data = json!({
                    "type": "cleanup_expired",
                    "cleaned": cleaned_count,
                    "timestamp": Utc::now(),
                });
                
                // Synthesize: ΔNew = ΔLocal_Root ⊕ ΔAction_Data
                let action_distinction = engine.canonicalize(&action_data);
                let new_root = engine.synthesize(&self.local_root, &action_distinction);
                
                self.update_local_root(new_root.clone());
                new_root
            }
            // ...
        }
    }
}
```

### Required Action Types

Add to `src/actions/mod.rs`:

```rust
pub enum ConsolidationAction {
    CleanupExpired,
    FindUnconnectedPairs { k: usize, threshold: f32 },
    // ...
}

pub enum LineageAction {
    QueryConnectivity { a: String, b: String },
    QueryHighlyConnected { k: usize },
    // ...
}

pub enum SleepAction {
    DreamSynthesis { n: usize, steps: usize },
    // ...
}
```

---

## Time Estimate

| Phase | Duration | Complexity |
|-------|----------|------------|
| 1: TTL Support | 1 day | Medium |
| 2: Graph Connectivity | 1 day | Medium |
| 3: Similar Unconnected Pairs | 0.5 day | Low |
| 4: Random Walk | 0.5 day | Low |
| 5: Python Bindings | 1 day | Medium |
| 6: WASM/JS Bindings | 1 day | Medium |
| 7: Integration & Docs | 0.5 day | Low |
| **Total** | **~5 days** | |

---

## Success Criteria

- [ ] All 4 missing APIs implemented
- [ ] All bindings (Python, WASM/JS) updated
- [ ] ALIS AI example demonstrates all features
- [ ] Zero compiler warnings
- [ ] All existing tests pass (608)
- [ ] New tests added for all features
- [ ] Documentation complete
- [ ] ALIS AI team confirms requirements met

---

## Next Steps

1. **Review this checklist** with ALIS AI team
2. **Confirm priority** (which features are must-have vs nice-to-have)
3. **Begin implementation** starting with Phase 1 (TTL)
4. **Daily check-ins** on progress

---

**Created:** 2026-02-16  
**Status:** Ready for implementation  
**Owner:** AI Agent Team
