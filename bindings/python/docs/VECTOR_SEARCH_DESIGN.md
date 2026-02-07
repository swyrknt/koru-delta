# Vector Search Enhancement Design

**Version:** v2.0 → v2.1  
**Status:** Design Phase  
**Author:** Agent Kimi  
**Date:** 2026-02-07

---

## Executive Summary

Current vector search uses a flat index (O(n) brute-force), suitable for 10K-100K vectors but inadequate for production RAG workloads. This design proposes a multi-tier ANN architecture that maintains KoruDelta's causal guarantees while achieving million-scale vector search.

---

## Current State (v2.5)

```rust
// Flat index - O(n) scan
pub struct VectorIndex {
    vectors: DashMap<String, Vector>,  // All vectors in memory
}

impl VectorIndex {
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        // Brute-force: compare against EVERY vector
        self.vectors.iter()
            .map(|(id, v)| (id, cosine_similarity(query, v)))
            .collect::<Vec<_>>()
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
            .take(k)
    }
}
```

**Performance:**
- 1K vectors: ~0.1ms
- 10K vectors: ~1ms
- 100K vectors: ~10ms
- 1M vectors: ~100ms ❌ Unusable

---

## Target State (v2.6)

### Goals

| Metric | Current | Target | Notes |
|--------|---------|--------|-------|
| Search Latency (p99) | 100ms @ 1M | 5ms @ 1M | 20x improvement |
| Memory Overhead | 1x | 1.5x | Acceptable for speed |
| Index Build Time | N/A | <5min @ 1M | Background process |
| Recall@10 | 100% | >95% | HNSW typical |
| Causal Consistency | ✓ | ✓ | Must preserve |

### Architecture: Tiered ANN Index

```
┌─────────────────────────────────────────────────────────────┐
│                    Vector Search Architecture               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Hot Index  │    │   HNSW Index │    │  Disk Index  │  │
│  │  (in-mem)    │    │  (in-mem)    │    │  (mmap)      │  │
│  │              │    │              │    │              │  │
│  │ • Recent     │    │ • Navigable  │    │ • Archived   │  │
│  │ • Frequently │    │   Small      │    │ • Cold       │  │
│  │   accessed   │    │   World      │    │   memories   │  │
│  │ • ~10K       │    │ • ~1M nodes  │    │ • Unlimited  │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │           │
│         └───────────────────┼───────────────────┘           │
│                             │                               │
│                    ┌────────▼────────┐                      │
│                    │  Search Router  │                      │
│                    │                 │                      │
│                    │ 1. Query Hot    │                      │
│                    │ 2. Query HNSW   │                      │
│                    │ 3. Merge        │                      │
│                    └─────────────────┘                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Detailed Design

### 1. HNSW Index Layer

**Hierarchical Navigable Small World** - graph-based ANN

```rust
pub struct HnswIndex {
    // Multi-layer graph structure
    layers: Vec<Layer>,
    
    // Entry point (top layer)
    entry_point: NodeId,
    
    // Configuration
    m: usize,           // Max connections per node (default: 16)
    ef_construction: usize,  // Size of dynamic candidate list (default: 200)
    ef_search: usize,   // Size of dynamic candidate list for search (default: 50)
}

struct Layer {
    nodes: HashMap<NodeId, Vec<NodeId>>,  // node_id -> neighbors
}

impl HnswIndex {
    /// Build index from vectors
    pub fn build(vectors: &[(String, Vector)]) -> Self {
        // Insert each vector, creating graph connections
        // O(n log n) construction
    }
    
    /// Search for k nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        // Greedy search from entry point
        // Descend through layers
        // O(log n) typical case
    }
}
```

**Properties:**
- **Construction:** O(n log n)
- **Search:** O(log n)  
- **Memory:** ~1.5x raw vectors (graph overhead)
- **Recall:** 95-99% typical

### 2. Causal-Consistent Index Updates

**Challenge:** HNSW is mutable graph. How do we maintain causality?

**Solution:** Versioned index snapshots

```rust
pub struct CausalVectorIndex {
    // Immutable index snapshots
    index_snapshots: DashMap<VersionId, Arc<HnswIndex>>,
    
    // Current working index
    current: RwLock<Arc<HnswIndex>>,
    
    // Delta of vectors since last snapshot
    pending_vectors: RwLock<Vec<(String, Vector)>>,
}

impl CausalVectorIndex {
    /// Add vector (causal - creates new version)
    pub async fn add(&self, id: String, vector: Vector) -> DeltaResult<()> {
        // Add to pending
        self.pending_vectors.write().push((id, vector));
        
        // If pending exceeds threshold, rebuild index
        if self.should_rebuild() {
            self.rebuild_index().await?;
        }
        
        Ok(())
    }
    
    /// Search at specific version (time travel!)
    pub fn search_at(&self, version: VersionId, query: &[f32], k: usize) 
        -> Vec<SearchResult> {
        // Get snapshot for that version
        let index = self.index_snapshots.get(&version)?;
        index.search(query, k)
    }
}
```

**Rebuild Strategy:**
- Threshold: 1000 new vectors or 5 minutes
- Background async rebuild
- Swap atomic pointer when complete
- Keep last 10 snapshots for time-travel queries

### 3. Multi-Tier Storage

```rust
pub struct TieredVectorStorage {
    // Tier 1: Hot (recent + frequent access)
    hot: DashMap<String, Vector>,
    hot_capacity: usize,  // LRU eviction
    
    // Tier 2: HNSW index (active search space)
    hnsw: CausalVectorIndex,
    hnsw_capacity: usize,  // 1M vectors
    
    // Tier 3: Disk (mmap for cold vectors)
    disk: Arc<DiskVectorStore>,
}

