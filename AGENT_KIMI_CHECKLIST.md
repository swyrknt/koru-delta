# KoruDelta: Agent Execution Checklist

**Document Purpose:** Track progress toward AI-ready v2.5.0  
**Current Version:** 2.0.0 (production-ready causal database)  
**Target Version:** 2.5.0 (AI infrastructure ready)  
**Last Updated:** 2026-02-06  
**Owner:** Agent Kimi

---

## THE VISION

**KoruDelta becomes "The Database for AI Agents"**

**Why this is PERFECT:**
| KoruDelta Feature | AI Agent Use Case |
|-------------------|-------------------|
| Hot/Warm/Cold/Deep memory | Agent memory with natural forgetting |
| Automatic versioning | Every decision/thought preserved |
| Causal tracking | Explainable AI (how did agent decide?) |
| Time travel | Agents "recall" past contexts |
| Edge deployment (8MB) | On-device AI (phones, robots, IoT) |
| Zero config | Agents spin up storage dynamically |

**The Gap:** We're a general causal database. Add vector search + AI-native APIs = revolutionary.

---

## REALISTIC TIMELINES (AI Agent Speed)

**Tonight (4-6 hours):**
- Vector storage module (f32 arrays + cosine similarity)
- Basic Python bindings (PyO3, core put/get/history)
- Agent memory layer (episodic/semantic wrapper)

**This Week (3-5 days):**
- Full Python package (pip installable)
- Vector ANN search (naive HNSW or flat index)
- RAG utilities (chunking, embedding interface)
- AI examples + documentation

**Next Week (7-10 days):**
- JS/TS bindings
- Web playground with AI demos
- v2.5.0 release

**Total: ~2 weeks to AI-ready**

---

## CHECKLIST: Current State (v2.0.0)

### ✅ FOUNDATION (Complete - Do Not Touch)
- [x] Core causal database with versioning
- [x] Hot/Warm/Cold/Deep memory tiers
- [x] WAL persistence with crash recovery
- [x] 400ns reads, 50µs writes
- [x] 8MB binary (edge-ready)
- [x] 321 tests passing
- [x] Multi-node clustering
- [x] CLI with auth commands
- [x] Query engine (filters, sort, aggregate)

---

## CHECKLIST: Tonight's Sprint (Hours 1-6)

### Hour 1-2: Vector Storage Module ✅ COMPLETE
- [x] Create `src/vector/` module
- [x] Define `Vector` type (Arc<[f32]> for memory efficiency)
- [x] Cosine similarity implementation (cached magnitude)
- [x] Euclidean distance implementation
- [x] Dot product, PartialEq, Hash, Display
- [x] Flat index (brute-force ANN) with namespace support
- [x] Thread-safe `VectorIndex` wrapper
- [x] Unit tests (24 vector tests, all passing)
- [x] Zero warnings, clippy clean

**Files created:**
- `src/vector/mod.rs` - Module exports + serialization helpers
- `src/vector/types.rs` - Vector type + math operations
- `src/vector/index.rs` - Flat ANN index + trait

**Lines of code:** 977 lines  
**Tests added:** 24 vector-specific tests  
**Total tests:** 345 passing

**Key API:**
```rust
pub struct Vector {
    data: Arc<[f32]>,  // Memory-efficient shared storage
    model: String,
}

impl Vector {
    pub fn cosine_similarity(&self, other: &Vector) -> Option<f32>
    pub fn euclidean_distance(&self, other: &Vector) -> Option<f32>
    pub fn dot_product(&self, other: &Vector) -> Option<f32>
}

pub struct VectorIndex { ... }
impl VectorIndex {
    pub fn add(&self, key: FullKey, vector: Vector)
    pub fn search(&self, query: &Vector, opts: &VectorSearchOptions) -> Vec<VectorSearchResult>
}
```

### Hour 2-3: Vector Search API ✅ COMPLETE
- [x] `KoruDelta::embed()` - store vector with metadata + versioning
- [x] `KoruDelta::embed_search()` - similarity search with namespace filtering
- [x] `KoruDelta::get_embed()` - retrieve stored vector
- [x] `KoruDelta::delete_embed()` - remove from index (append-only storage)
- [x] Integration tests (11 tests, all passing)
- [x] Vector index added to KoruDelta struct
- [x] Thread-safe via VectorIndex (Arc internally)

**Files modified:**
- `src/core.rs` - Added vector_index field + 4 vector methods

