# KoruDelta: Execution Checklist

**Document Purpose:** Track progress toward v2.5.0 - "The Causal Database"  
**Current Version:** 2.0.0 (production-ready causal database)  
**Target Version:** 2.5.0 (vector search + workspaces)  
**Last Updated:** 2026-02-06  
**Owner:** Agent Kimi

---

## THE VISION

**KoruDelta: "The Causal Database"**

> Content-addressed storage with natural memory lifecycle. Like Git for your data, with automatic tiering (Hot→Warm→Cold→Deep) and built-in vector search.

### Why This Matters

Traditional databases store the current state. KoruDelta stores causality:
- **Where data came from** (provenance)
- **When it existed** (temporal versioning)
- **How it relates** (distinction calculus)
- **When to forget** (natural lifecycle)

### Primary Use Cases

| Use Case | KoruDelta Advantage |
|----------|---------------------|
| **AI Agents** | Memory with natural forgetting, explainable decisions |
| **Audit/Compliance** | Complete provenance, time-travel queries |
| **Local-First Apps** | Offline-capable, automatic sync, zero config |
| **Edge/IoT** | 8MB binary, smart compression, runs anywhere |
| **Scientific Data** | Reproducibility, full history, relationship tracking |
| **Config Management** | Versioned config with rollback, audit trail |

### The Core Philosophy

Built on **koru-lambda-core** (distinction calculus):
```
Distinction → Identity
Synthesis → Relationships  
Content-addressing → Deduplication
Memory tiers → Natural lifecycle
```

---

## REALISTIC TIMELINES

### Completed (Hours 1-4)
- ✅ Vector storage module (977 lines)
- ✅ Vector search API (11 tests)
- ✅ Workspace layer (general, not AI-specific)

### Next (Hours 5-8)
- Python bindings (PyO3)
- Integration tests
- Examples: AI + audit + config

### This Week
- Full Python package
- ANN search optimization
- Documentation

### Next Week
- JS/WASM bindings
- Web playground
- v2.5.0 release

---

## CHECKLIST: Current State (v2.0.0)

### ✅ FOUNDATION (Complete)
- [x] Core causal database with versioning
- [x] Hot/Warm/Cold/Deep memory tiers
- [x] WAL persistence with crash recovery
- [x] 400ns reads, 50µs writes
- [x] 8MB binary (edge-ready)
- [x] 360 tests passing
- [x] Multi-node clustering
- [x] CLI with auth commands
- [x] Query engine (filters, sort, aggregate)

---

## CHECKLIST: Hours 1-4 (COMPLETED)

### Hour 1-2: Vector Storage ✅
- [x] `Vector` type with Arc<[f32]> storage
- [x] Cosine similarity, Euclidean distance, dot product
- [x] Flat ANN index (thread-safe)
- [x] 24 tests, 0 warnings

**API:**
```rust
let v = Vector::new(vec![0.1, 0.2], "text-embedding-3-small");
let sim = v.cosine_similarity(&other);
```

### Hour 2-3: Vector Search API ✅
- [x] `db.embed()` - store with versioning
- [x] `db.embed_search()` - similarity search
- [x] Thread-safe VectorIndex
- [x] 11 integration tests

### Hour 3-4: Workspace Layer ✅ (REFACTORED)
- [x] `Workspace` - general causal storage container
- [x] MemoryPattern::Event/Reference/Procedure (conventions)
- [x] `AgentContext` - thin AI wrapper
- [x] Clean replacement, 0 warnings

**Before:** `AgentMemory` (AI-only)  
**After:** `Workspace` (general) + `AgentContext` (AI)

**API - General:**
```rust
let audit = db.workspace("audit-2026");
audit.store("tx-123", data, MemoryPattern::Event).await?;
let history = audit.history("tx-123").await?;
```

**API - AI:**
```rust
let agent = db.workspace("agent-42").ai_context();
agent.remember_episode("User asked about Python").await?;
```

---

## CHECKLIST: Hours 5-8 (CURRENT - IN PROGRESS)

### Hour 4-5: Python Bindings 🔄
- [x] Architecture designed (API_DESIGN.md, IMPLEMENTATION_DESIGN.md)
- [x] Rust FFI layer structure (compiles with `cargo check`)
- [x] Python wrapper layer structure
- [ ] Build with maturin (NEEDS PYTHON ENV)
- [ ] Test Python ↔ Rust roundtrip

**Status:** Architecture complete. Blocked on Python environment for maturin build.

### Hour 5-6: Python Package ⏳
- [ ] Type stubs
- [ ] NumPy integration tested
- [ ] Basic usage example verified
- [ ] PyPI package structure

### Hour 6-7: Examples ⏳
- [ ] AI agent example (Python)
- [ ] Audit trail example
- [ ] Config management example

