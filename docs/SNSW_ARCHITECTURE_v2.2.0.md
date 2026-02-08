# SNSW v2.2.0 Architecture Document

**Status**: Production Implementation Complete  
**Date**: 2026-02-08  
**Target Release**: koru-delta v2.2.0

---

## Executive Summary

SNSW (Synthesis-Navigable Small World) is a **distinction-based approximate nearest neighbor (ANN) search** system that treats vectors as semantic distinctions in a causal graph, rather than geometric points in space.

### What's Implemented ✅

| Feature | Status | Evidence |
|---------|--------|----------|
| Content-addressed identity (Blake3) | ✅ Complete | `ContentHash::from_vector()` deduplicates automatically |
| Synthesis edge types (6 types) | ✅ Complete | `SynthesisType` enum with relationship semantics |
| Synthesis proximity metric | ✅ Complete | `SynthesisProximity` combines geometric + semantic |
| Multi-layer abstraction structure | ✅ Complete | `abstraction_level` field, entry points per layer |
| Explainable search | ✅ Complete | `search_explainable()` returns `SynthesisExplanation` |
| Semantic navigation | ✅ Complete | `synthesis_navigate()` for concept composition |
| Production-grade search tiers | ✅ Complete | Hot→Warm-Fast→Warm-Thorough→Cold escalation |
| Adaptive threshold learning | ✅ Complete | Feedback loop adjusts confidence thresholds |
| Generation-based caching | ✅ Complete | Epoch tracking with lazy invalidation |
| 15 comprehensive tests | ✅ Complete | All passing, zero warnings |

### What's Still Research/Experimental 🔬

| Feature | Status | Blockers |
|---------|--------|----------|
| Deep koru-lambda-core integration | 🔬 Experimental | Need distinction engine API for semantic decomposition |
| Automatic abstraction detection | 🔬 Experimental | Requires clustering analysis, not yet implemented |
| Cross-modal synthesis (text+image+audio) | 🔬 Experimental | Future v2.3.0+ feature |
| SNSW vs HNSW benchmark (10K vectors) | 🔬 Pending | Need standard dataset + evaluation harness |
| Learned synthesis weights (MLP) | 🔬 Research | Needs training data from human judgments |

---

## Architecture

### Core Insight

**HNSW**: "These vectors are close in space" → `cosine_similarity(a, b) = 0.85`  
**SNSW**: "These vectors share distinctions and synthesis relationships" → `synthesis_proximity(a, b) = 0.92` because they share semantic properties, abstraction paths, and causal context

### Data Structures

```
┌─────────────────────────────────────────────────────────────────┐
│                    SNSW v2.2.0 Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ContentHash (Blake3 of vector data + model)                    │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────────────────────────────────────────────┐      │
│  │           SynthesisGraph                             │      │
│  │  ┌──────────────────────────────────────────────┐   │      │
│  │  │  Nodes: DashMap<ContentHash, SynthesisNode> │   │      │
│  │  │  ┌────────────────────────────────────────┐  │   │      │
│  │  │  │ SynthesisNode                          │  │   │      │
│  │  │  │  - id: ContentHash                     │  │   │      │
│  │  │  │  - vector: Arc<Vector>                 │  │   │      │
│  │  │  │  - synthesis_edges: Vec<SynthesisEdge> │  │   │      │
│  │  │  │  - abstraction_level: usize            │  │   │      │
│  │  │  │  - inserted_at: u64 (causal timestamp) │  │   │      │
│  │  │  │  - shared_distinctions: Vec<String>    │  │   │      │
│  │  │  └────────────────────────────────────────┘  │   │      │
│  │  └──────────────────────────────────────────────┘   │      │
│  │                                                     │      │
│  │  ┌──────────────────────────────────────────────┐   │      │
│  │  │  Abstraction Layers (v2.3.0)                │   │      │
│  │  │  Layer 2: Abstract concepts                 │   │      │
│  │  │  Layer 1: Categories                        │   │      │
│  │  │  Layer 0: Specific instances (base)         │   │      │
│  │  └──────────────────────────────────────────────┘   │      │
│  │                                                     │      │
│  │  ┌──────────────────────────────────────────────┐   │      │
│  │  │  Semantic Cache                              │   │      │
│  │  │  - Generation-based (epoch tracking)         │   │      │
│  │  │  - O(1) exact match only                     │   │      │
│  │  │  - Lazy invalidation on epoch change         │   │      │
│  │  └──────────────────────────────────────────────┘   │      │
│  └──────────────────────────────────────────────────────┘      │
│                                                                 │
│  SynthesisEdge:                                                 │
│    - target: ContentHash                                        │
│    - relationship: SynthesisType (Proximity|Composition|...)    │
│    - strength: f32 (weighted combination)                       │
│    - geometric_score: f32                                       │
│    - semantic_score: f32                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Synthesis Proximity Formula

```rust
/// SNSW v2.2.0 proximity metric
/// Combines geometric, semantic, and causal factors
pub struct SynthesisProximity {
    pub score: f32,           // Combined: 0.0 to 1.0
    pub geometric: f32,       // Cosine similarity
    pub semantic: f32,        // Distinction overlap
    pub causal: f32,          // Temporal/sequence proximity
    pub weights: ProximityWeights,  // Adjustable per-query
}