**API Usage:**
```rust
// Store embedding
let v = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
db.embed("docs", "doc1", v, Some(json!({"title": "AI"}))).await?;

// Search
let query = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
let results = db.embed_search(Some("docs"), &query, 
    VectorSearchOptions::new().top_k(5).threshold(0.8)
).await?;
```

### Hour 3-4: Workspace Layer ✅ COMPLETE (REFACTORED)
- [x] Create `src/memory/workspace.rs` module (GENERAL - not AI-specific)
- [x] `Workspace` struct - general purpose causal storage container
- [x] Memory patterns: Event, Reference, Procedure (conventions, not hardcoded)
- [x] `AgentContext` thin wrapper for AI-specific convenience methods
- [x] Relevance scoring (importance + recency + frequency + semantic)
- [x] Natural consolidation (30-day threshold for old items)
- [x] Thread-safe with RwLock
- [x] 0 warnings, clippy clean
- [x] Deleted old `agent.rs` - clean replacement

**Refactor Summary:**
- **Before:** AgentMemory (AI-specific, too narrow)
- **After:** Workspace (general) + AgentContext (AI wrapper)

**New API - General Purpose:**
```rust
// Any application can use workspaces
let audit = db.workspace("audit-2026");
audit.store("tx-123", data, MemoryPattern::Event).await?;
let history = audit.history("tx-123").await?;

// AI agents use the same workspace
let agent = db.workspace("agent-42").ai_context();
agent.remember_episode("User asked about Python").await?;
```

**Why this is better:**
- **General:** Audit trails, config management, scientific data - all work
- **Flexible:** Patterns are conventions, not enforced types
- **Clean:** AI is a use case, not the whole identity
- **Aligned with koru-lambda-core:** Workspaces are distinctions

### Hour 4-5: Python Bindings (PyO3)
- [ ] Create `bindings/python/` directory
- [ ] `Cargo.toml` with pyo3 dependency
- [ ] Expose core API:
  - `KoruDelta.start()`
  - `put(namespace, key, value)`
  - `get(namespace, key)`
  - `history(namespace, key)`
  - `embed(namespace, key, vector)`
  - `embed_search(namespace, query_vector, k)`
- [ ] Async support via `pyo3-asyncio`

**Files:**
- `bindings/python/Cargo.toml`
- `bindings/python/src/lib.rs`
- `bindings/python/koru_delta/__init__.py`
- `bindings/python/setup.py`

### Hour 5-6: Integration & Tests
- [ ] Integration test: Python ↔ Rust roundtrip
- [ ] Test vector search accuracy
- [ ] Test agent memory recall
- [ ] Update `src/lib.rs` exports
- [ ] Build passes, tests pass

**Verify:**
```bash
cargo test --release  # All 321+ tests pass
cd bindings/python && python -m pytest  # Python tests pass
```

---

## CHECKLIST: This Week (Days 2-5)

### Day 2: Python Package Polish
- [ ] `pip install maturin` build setup
- [ ] Type stubs (`koru_delta.pyi`)
- [ ] NumPy integration (zero-copy array passing)
- [ ] Jupyter notebook example
- [ ] PyPI upload ready

**Deliverable:**
```bash
pip install koru-delta
python -c "from koru_delta import KoruDelta; print('OK')"
```

### Day 2-3: Vector ANN (Approximate Search)
- [ ] HNSW implementation OR use `hnsw` crate
- [ ] Index persistence (save/load)
- [ ] Batch embedding insert
- [ ] Benchmark: 1M vectors, <10ms query

**Key decision:** Use existing crate or implement?
- **Fast path:** Use `instant-distance` or `hnsw` crate
- **Clean path:** Implement simple HNSW (2-3 days)

### Day 3: RAG Utilities
- [ ] Document chunking (`src/rag/chunker.rs`)
  - Fixed size chunks
  - Sliding window
  - Semantic chunks (future)
- [ ] Embedding interface trait
- [ ] Hybrid search (vector + keyword)
- [ ] Source attribution

**Files:**
- `src/rag/mod.rs`
- `src/rag/chunker.rs`
- `src/rag/embedder.rs` (trait for OpenAI/local)

### Day 3-4: AI Examples
- [ ] `examples/ai_agent.rs` - Autonomous agent with memory
- [ ] `examples/rag_search.rs` - Document Q&A
- [ ] `examples/chatbot_memory.rs` - Conversational AI
- [ ] Python notebooks:
  - `notebooks/agent_memory.ipynb`
  - `notebooks/rag_pipeline.ipynb`

