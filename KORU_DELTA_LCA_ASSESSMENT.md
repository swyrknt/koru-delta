# Koru-Delta LocalCausalAgent Assessment

**Assessment Date:** 2026-02-14  
**Koru-Delta Version:** 2.0.0  
**Koru-Lambda-Core Version:** 1.2.0 (current dependency)  
**ALIS AI Architecture Version:** v2.0

---

## Executive Summary

**Finding:** Koru-Delta is **NOT** currently using the `LocalCausalAgent` (LCA) trait from `koru-lambda-core`. It uses the `DistinctionEngine` directly for content-addressing but bypasses the LCA pattern entirely.

**Impact:** Moderate. Koru-Delta functions correctly as a standalone causal database but cannot participate as a "Delta Agent" in the ALIS AI architecture without refactoring.

**Refactoring Effort:** Medium (approximately 2-3 weeks of focused development).

---

## Part 1: Current LCA Usage Assessment

### 1.1 What is LocalCausalAgent?

The `LocalCausalAgent` trait in `koru-lambda-core` (available v1.1.0+) enforces a strict pattern for all subsystems:

```rust
pub trait LocalCausalAgent {
    type ActionData: Canonicalizable;
    
    /// Current causal root (local perspective)
    fn get_current_root(&self) -> &Distinction;
    
    /// Perform action via synthesis: ΔNew = ΔLocal_Root ⊕ ΔAction_Data
    fn synthesize_action(
        &mut self,
        action_data: Self::ActionData,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction;
    
    /// Advance causal state
    fn update_local_root(&mut self, new_root: Distinction);
}
```

This pattern ensures:
- **Locality**: All state changes originate from a local root distinction
- **Causality**: Every state transition is a synthesis operation
- **Determinism**: All action data must be `Canonicalizable`
- **Unified Field**: All agents share ONE `DistinctionEngine` (the "Koru Field")

### 1.2 Current Koru-Delta Architecture

Koru-Delta's current architecture:

```
┌─────────────────────────────────────────┐
│         KoruDelta Public API            │  ← No LCA implementation
│    (put, get, history, get_at)          │
├─────────────────────────────────────────┤
│      Vector Search Layer                │  ← Uses engine for hashing only
├─────────────────────────────────────────┤
│       Auth Layer                        │  ← Independent
├─────────────────────────────────────────┤
│      Reconciliation Layer               │  ← Independent
├─────────────────────────────────────────┤
│      Evolutionary Processes             │  ← Independent
├─────────────────────────────────────────┤
│       Memory Tiering                    │  ← Independent
├─────────────────────────────────────────┤
│        Causal Storage Layer             │  ← Uses engine directly
├─────────────────────────────────────────┤
│      Distinction Engine (core)          │  ← Direct usage
└─────────────────────────────────────────┘
```

### 1.3 How Koru-Delta Currently Uses koru-lambda-core

**Current Usage (in `src/mapper.rs`):**

```rust
pub fn json_to_distinction(
    value: &JsonValue,
    engine: &DistinctionEngine,
) -> DeltaResult<Distinction> {
    // Serialize to canonical JSON bytes
    let bytes = serde_json::to_vec(value).map_err(DeltaError::SerializationError)?;
    
    // Map bytes to distinction structure
    Ok(Self::bytes_to_distinction(&bytes, engine))
}

pub fn bytes_to_distinction(bytes: &[u8], engine: &DistinctionEngine) -> Distinction {
    if bytes.is_empty() {
        return engine.d0().clone();
    }
    
    // Convert each byte to a distinction and fold
    bytes
        .iter()
        .map(|&byte| byte.to_canonical_structure(engine))
        .fold(engine.d0().clone(), |acc, d| engine.synthesize(&acc, &d))
}
```

**Key Observations:**
1. **Direct engine access**: Uses `engine.synthesize()` but NOT through the LCA pattern
2. **No local root**: Each operation starts from `d0` (void), not a persistent local root
3. **No action canonicalization**: Operations are not modeled as `ActionData` types
4. **Independent instances**: Each `KoruDelta` creates its own `DistinctionEngine` via `Arc`