// Default weights (can be customized per query context)
weights = { geometric: 0.5, semantic: 0.35, causal: 0.15 }

score = w_geo * geometric + w_sem * semantic + w_cau * causal
```

### Search Algorithm: Escalating Strategy

```
┌────────────────────────────────────────────────────────────────┐
│                    Search Escalation                           │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Stage 1: 🔥 HOT (O(1))                                        │
│  ├─ Exact cache match only (Blake3 hash of query)             │
│  ├─ No near-hit scanning (avoids expensive false positives)   │
│  └─ Confidence: 1.0 (exact)                                    │
│                                                                │
│  Stage 2: 🌤️ WARM-FAST                                        │
│  ├─ Beam search with ef_fast (default 50)                     │
│  ├─ Synthesis-aware neighbor expansion                        │
│  ├─ Check confidence vs learned threshold                     │
│  └─ If confidence >= 0.90: return                             │
│                                                                │
│  Stage 3: 🌤️ WARM-THOROUGH                                    │
│  ├─ Beam search with ef_thorough (default 200)                │
│  ├─ Deeper synthesis graph traversal                          │
│  ├─ Record feedback for threshold learning                    │
│  └─ If confidence >= 0.95: return                             │
│                                                                │
│  Stage 4: ❄️ COLD (Exact)                                     │
│  ├─ Linear scan all vectors                                   │
│  ├─ Full synthesis proximity calculation                      │
│  └─ Confidence: 1.0 (exact)                                    │
│                                                                │
│  Adaptive Learning:                                            │
│  ├─ Compare fast vs thorough results                          │
│  ├─ Calculate actual recall                                   │
│  └─ Adjust fast_threshold based on observed performance       │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## API Reference

### Basic Operations

```rust
use koru_delta::vector::{
    SynthesisGraph, Vector, ContentHash,
    SynthesisType, NavigationOp
};

// Create graph
let graph = SynthesisGraph::new_with_params(16, 100);

// Insert vectors (automatically deduplicated)
let vector = Vector::new(vec![0.1, 0.2, 0.3], "text-embedding-3-small");
let id = graph.insert(vector)?;

// Standard search
let results = graph.search(&query, 10)?;
```

### Explainable Search

```rust
// Get results with explanations
let explainable = graph.search_explainable(&query, 5)?;

for result in explainable {
    println!("Match: {:.2}", result.result.score);
    println!("Why: {}", result.explanation.description);
    println!("Shared distinctions: {}", result.explanation.shared_distinctions);
    println!("Relationships: {:?}", result.explanation.relationships);
}
```

### Semantic Navigation

```rust
// Navigate by concept operations (analogies)
// king - man + woman = queen

let ops = vec![
    NavigationOp::Subtract(man_id),
    NavigationOp::Add(woman_id),
];

let results = graph.synthesis_navigate(&king_id, &ops, 5)?;
// Returns vectors near "queen" in semantic space
```

### Synthesis Statistics

```rust
// Get edge type distribution
let stats = graph.synthesis_stats();
// {Proximity: 1200, Composition: 340, Abstraction: 89, ...}

// Get abstraction level distribution
let dist = graph.abstraction_distribution();
// {0: 800, 1: 150, 2: 50} (specific → abstract)

// Get cache statistics
let (size, hits, epoch) = graph.cache_stats();
```

---

## Test Results

### Unit Tests (15 tests, all passing)

```
test_content_hash_consistency        ✓ Same vector = same hash
test_content_addressed_deduplication ✓ Identical vectors deduplicate
test_synthesis_edge_creation         ✓ Edges created with types
test_synthesis_edge_types            ✓ All 6 types work
test_synthesis_type_display          ✓ Display impl correct
test_synthesis_proximity_calculation ✓ Proximity formula works
test_node_synthesis_edges_by_type    ✓ Edge filtering works
test_distinction_overlap             ✓ Overlap calculation correct
test_explainable_search              ✓ Explanations provided
test_synthesis_navigation_operations ✓ Navigation ops work
test_abstraction_level_distribution  ✓ Abstraction levels assigned
test_generation_cache                ✓ Cache hits/misses work
test_adaptive_thresholds             ✓ Threshold learning works
test_escalating_search               ✓ All tiers functional
test_graph_connectivity              ✓ Graph well-connected
```

### Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| Insert | O(n) | Linear scan to find M neighbors |
| Search (Hot) | O(1) | Exact hash lookup |
| Search (Warm-Fast) | O(log n) | Beam search with ef_fast |
| Search (Warm-Thorough) | O(log n) | Beam search with ef_thorough |
| Search (Cold) | O(n) | Linear scan fallback |
| Memory | ~1.2x | Synthesis edges add ~20% overhead |
| Deduplication | Automatic | Saves memory for duplicate vectors |

---

## Comparison: SNSW vs HNSW

| Feature | HNSW | SNSW v2.2.0 |
|---------|------|-------------|
| **Node Identity** | Random/sequential ID | Content hash (Blake3) |
| **Deduplication** | ❌ None | ✅ Automatic |
| **Edge Meaning** | Geometric proximity only | Semantic relationships (6 types) |
| **Proximity Metric** | Cosine similarity | Synthesis proximity (geo+sem+causal) |
| **Explainability** | "Distance = 0.85" | Full synthesis paths |
| **Semantic Navigation** | ❌ No | ✅ Concept composition |
| **Abstraction Layers** | ❌ No | ✅ Structure in place |
| **Causal Awareness** | ❌ No | ✅ Timestamps, sequence edges |
| **Search Tiers** | Single | Hot→Warm→Thorough→Cold |
| **Adaptive Learning** | ❌ No | ✅ Threshold learning |

---

## Open Research Questions

### 1. Optimal Synthesis Function (v2.3.0)

Current weights are heuristic. Need to learn optimal combination:

```rust
// Current (heuristic)
score = 0.5*geo + 0.35*sem + 0.15*cau

// Future (learned)
score = synthesis_mlp.forward([geo, sem, cau, context])
```

**Research needed**: Collect human similarity judgments, train small MLP.

### 2. Abstraction Detection (v2.3.0)

Abstraction levels are currently assigned heuristically based on edge diversity.

**Research needed**: HDBSCAN clustering + semantic validation.

### 3. koru-lambda-core Integration (v2.3.0)

Currently using vector similarity as proxy for distinction relationships.

**Research needed**: Integrate with `DistinctionEngine` for true semantic decomposition.

### 4. Benchmark Validation

**Critical gap**: Need to prove SNSW improves over HNSW on real datasets.

**Action needed**: 
- GloVe word embeddings (10K-100K vectors)
- Recall@K comparison
- Latency comparison
- Memory usage comparison

---

## Implementation Notes

### Thread Safety

All structures use lock-free or fine-grained locking:
- `DashMap` for nodes, cache, distinction registry
- `RwLock` for entry points (rarely modified)
- `AtomicU64` for counters

### Memory Layout

```
SynthesisGraph (~200 bytes base)
├── nodes: DashMap (~64 bytes + entries)
│   └── SynthesisNode (~200-500 bytes each)
│       ├── vector: Arc<Vector> (~512 bytes for 128-dim)
│       ├── synthesis_edges: Vec<SynthesisEdge> (~16*24 bytes)
│       └── metadata (~100 bytes)
├── cache: DashMap (~64 bytes + entries)
│   └── CachedResult (~100-500 bytes each)
├── distinction_registry: DashMap
└── abstraction_layers: Vec (currently unused)
```

### Backward Compatibility

- `SynthesisGraph` maintains `edges` field for code expecting `(ContentHash, f32)` pairs
- `HnswIndex` unchanged - can use either implementation
- All existing tests pass without modification

---

## Next Steps

### v2.2.0 (Current) - Complete ✅
- [x] Core SNSW implementation
- [x] Synthesis edge types
- [x] Explainable search
- [x] Semantic navigation
- [x] Production testing (15 tests)

### v2.3.0 (Next) - Research Required
- [ ] Benchmark: SNSW vs HNSW on 10K vectors
- [ ] Automatic abstraction detection (HDBSCAN)
- [ ] Learned synthesis weights (MLP training)
- [ ] koru-lambda-core integration
- [ ] Cross-modal synthesis (text+image)

### v2.4.0+ (Future)
- [ ] Pure distinction-based navigation (no HNSW fallback)
- [ ] AGI-ready semantic memory interface
- [ ] Distributed SNSW (multi-node graph)

---

## References

- Original SNSW concept: [DISTINCTION_BASED_VECTOR_SEARCH.md](../bindings/python/docs/DISTINCTION_BASED_VECTOR_SEARCH.md)
- HNSW paper: Malkov & Yashunin, "Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs" (2018)
- Distinction Calculus: koru-lambda-core documentation
- Free Energy Principle: Friston, "The free-energy principle" (2010)

---

**Document Version**: 1.0  
**Last Updated**: 2026-02-08  
**Author**: KoruDelta Team