impl TieredVectorStorage {
    pub async fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        let mut results = Vec::new();
        
        // 1. Search hot (fast path)
        results.extend(self.search_hot(query, k));
        
        // 2. Search HNSW (main index)
        results.extend(self.hnsw.search(query, k));
        
        // 3. If insufficient results, search disk
        if results.len() < k {
            results.extend(self.disk.search(query, k - results.len()).await);
        }
        
        // Merge and rank
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(k);
        results
    }
}
```

### 4. Memory Lifecycle Integration

```rust
impl Workspace {
    /// Automatic memory tier management
    pub async fn consolidate_memories(&self) -> DeltaResult<()> {
        // Hot → Warm: Not accessed in 24h
        let stale = self.find_stale_memories(Duration::hours(24)).await?;
        for memory in stale {
            self.move_to_warm(&memory).await?;
        }
        
        // Warm → Cold: Not accessed in 7d, compress
        let old = self.find_warm_memories(Duration::days(7)).await?;
        for memory in old {
            let compressed = self.compress(&memory).await?;
            self.move_to_cold(&compressed).await?;
        }
        
        // Cold → Deep: Not accessed in 30d, embed-only
        let ancient = self.find_cold_memories(Duration::days(30)).await?;
        for memory in ancient {
            let genomic = self.genomic_encode(&memory).await?;
            self.move_to_deep(&genomic).await?;
        }
        
        Ok(())
    }
}
```

---

## Implementation Plan

### Phase 1: HNSW Core (Week 1)
- [ ] Implement HNSW data structure
- [ ] Add graph construction algorithm
- [ ] Implement greedy search
- [ ] Unit tests (recall benchmarks)

### Phase 2: Causal Integration (Week 2)
- [ ] Versioned index snapshots
- [ ] Background rebuild pipeline
- [ ] Time-travel search (`search_at`)
- [ ] Integration with existing VectorIndex

### Phase 3: Multi-Tier (Week 3)
- [ ] Hot vector cache (LRU)
- [ ] Disk-backed cold storage
- [ ] Tier promotion/demotion
- [ ] Performance benchmarks

### Phase 4: Python Bindings (Week 4)
- [ ] Expose `search_hnsw()` API
- [ ] Configuration options (m, ef_construction)
- [ ] Benchmark script
- [ ] Documentation

---

## Benchmarks

### Target Performance

| Dataset Size | Index Build | Query (p50) | Query (p99) | Recall@10 |
|--------------|-------------|-------------|-------------|-----------|
| 10K | 100ms | 0.01ms | 0.05ms | 100% |
| 100K | 2s | 0.1ms | 0.3ms | 98% |
| 1M | 30s | 1ms | 5ms | 95% |
| 10M | 5min | 2ms | 10ms | 92% |

### Comparison

| System | 1M Query | Notes |
|--------|----------|-------|
| Flat (current) | 100ms | Exact, but slow |
| **HNSW (target)** | **5ms** | **95% recall** |
| Pinecone | 10ms | Cloud-only |
| Milvus | 5ms | Heavy deployment |
| pgvector | 20ms | PostgreSQL plugin |

---

## API Changes

### Rust API

```rust
// Current (preserved for backward compat)
impl KoruDelta {
    pub async fn embed_search(&self, ...) -> Vec<SearchResult>;  // Uses flat
}

// New: HNSW search
impl KoruDelta {
    /// Search using HNSW index (fast, approximate)
    pub async fn similar_approximate(&self, 
        namespace: &str,
        query: &[f32],
        top_k: usize,
        ef_search: Option<usize>,  // HNSW parameter
    ) -> Vec<SearchResult>;
    
    /// Search at specific version (causal time travel!)
    pub async fn similar_at(&self,
        namespace: &str,
        query: &[f32],
        timestamp: &str,
        top_k: usize,
    ) -> Vec<SearchResult>;
    
    /// Force index rebuild
    pub async fn rebuild_vector_index(&self, namespace: &str) -> DeltaResult<()>;
}
```

### Python API

```python
# Current (preserved)
results = await db.similar("docs", query=[0.1, ...], top_k=10)

# New: Approximate search
results = await db.similar_approximate(
    "docs", 
    query=[0.1, ...], 
    top_k=10,
    ef_search=50  # Higher = more accurate, slower
)

# New: Time-travel vector search
results = await db.similar_at(
    "docs",
    query=[0.1, ...],
    timestamp="2026-02-01T10:00:00Z",  # What was similar then?
    top_k=10
)
```

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| HNSW complexity | High | Use well-tested crate (hnsw_rs) |
| Memory overhead | Medium | Configurable M parameter |
| Build time | Medium | Background async, incremental |
| Recall degradation | Low | Fallback to exact search for small k |
| Causal consistency | High | Versioned snapshots + tests |

---

## Success Criteria

- [ ] 1M vectors searchable in <10ms (p99)
- [ ] Recall@10 > 95% on standard benchmarks
- [ ] Index builds in background without blocking writes
- [ ] Time-travel vector search works (causal guarantee)
- [ ] Memory overhead < 2x raw vectors
- [ ] Python bindings expose all features
- [ ] Benchmarks published

---

## Related Work

- **hnswlib**: Reference C++ implementation
- **hnsw_rs**: Rust port (evaluate for use)
- **pgvector**: PostgreSQL vector extension (comparison)
- **Pinecone**: Managed vector DB (competitor benchmark)

---

**Next Steps:**
1. Evaluate hnsw_rs vs custom implementation
2. Create proof-of-concept (100K vectors)
3. Benchmark against current flat index
4. Integrate with causal versioning system