### Day 4-5: Documentation
- [ ] "Building AI Agents with KoruDelta" guide
- [ ] API reference for vector search
- [ ] "RAG Architecture Patterns"
- [ ] README update with AI focus
- [ ] Changelog for v2.5.0

---

## CHECKLIST: Next Week (Days 6-10)

### Day 6-7: JavaScript/TypeScript Bindings
- [ ] WASM build with `wasm-bindgen`
- [ ] Node.js native addon (neon or napi-rs)
- [ ] TypeScript definitions
- [ ] npm package

**Files:**
- `bindings/javascript/` directory
- `package.json`
- `index.d.ts`

### Day 7-8: Web Playground
- [ ] Simple HTML/JS frontend
- [ ] "Build an AI Agent" interactive demo
- [ ] "RAG over your documents" demo
- [ ] Deploy to GitHub Pages

**Features:**
- Upload documents
- Ask questions
- See agent memory in action
- Time travel visualization

### Day 8-9: Release Prep
- [ ] Version bump to 2.5.0
- [ ] Final test suite run
- [ ] Performance benchmarks
- [ ] Security audit (dependencies)
- [ ] Git tag + release notes

### Day 9-10: Launch
- [ ] GitHub release
- [ ] PyPI publish
- [ ] npm publish
- [ ] Hacker News post
- [ ] Reddit r/rust, r/MachineLearning
- [ ] Twitter/X announcement

---

## IMPLEMENTATION DETAILS

### Vector Storage Design

**Storage Strategy:**
```rust
// Vectors stored as regular values (versioned!)
db.put("embeddings", "doc1", json!({
    "vector": [0.1, 0.2, ...],
    "model": "text-embedding-3-small",
    "dimensions": 1536,
    "metadata": { ... }
}))
```

**Index Strategy:**
```rust
// In-memory index (DashMap for thread safety)
pub struct VectorIndex {
    // namespace -> (key -> Vector)
    vectors: DashMap<String, DashMap<String, Vector>>,
    // HNSW index per namespace (or flat for small datasets)
    indexes: DashMap<String, Box<dyn AnnIndex>>,
}
```

### Agent Memory Design

**Memory Structure:**
```rust
pub struct Memory {
    pub id: String,           // UUID
    pub content: String,      // Text content
    pub embedding: Vector,    // Semantic representation
    pub memory_type: MemoryType,  // Episodic, Semantic, Procedural
    pub importance: f32,      // 0.0 - 1.0
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
}

pub enum MemoryType {
    Episodic,   // Specific events ("User asked about Python")
    Semantic,   // Facts ("Python bindings exist")
    Procedural, // How-to ("To install: pip install koru-delta")
}
```

**Recall Algorithm:**
1. Embed query
2. Find similar memories (vector search)
3. Score by: similarity × importance × recency
4. Return top-k

**Consolidation Algorithm:**
1. Find old memories (>30 days)
2. Group similar memories
3. Summarize with LLM (user-provided callback)
4. Store summary, archive originals

### Python API Design

```python
import asyncio
from koru_delta import KoruDelta

async def main():
    # Start database
    db = await KoruDelta.start()
    
    # Regular operations
    await db.put("users", "alice", {"name": "Alice"})
    user = await db.get("users", "alice")
    
    # Vector operations
    await db.embed("documents", "doc1", 
        vector=[0.1, 0.2, ...],
        model="text-embedding-3-small"
    )
    
    results = await db.embed_search(
        "documents",
        query_vector=[0.1, 0.2, ...],
        k=5
    )
    
    # Agent memory
    memory = db.agent_memory("agent-42")
    await memory.remember_episode("User asked about Python bindings")
    relevant = await memory.recall("What did user ask about?", limit=3)

asyncio.run(main())
```

---

## DEPENDENCIES TO ADD

### For Vector Search
```toml
# Cargo.toml additions
[dependencies]
# Vector math
ndarray = "0.15"

# ANN (choose one)
# instant-distance = "0.7"  # Fast HNSW
# hnsw = "0.11"             # Alternative
# OR implement ourselves (cleaner, no deps)
```

### For Python Bindings
```toml
# bindings/python/Cargo.toml
[dependencies]
pyo3 = { version = "0.22", features = ["extension-module"] }
pyo3-asyncio = { version = "0.22", features = ["tokio-runtime"] }
numpy = "0.22"  # For zero-copy array passing
```

---

## FILE STRUCTURE CHANGES

