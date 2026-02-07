# KoruDelta Core Identity: What Are We Really Building?

## The Heart and Soul

**KoruDelta is a CAUSAL DATABASE** - every piece of data knows:
- Where it came from (provenance)
- When it existed (time)
- How it relates to other data (distinctions)
- When to forget (natural memory lifecycle)

The AI agent stuff is just ONE application of this. The core is much deeper.

---

## The Core Capabilities (Use-Case Agnostic)

### 1. Content-Addressed Storage (via koru-lambda-core)
- Same data = same identity
- Automatic deduplication
- Structural integrity via distinction calculus

### 2. Temporal Versioning
- Every state preserved
- Time travel queries
- Audit trail for everything

### 3. Memory Lifecycle Management
- Hot → Warm → Cold → Deep
- Natural forgetting
- Automatic consolidation

### 4. Edge-Native
- 8MB binary
- No external dependencies
- Runs on anything

---

## Current Positioning Problem

**Too narrow:** "Database for AI Agents"  
**Better:** "The Causal Database" with AI agents as a primary use case

### The Risk of "Agent Memory"
- Sounds like a toy/feature, not a real database
- Scares away non-AI users who need audit trails
- Misses the bigger picture: causal tracking is valuable everywhere

---

## What Should AgentMemory Really Be?

**Current:** `db.agent_memory("agent-42")` - hardcoded AI semantics  
**Better:** Generic "Memory Spaces" that AI agents use

```rust
// Generic: Any application can use this
let workspace = db.workspace("project-alpha");
let personal = db.workspace("user-alice");
let agent = db.workspace("agent-42");  // AI agents just use it too

// All get:
// - Versioned storage
// - Semantic search (if embeddings added)
// - Natural lifecycle
// - Isolation from other workspaces
```

The episodic/semantic/procedural types are just CONVENTIONS, not requirements.

---

## Refactoring Strategy

### 1. Rename AgentMemory → Workspace

```rust
pub struct Workspace {
    db: KoruDelta,
    name: String,  // Was agent_id
}

impl Workspace {
    // Generic operations
    pub async fn remember(&self, key: &str, content: Value, importance: f32) -> Result<()>;
    pub async fn recall(&self, query: &str) -> Vec<RecallResult>;
    pub async fn at_time(&self, key: &str, timestamp: DateTime) -> Option<Value>;
    pub async fn history(&self, key: &str) -> Vec<Version>;
    pub async fn consolidate(&self) -> ConsolidationResult;
    
    // AI-specific helpers (optional conveniences)
    pub fn for_agent(self) -> AgentContext { ... }
}

// AI-specific wrapper (thin layer)
pub struct AgentContext {
    workspace: Workspace,
}

impl AgentContext {
    pub async fn remember_episode(&self, ...) { self.workspace.remember(...).await }
    pub async fn remember_fact(&self, ...) { self.workspace.remember(...).await }
    // etc.
}
```

### 2. Reframe the Three Memory Types

**Current (AI-specific):**
- Episodic: Events
- Semantic: Facts  
- Procedural: How-to

**Better (Generic with AI examples):**

| Type | Generic Meaning | AI Use | Other Uses |
|------|----------------|--------|-----------|
| Episodic | Event log | Agent experiences | Audit trail, metrics, logs |
| Semantic | Reference data | Agent knowledge | Config, user profiles, taxonomy |
| Procedural | Computable knowledge | Agent skills | Workflows, business rules, formulas |

### 3. Keep Vector Search General

**Current:** `db.embed()` for AI embeddings  
**Better:** `db.index_vector()` for ANY vector data

```rust
// AI use
await workspace.put("doc1", {"text": "..."}, embedding: vec);

// Scientific use (same underlying mechanism)
await workspace.put("sensor-reading", {"temp": 25.3}, embedding: fourier_features);

// Search works the same for both
await workspace.similar(query_vector, top_k: 10);
```

### 4. Highlight Non-AI Use Cases

**Audit & Compliance:**
```rust
// Every financial transaction preserved
let audit = db.workspace("audit-2026");
audit.remember("tx-12345", tx_data, importance: 1.0).await;

// Regulator asks: "What did you know at time X?"
let state = audit.at_time("tx-12345", timestamp).await;
```

**Local-First Apps:**
```rust
// Offline-capable, syncs when online
let local = db.workspace("offline-cache");
local.remember("article-1", article_data).await;
// Automatic sync via reconciliation
```

**Scientific Data:**
```rust
// Full provenance for reproducibility
let experiment = db.workspace("exp-2026-02");
experiment.remember("run-1", {
    "params": {...},
    "results": {...}
}, importance: 0.9).await;

// See how results evolved
let history = experiment.history("run-1").await;
```