### 1.4 Missing LCA Components

| Component | Current State | LCA Requirement |
|-----------|---------------|-----------------|
| `KoruDelta` | Direct `DistinctionEngine` usage | Must implement `LocalCausalAgent` |
| `CausalStorage` | Direct engine access | Must have local root + action synthesis |
| `Workspace` | Independent container | Should be LCA with memory actions |
| `ConsolidationProcess` | Background task | Should be synthesis-based |
| `DistillationProcess` | Background task | Should be synthesis-based |
| `GenomeUpdateProcess` | Background task | Should be synthesis-based |

---

## Part 2: Refactoring Requirements for ALIS AI

### 2.1 Architectural Gap Analysis

**ALIS AI expects:**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           THE KORU FIELD                                    │
│                    (Shared DistinctionEngine Arc)                          │
│                                                                             │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│   │   PULSE      │  │ PERCEPTION   │  │ EXPRESSION   │  │    DELTA     │   │
│   │   AGENT      │  │    AGENT     │  │    AGENT     │  │    AGENT     │   │
│   ├──────────────┤  ├──────────────┤  ├──────────────┤  ├──────────────┤   │
│   │ Root: BEAT   │  │ Root: SENSE  │  │ Root: SPEAK  │  │ Root: MEMORY │   │
│   │ LCA: ✓       │  │ LCA: ✓       │  │ LCA: ✓       │  │ LCA: ?       │   │
│   └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Current Koru-Delta as "Delta Agent":**
- ❌ No `LocalCausalAgent` implementation
- ❌ No shared engine (creates its own)
- ❌ No action-based state transitions
- ❌ No local root persistence

### 2.2 Required Refactoring

#### Phase 1: Core LCA Implementation (1 week)

**1. Add LCA trait implementation to `KoruDelta`:**

```rust
// New: src/lca_adapter.rs
use koru_lambda_core::{LocalCausalAgent, Canonicalizable, Distinction};

pub enum DeltaAction {
    Store { namespace: String, key: String, value: JsonValue },
    Retrieve { namespace: String, key: String },
    Consolidate { target_tier: MemoryTier },
    Distill { fitness_threshold: f32 },
    Sync { peer: PeerId },
}

impl Canonicalizable for DeltaAction {
    fn to_canonical_structure(&self, engine: &DistinctionEngine) -> Distinction {
        // Convert action to canonical distinction
        match self {
            DeltaAction::Store { namespace, key, value } => {
                let ns_d = namespace.to_canonical_structure(engine);
                let key_d = key.to_canonical_structure(engine);
                let val_d = DocumentMapper::json_to_distinction(value, engine).unwrap();
                engine.synthesize(&engine.synthesize(&ns_d, &key_d), &val_d)
            }
            // ... other variants
        }
    }
}

impl LocalCausalAgent for KoruDelta {
    type ActionData = DeltaAction;
    
    fn get_current_root(&self) -> &Distinction {
        &self.local_root
    }
    
    fn synthesize_action(
        &mut self,
        action_data: Self::ActionData,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        let action_d = action_data.to_canonical_structure(engine);
        let new_root = engine.synthesize(&self.local_root, &action_d);
        self.update_local_root(new_root.clone());
        new_root
    }
    
    fn update_local_root(&mut self, new_root: Distinction) {
        self.local_root = new_root;
    }
}
```

**2. Modify `KoruDelta` struct:**

```rust
pub struct KoruDeltaGeneric<R: Runtime> {
    // ... existing fields ...
    
    /// Local causal root (NEW)
    local_root: Distinction,
    
    /// Shared engine reference (must be shared with ALIS field)
    engine: Arc<DistinctionEngine>,
}
```

**3. Update construction to accept shared engine:**

```rust
impl<R: Runtime> KoruDeltaGeneric<R> {
    /// Create with shared engine (for ALIS integration)
    pub fn with_shared_engine(
        engine: Arc<DistinctionEngine>,
        local_root: Distinction,
        runtime: R,
    ) -> Self {
        // ... initialization with shared engine
    }
}
```

#### Phase 2: Action-Based API (1 week)

**Replace direct storage operations with action synthesis:**