```
src/
  lib.rs              # Add: pub mod vector; pub mod memory;
  vector/
    mod.rs            # Public API
    types.rs          # Vector struct
    index.rs          # ANN index trait + impls
    metrics.rs        # Cosine, Euclidean
  memory/
    mod.rs            # Existing memory tiers
    agent.rs          # NEW: AgentMemory API
    types.rs          # Memory, MemoryType
  rag/
    mod.rs            # Public API
    chunker.rs        # Document chunking
    embedder.rs       # Embedding trait
bindings/
  python/
    Cargo.toml
    src/lib.rs
    koru_delta/
      __init__.py
    setup.py
  javascript/
    Cargo.toml
    src/lib.rs
    package.json
    index.d.ts
examples/
  ai_agent.rs
  rag_search.rs
  chatbot_memory.rs
notebooks/
  agent_memory.ipynb
  rag_pipeline.ipynb
```

---

## SUCCESS CRITERIA

### Tonight (After 6 hours)
- [ ] `cargo test` passes (321+ tests)
- [ ] `python -c "from koru_delta import KoruDelta"` works
- [ ] Vector cosine similarity correct (tested)
- [ ] Agent memory recall works end-to-end

### This Week
- [ ] `pip install koru-delta` works
- [ ] 1M vector search < 10ms
- [ ] RAG example runs end-to-end
- [ ] 3 AI examples complete

### Next Week
- [ ] `npm install koru-delta` works
- [ ] Web playground live
- [ ] v2.5.0 tagged and released
- [ ] GitHub stars: +500

---

## RISK MITIGATION

### Risk: PyO3 build issues
**Mitigation:** Test build early (Hour 4). Use `maturin` for packaging.

### Risk: ANN performance
**Mitigation:** Start with flat index. Optimize later. 10ms for 100K is fine for MVP.

### Risk: Scope creep
**Mitigation:** Stick to checklist. No "just one more feature."

### Risk: WASM compatibility
**Mitigation:** Feature-gate vector search. Core works on WASM, AI features don't need to.

---

## DECISION LOG

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-06 | Flat index first, HNSW later | 10ms for 100K is fine MVP |
| 2026-02-06 | Python before JS | AI market is Python-first |
| 2026-02-06 | Store vectors as JSON | Leverages existing versioning |
| 2026-02-06 | No embedding generation | Use OpenAI/Anthropic, we store |

---

## DAILY AGENT CHECKLIST

### Before Starting Work
```bash
# 1. Verify clean state
git status  # Should be clean
cargo test --release  # Should pass

# 2. Pick next unchecked item from this checklist
# 3. Time-box: Set timer for estimated duration
```

### During Work
- [ ] Writing tests as I go (not after)
- [ ] Committing every 30-60 minutes
- [ ] Running `cargo clippy` regularly
- [ ] Updating this checklist as items complete

### Before Finishing
```bash
# 1. Final verification
cargo test --release
cargo fmt --check
cargo clippy -- -D warnings

# 2. Update checklist
# Mark items complete, add notes

# 3. Commit
git add .
git commit -m "feat: [what was done]"
```

---

## THE NORTH STAR

**Every commit should answer:**
> "Does this make KoruDelta the obvious choice for AI agent developers?"

**If yes:** Perfect.  
**If no:** Why are we doing it?  
**If maybe:** Defer.

---

## CURRENT STATUS

**Last Updated:** 2026-02-06  
**Next Action:** Hour 4-5 - Python Bindings (PyO3)  
**ETA v2.5.0:** ~8 days (ahead of schedule)

**Progress:**
- [x] Hour 1-2: Vector storage (977 lines, 24 tests)
- [x] Hour 2-3: Vector search API (KoruDelta integration, 11 tests)
- [x] Hour 3-4: Agent memory (996 lines, 11 tests)
- [ ] Hour 4-5: Python bindings
- [ ] Hour 5-6: Integration & docs
- [ ] Day 2: Python package polish
- [ ] Day 2-3: ANN search optimization
- [ ] Day 3: RAG utilities
- [ ] Day 3-4: AI examples
- [ ] Day 4-5: Documentation
- [ ] Day 6-7: JS bindings
- [ ] Day 7-8: Web playground
- [ ] Day 8-9: Release prep
- [ ] Day 9-10: Launch

**Stats:**
- **Total tests:** 367 passing
- **Code added:** ~2,000 lines
- **Warnings:** 0
- **Commits:** 2
- **Status:** On track, no blockers

---

**Remember:** We're not building a better PostgreSQL. We're building the database that AI agents dream of. Move fast, ship often, make it magical.