**Edge/IoT:**
```rust
// Limited resources, need smart caching
let device = db.workspace("sensor-node-42");
device.remember("reading", data, importance: calculate_importance(data)).await;
// Old data automatically consolidates to save space
```

---

## New Positioning

**Old:** "The database for AI agents"  
**New:** "The Causal Database: Versioned, edge-native storage with natural memory lifecycle"

**Tagline options:**
1. "Git for your data"
2. "The database that remembers"
3. "Causal storage for the edge era"
4. "Time-travel database with natural forgetting"

**Description:**
> KoruDelta is a zero-config, causal database that gives every piece of data a complete history. Built on distinction calculus, it provides automatic versioning, time-travel queries, and natural memory lifecycle management (hot → warm → cold → deep). Runs on edge devices, embeds in applications, and tracks provenance for audit trails.
>
> Perfect for: AI agents needing memory, audit-heavy applications, local-first software, edge computing, and any system where understanding "how did we get here?" matters.

---

## Architecture Refactor Plan

### Phase 1: Rename and Generalize
1. `AgentMemory` → `Workspace`
2. Keep `AgentContext` as thin wrapper
3. Update all docs to use generic language

### Phase 2: Feature Rebalancing
1. Highlight non-AI examples equally
2. Add examples: audit trail, scientific data, config management
3. Keep AI examples but as "one powerful use case"

### Phase 3: Ecosystem
1. Build integrations for general use (not just AI)
   - Log aggregation
   - Config management  
   - Time-series with versioning
   - Document management

---

## The koru-lambda-core Connection

**The deeper identity:** KoruDelta makes distinction calculus practical.

From the math:
- Distinctions create structure
- Synthesis combines distinctions  
- Everything is content-addressed
- Relationships are primary

To the user:
- Everything is versioned automatically
- Same data is stored once (deduplication)
- You can see how data relates and evolves
- The system "forgets" naturally like human memory

**The positioning:** "Practical distinction calculus for application developers"

---

## My Recommendation

**YES, refactor. But keep the AI features.**

1. **Rename AgentMemory to Workspace** - makes it clear it's general-purpose
2. **Keep episodic/semantic/procedural as "memory patterns"** - not hardcoded types
3. **Lead with "Causal Database"** - AI is a use case, not the whole identity
4. **Build examples across domains:**
   - AI agents (current)
   - Financial audit (compliance)
   - Scientific reproducibility (provenance)
   - Config management (versioning)
   - Edge IoT (lifecycle)

5. **Keep the "natural memory" concept** - it's unique and applies everywhere:
   - Old configs should be archived
   - Stale sensor data should compress
   - Important audit events should be preserved
   - AI agent memories should consolidate

**The heart:** KoruDelta gives data a natural lifecycle like human memory, with complete provenance like Git, running at the edge like SQLite.

**The flexibility:** Workspaces let you organize this causal storage however you need - for AI agents, audit logs, scientific data, or anything else.

---

## Implementation

```rust
// New API design
pub struct Workspace {
    db: Arc<KoruDelta>,
    namespace: String,
}

impl Workspace {
    /// Store with automatic versioning
    pub async fn put(&self, key: &str, value: Value) -> Result<Version>;
    
    /// Get current value
    pub async fn get(&self, key: &str) -> Result<Value>;
    
    /// Get value at specific time
    pub async fn get_at(&self, key: &str, time: DateTime) -> Result<Value>;
    
    /// Get complete history
    pub async fn history(&self, key: &str) -> Vec<Version>;
    
    /// Semantic search (if vectors indexed)
    pub async fn search(&self, query: &str) -> Vec<SearchResult>;
    
    /// Natural consolidation of old data
    pub async fn consolidate(&self) -> ConsolidationReport;
}

// AI-specific convenience layer (thin)
pub mod ai {
    use super::Workspace;
    
    pub struct AgentMemory {
        workspace: Workspace,
    }
    
    impl AgentMemory {
        pub fn new(workspace: Workspace) -> Self { ... }
        
        // These are just conventions on top of Workspace
        pub async fn remember_episode(&self, content: &str) {
            self.workspace.put("episodes/uuid", json!({
                "type": "episodic",
                "content": content,
                "importance": 0.7,
            })).await
        }
        
        pub async fn recall(&self, query: &str) -> Vec<Memory> {
            self.workspace.search(query).await
        }
    }
}
```

This keeps the AI features but makes it clear they're built on a general-purpose causal foundation.