```rust
// OLD API (direct storage)
pub async fn put<T: Serialize>(
    &self,
    namespace: impl Into<String>,
    key: impl Into<String>,
    value: T,
) -> DeltaResult<VersionedValue> {
    // Direct storage manipulation
}

// NEW API (action synthesis)
pub async fn store(
    &mut self,  // Note: &mut self required for LCA
    namespace: impl Into<String>,
    key: impl Into<String>,
    value: impl Serialize,
) -> DeltaResult<(Distinction, VersionedValue)> {
    let action = DeltaAction::Store {
        namespace: namespace.into(),
        key: key.into(),
        value: serde_json::to_value(value)?,
    };
    
    // Perform synthesis via LCA
    let new_root = self.synthesize_action(action, &self.engine);
    
    // Also store in CausalStorage for retrieval
    let versioned = self.storage.put(&namespace, &key, value)?;
    
    Ok((new_root, versioned))
}
```

#### Phase 3: Background Processes as LCAs (3-4 days)

**Convert evolutionary processes to implement LCA:**

```rust
// src/processes/consolidation.rs
pub struct ConsolidationProcess {
    local_root: Distinction,
    config: ConsolidationConfig,
}

impl LocalCausalAgent for ConsolidationProcess {
    type ActionData = ConsolidationAction;
    
    fn synthesize_action(
        &mut self,
        action_data: ConsolidationAction,
        engine: &Arc<DistinctionEngine>,
    ) -> Distinction {
        match action_data {
            ConsolidationAction::Consolidate { from_tier, to_tier } => {
                // Perform consolidation
                // Return new root via synthesis
            }
        }
    }
    // ...
}
```

#### Phase 4: ALIS Bridge Integration (2-3 days)

**Add Bridge Agent compatibility:**

```rust
// New module: src/alis_bridge.rs

/// Bridge agent for ALIS integration
pub struct AlisBridge {
    delta_agent: KoruDelta,
}

impl AlisBridge {
    /// Synchronize with other ALIS agents during Consolidation phase
    pub async fn synchronize(&mut self, pulse_phase: Phase) -> DeltaResult<Distinction> {
        match pulse_phase {
            Phase::Consolidation => {
                // Trigger Delta agent consolidation
                let action = DeltaAction::Consolidate { 
                    target_tier: MemoryTier::Cold 
                };
                Ok(self.delta_agent.synthesize_action(action, &self.delta_agent.engine()))
            }
            _ => Ok(self.delta_agent.get_current_root().clone())
        }
    }
}
```

### 2.3 Refactoring Effort Estimate

| Task | Effort | Complexity |
|------|--------|------------|
| Add LCA trait to `KoruDelta` | 2 days | Medium |
| Add action types (`DeltaAction`) | 1 day | Low |
| Refactor `put`/`get` APIs | 2 days | Medium |
| Convert processes to LCAs | 2 days | Medium |
| Add ALIS bridge module | 1 day | Low |
| Update tests | 2 days | Medium |
| Documentation | 1 day | Low |
| **TOTAL** | **~11 days** | **Medium** |

---

## Part 3: New Features for General Usability

To fit within ALIS AI without making Koru-Delta solely dedicated to it, the following features should be added:

### 3.1 Feature Flags for ALIS Integration

```toml
# Cargo.toml
[features]
default = ["standalone"]
standalone = []  # Current behavior (independent engine)
alis-integration = ["shared-engine"]  # LCA mode with shared engine
shared-engine = []  # Enable shared DistinctionEngine
```

### 3.2 Optional LCA Mode

```rust
// src/core.rs
pub struct KoruDeltaGeneric<R: Runtime> {
    // ... existing fields ...
    
    /// LCA mode (optional)
    #[cfg(feature = "alis-integration")]
    lca_state: Option<LcaState>,
}

#[cfg(feature = "alis-integration")]
struct LcaState {
    local_root: Distinction,
    action_log: Vec<Distinction>,
}

impl<R: Runtime> KoruDeltaGeneric<R> {
    /// Enable LCA mode (for ALIS integration)
    #[cfg(feature = "alis-integration")]
    pub fn enable_lca_mode(&mut self, local_root: Distinction) {
        self.lca_state = Some(LcaState {
            local_root,
            action_log: Vec::new(),
        });
    }
    
    /// Check if running in LCA mode
    #[cfg(feature = "alis-integration")]
    pub fn is_lca_mode(&self) -> bool {
        self.lca_state.is_some()
    }
}
```