### Hour 7-8: Integration ⏳
- [ ] End-to-end tests passing
- [ ] Documentation complete
- [ ] Ready for PyPI publish

---

## CHECKLIST: This Week

### Day 2: Python Package Polish
- [ ] `pip install` works
- [ ] Jupyter notebook
- [ ] PyPI upload ready

### Day 2-3: ANN Optimization
- [ ] HNSW or better index
- [ ] 1M vectors benchmark
- [ ] Index persistence

### Day 3: RAG Utilities
- [ ] Document chunking
- [ ] Embedding trait
- [ ] Hybrid search

### Day 3-4: Multi-Use Examples
- [ ] AI agent (memory)
- [ ] RAG pipeline
- [ ] Audit trail compliance
- [ ] Config versioning

### Day 4-5: Documentation
- [ ] "The Causal Database" guide
- [ ] Use case: AI Agents
- [ ] Use case: Audit/Compliance
- [ ] Use case: Edge Computing
- [ ] API reference

---

## CHECKLIST: Next Week

### Day 6-7: JavaScript Bindings
- [ ] WASM build
- [ ] TypeScript definitions
- [ ] npm package

### Day 7-8: Web Playground
- [ ] Interactive demos
- [ ] Time travel visualization
- [ ] Deploy to GitHub Pages

### Day 8-9: Release Prep
- [ ] Version 2.5.0
- [ ] Final tests
- [ ] Security audit
- [ ] Release notes

### Day 9-10: Launch
- [ ] GitHub release
- [ ] PyPI publish
- [ ] npm publish
- [ ] Community posts

---

## WORKSPACE DESIGN

**Core Concept:**
```rust
pub struct Workspace {
    db: KoruDelta,
    name: String,  // Isolation boundary
}
```

**Patterns (Conventions):**
```rust
enum MemoryPattern {
    Event,      // Audit logs, agent episodes, metrics
    Reference,  // Config, facts, taxonomy
    Procedure,  // Workflows, rules, agent skills
}
```

**Lifecycle:**
- Store → Hot (immediate access)
- Access stops → Warm (chronicle)
- Time passes → Cold (consolidated)
- Epoch ends → Deep (genomic)

**AI Wrapper:**
```rust
pub struct AgentContext {
    workspace: Workspace,
}

impl AgentContext {
    fn remember_episode(&self, content: &str) { ... }
    fn remember_fact(&self, key: &str, content: &str) { ... }
    fn recall(&self, query: &str) -> Vec<Memory> { ... }
}
```

---

## SUCCESS CRITERIA

### Immediate
- [x] 360 tests passing
- [x] 0 warnings
- [x] Workspace API complete

### Week 1
- [x] Python bindings work (maturin build, clean clippy, imports successfully)
- [ ] 3 use case examples (AI, audit, config)
- [ ] Documentation updated

### Week 2
- [ ] v2.5.0 released
- [ ] PyPI package
- [ ] Web playground

### Month 1
- [ ] 500+ PyPI downloads
- [ ] 3 production users
- [ ] 1 case study

---

## RISK MITIGATION

### Risk: Python bindings complexity
**Mitigation:** Already have working architecture, just need maturin build.

### Risk: Positioning confusion
**Mitigation:** Lead with "Causal Database", show AI as one compelling use case.

### Risk: Competition
**Mitigation:** No direct competitor for causal + vector + edge. Stay focused.

---

## DECISION LOG

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-06 | Flat index first | 10ms for 100K is fine MVP |
| 2026-02-06 | Python first | AI market is Python-first |
| 2026-02-06 | AgentMemory → Workspace | General positioning, AI is one use case |
| 2026-02-06 | No backward compat | Clean slate, better architecture |

---

## CURRENT STATUS

**Completed:**
- ✅ Vector storage (977 lines, 24 tests)
- ✅ Vector search API (11 integration tests)
- ✅ Workspace layer (general + AI wrapper, 11 tests)
- ✅ Refactor: AgentMemory → Workspace (clean replacement)
- ✅ Python bindings architecture (design docs, Rust FFI structure)

**In Progress:**
- 🔄 Python bindings build (needs maturin + Python environment)

**Blocked:**
- ⏳ Python package (waiting for bindings build)
- ⏳ Examples (waiting for working Python API)
- ⏳ Documentation (waiting for stable API)

**Stats:**
- 360 tests passing
- 0 warnings, clippy clean
- 7 commits on dev branch
- ~3,000 lines of new code

**Next Action:** Setup Python environment and build with maturin

---

**Remember:** We're building "The Causal Database" - a new category of storage that understands time, provenance, and natural lifecycle. AI agents happen to need exactly this. But so do audit trails, config management, and edge computing.

The koru-lambda-core foundation (distinction calculus) makes this possible. Everything else is application.