### 3.3 New Features Required

| Feature | Purpose | ALIS Relevance | General Use |
|---------|---------|----------------|-------------|
| **Action Tagging** | Tag operations with provenance | Essential for traceability | Useful for audit logs |
| **Background Synthesis** | Async consolidation as synthesis | Required for Dream phase | Powers automated maintenance |
| **Query as Synthesis** | Model queries as actions | Enables reactive synthesis | More powerful query model |
| **Cross-Workspace Sync** | Synchronize between workspaces | Required for Bridge agent | Enables distributed DB |
| **Causal Notifications** | Notify on causal state change | Required for agent coordination | Real-time subscriptions |

### 3.4 API Additions

```rust
impl KoruDelta {
    /// Store with action tagging (useful for ALIS, generally useful)
    pub async fn store_tagged(
        &mut self,
        namespace: &str,
        key: &str,
        value: impl Serialize,
        tags: Vec<String>,  // "user_input", "derived", "consolidated"
    ) -> DeltaResult<VersionedValue>;
    
    /// Query with synthesis-based filtering (powerful for all users)
    pub async fn query_synthesis(
        &self,
        pattern: &Distinction,  // Query as distinction
    ) -> DeltaResult<Vec<VersionedValue>>;
    
    /// Subscribe to causal changes (real-time for all)
    pub async fn subscribe_causal(
        &self,
        callback: impl Fn(CausalEvent) + Send + Sync,
    ) -> SubscriptionId;
}
```

---

## Part 4: Recommendations

### 4.1 Immediate Actions (Next Sprint)

1. **Add LCA feature flag** - Enable conditional compilation
2. **Implement `LocalCausalAgent` for `KoruDelta`** - Core trait implementation
3. **Add action types** - Define `DeltaAction` enum
4. **Create ALIS bridge module** - Minimal integration point

### 4.2 Medium-Term (Next Month)

1. **Refactor processes to LCAs** - Background tasks as agents
2. **Add shared engine support** - Enable ALIS field participation
3. **Implement causal notifications** - Real-time agent coordination
4. **Cross-workspace sync** - Distributed capability

### 4.3 Long-Term (Next Quarter)

1. **Full ALIS agent compatibility** - Complete Delta Agent specification
2. **Synthesis-based query engine** - Replace filter-based with distinction-based
3. **Dream phase support** - Background synthesis during idle

### 4.4 Backward Compatibility

**Strategy:** Maintain 100% backward compatibility via feature flags.

```rust
// Default behavior (unchanged)
let db = KoruDelta::start().await?;

// ALIS mode (opt-in)
#[cfg(feature = "alis-integration")]
let db = KoruDelta::with_shared_engine(shared_engine, local_root).await?;
```

---

## Part 5: Conclusion

### Summary

| Aspect | Status | Notes |
|--------|--------|-------|
| LCA Implementation | ❌ Missing | Direct engine usage instead |
| ALIS Compatibility | ⚠️ Partial | Can be used as storage, not as agent |
| Refactoring Effort | 📊 Medium | ~2-3 weeks focused work |
| General Usability | ✅ Good | Can remain standalone database |

### Key Finding

Koru-Delta is a **causal database** but not yet a **causal agent**. The distinction:
- **Causal Database**: Stores data with causal history
- **Causal Agent**: All operations are synthesis from a local root

For ALIS AI, Koru-Delta needs to become a **Delta Agent** - a causal agent specialized for memory consciousness.

### Final Recommendation

**Proceed with refactoring.** The effort is manageable (~2-3 weeks), the benefits for ALIS AI are substantial, and the general usability improvements (action tagging, causal notifications, synthesis queries) benefit all users - not just ALIS integrators.

---

*Assessment prepared for ALIS AI architecture integration planning.*
