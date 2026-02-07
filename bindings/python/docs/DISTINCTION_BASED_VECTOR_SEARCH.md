# Distinction-Based Vector Search (SNSW)

**Concept:** Synthesis-Navigable Small World  
**Innovation:** Apply distinction calculus to ANN search  
**Status:** Research/Design Phase  
**Date:** 2026-02-07

---

## The Insight

**HNSW** treats vectors as geometric points in space.  
**SNSW** treats vectors as **distinctions** in a semantic causal graph.

> "Similar vectors are not just close in space - they are synthesized from similar distinctions."

---

## Core Concepts from Distinction Calculus

### 1. Distinction → Identity

In koru-lambda-core:
```
A distinction creates identity.
Identical distinctions = identical identity.
```

**Applied to vectors:**
```rust
// Content-addressed vectors (not random IDs)
vector_id = blake3(vector_data + model_name)

// Identical embeddings automatically deduplicate
// Same vector, same hash, same node in graph
```

**Benefit:** Natural deduplication. Store 1M vectors, but if 100K are duplicates, you only store 900K.

### 2. Synthesis → Relationships

In koru-lambda-core:
```
Synthesis combines distinctions into new distinctions.
S(A, B) = C means C is synthesized from A and B.
```

**Applied to vectors:**
```rust
// Two vectors are "synthesized" from nearby points in distinction space
// Their relationship is not just distance - it's synthesis proximity

// Example: "king" - "man" + "woman" = "queen"
// These vectors are related through synthesis operations
```

**Benefit:** Navigate by semantic relationships, not just geometric proximity.

### 3. Content-Addressing → Deduplication

All vectors are stored in a content-addressed Merkle DAG:
```
vector_hash = hash(vector_data)
vector_ref = DAG.get(vector_hash)  // O(1) lookup
```

**Benefit:** 
- Tamper-evident (change vector → change hash → detect)
- Natural deduplication
- Immutable history (old versions preserved)

---

## Synthesis-Navigable Small World (SNSW)

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              SNSW: Distinction-Based ANN                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Layer 3 (Top): Coarse Distinctions                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  • "Animals" vs "Vehicles" vs "Concepts"            │   │
│  │  • High-level semantic categories                   │   │
│  │  • Sparse, long-range connections                   │   │
│  └──────────────────────┬──────────────────────────────┘   │
│                         │ synthesis edges                  │
│  Layer 2: Fine Distinctions                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  • "Dogs", "Cats", "Cars", "Trucks"                 │   │
│  │  • Medium granularity                               │   │
│  │  • Medium-range connections                         │   │
│  └──────────────────────┬──────────────────────────────┘   │
│                         │ synthesis edges                  │
│  Layer 1 (Bottom): Specific Instances                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  • "Golden Retriever puppy", "Tesla Model 3"        │   │
│  │  • Individual vectors                               │   │
│  │  • Dense, short-range connections                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Each node: Content-addressed (hash of vector)              │
│  Each edge: Synthesis relationship (not just distance)       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Key Differences from HNSW

| Aspect | HNSW | SNSW (Distinction-Based) |
|--------|------|--------------------------|
| **Node ID** | Random/Sequential | Content hash (Blake3) |
| **Edge meaning** | Geometric proximity | Synthesis relationship |
| **Layer organization** | Random long-range shortcuts | Semantic abstraction levels |
| **Deduplication** | None | Automatic (same hash = same node) |
| **Navigation** | Greedy by distance | Greedy by synthesis proximity |
| **Time travel** | Not supported | Natural (versioned distinctions) |
| **Explainability** | "Distance = 0.23" | "Synthesized from concepts X, Y" |

### The Synthesis Proximity Metric

Instead of just cosine similarity:

```rust
/// Synthesis proximity combines multiple factors
fn synthesis_proximity(a: &Vector, b: &Vector) -> f32 {
    let geometric = cosine_similarity(a, b);
    
    // Distinction calculus factors:
    let shared_distinctions = count_shared_distinctions(a, b);
    let synthesis_path_length = shortest_synthesis_path(a, b);
    let causal_relatedness = causal_proximity(a.id, b.id);
    
    // Weighted combination
    0.4 * geometric 
        + 0.3 * shared_distinctions 
        + 0.2 * (1.0 / synthesis_path_length)
        + 0.1 * causal_relatedness
}
```

**Shared Distinctions:** Two vectors about "Python programming" share the "Python" distinction, even if their geometric distance is large.

**Synthesis Path:** Navigate from "programming" → "languages" → "Python" → "asyncio" (semantic path, not just spatial).

**Causal Relatedness:** Vectors stored at similar times or in similar contexts are related (causal graph traversal).

---

## Implementation Sketch

### Data Structures

```rust
/// A node in the SNSW graph
/// Content-addressed by vector hash
#[derive(Clone)]
struct DistinctionNode {
    /// Content hash = identity (Blake3 of vector data)
    hash: ContentHash,
    
    /// The actual vector data
    vector: Arc<Vector>,
    
    /// Metadata for synthesis tracking
    metadata: DistinctionMetadata,
    
    /// Synthesis edges (not just distance-based)
    edges: Vec<SynthesisEdge>,
}

/// An edge represents a synthesis relationship
struct SynthesisEdge {
    /// Target node (content hash)
    target: ContentHash,
    
    /// Type of synthesis relationship
    relationship: SynthesisType,
    
    /// Strength of synthesis (0.0 to 1.0)
    strength: f32,
    
    /// Causal version when edge was created
    created_at: VersionId,
}

enum SynthesisType {
    /// Geometric similarity (traditional)
    Proximity,
    
    /// Semantic composition (A + B → C)
    Composition,
    
    /// Abstraction (specific → general)
    Abstraction,
    
    /// Instantiation (general → specific)
    Instantiation,
    
    /// Temporal sequence (time-based)
    Sequence,
    
    /// Causal dependency (A caused B)
    Causation,
}

/// Multi-layer graph (like HNSW layers, but semantic)
struct SynthesisGraph {
    /// Layer 0: Specific instances (dense)
    base_layer: DashMap<ContentHash, DistinctionNode>,
    
    /// Layer 1+: Abstract distinctions (sparse)
    /// Each layer is a coarser abstraction
    abstraction_layers: Vec<DashMap<ContentHash, DistinctionNode>>,
    
    /// Distinction engine from koru-lambda-core
    engine: DistinctionEngine,
}
```

### Building the Graph

```rust
impl SynthesisGraph {
    /// Insert a new vector into the graph
    pub async fn insert(&mut self, vector: Vector) -> ContentHash {
        // 1. Compute content hash (distinction identity)
        let hash = self.compute_content_hash(&vector);
        
        // 2. Check for existing (deduplication)
        if self.base_layer.contains_key(&hash) {
            return hash;  // Already exists!
        }
        
        // 3. Find synthesis neighbors (not just nearby vectors)
        let neighbors = self.find_synthesis_neighbors(&vector).await;
        
        // 4. Determine abstraction level
        let abstraction_level = self.compute_abstraction_level(&vector);
        
        // 5. Create synthesis edges
        let edges: Vec<SynthesisEdge> = neighbors
            .into_iter()
            .map(|(neighbor_hash, relationship, strength)| {
                SynthesisEdge {
                    target: neighbor_hash,
                    relationship,
                    strength,
                    created_at: self.current_version(),
                }
            })
            .collect();
        
        // 6. Insert into appropriate layers
        let node = DistinctionNode {
            hash: hash.clone(),
            vector: Arc::new(vector),
            metadata: DistinctionMetadata::new(),
            edges,
        };
        
        self.base_layer.insert(hash.clone(), node.clone());
        
        // 7. Also insert into abstraction layers if appropriate
        for layer in 0..abstraction_level {
            self.abstraction_layers[layer]
                .insert(hash.clone(), self.abstract_node(&node, layer));
        }
        
        hash
    }
    
    /// Find neighbors based on synthesis relationships
    async fn find_synthesis_neighbors(
        &self, 
        vector: &Vector
    ) -> Vec<(ContentHash, SynthesisType, f32)> {
        let mut candidates = Vec::new();
        
        // 1. Geometric proximity (traditional)
        let geometric_neighbors = self.geometric_search(vector, 50);
        for (hash, dist) in geometric_neighbors {
            candidates.push((hash, SynthesisType::Proximity, 1.0 - dist));
        }
        
        // 2. Semantic composition (via distinction engine)
        let components = self.engine.decompose(vector);
        for component in components {
            if let Some(hash) = self.find_by_distinction(component) {
                candidates.push((hash, SynthesisType::Composition, 0.9));
            }
        }
        
        // 3. Abstraction relationships
        let abstractions = self.find_abstractions(vector);
        for (hash, level) in abstractions {
            let strength = 1.0 / (level + 1) as f32;
            candidates.push((hash, SynthesisType::Abstraction, strength));
        }
        
        // 4. Temporal/causal neighbors
        let recent = self.find_recently_stored(100);
        for hash in recent {
            candidates.push((hash, SynthesisType::Sequence, 0.5));
        }
        
        // Rank by synthesis strength and return top-k
        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        candidates.truncate(32);  // M parameter like HNSW
        candidates
    }
}
```

### Search Algorithm

```rust
impl SynthesisGraph {
    /// Search for k nearest neighbors using synthesis navigation
    pub async fn search(
        &self, 
        query: &Vector, 
        k: usize,
        ef: usize  // Size of candidate list
    ) -> Vec<SearchResult> {
        // 1. Compute query's content hash
        let query_hash = self.compute_content_hash(query);
        
        // 2. Entry point: Find coarse abstraction matches (top layer)
        let mut entry_points = self.find_abstraction_matches(query, 3);
        
        // 3. Navigate down through layers (like HNSW)
        for layer in (0..self.abstraction_layers.len()).rev() {
            // Greedy search at this layer
            entry_points = self.greedy_search_layer(
                query, 
                &entry_points, 
                layer,
                ef
            ).await;
        }
        
        // 4. Final search in base layer (specific vectors)
        let candidates = self.greedy_search_layer(
            query,
            &entry_points,
            0,  // Base layer
            ef
        ).await;
        
        // 5. Return top-k by synthesis proximity
        let mut results: Vec<_> = candidates
            .into_iter()
            .map(|hash| {
                let node = self.base_layer.get(&hash).unwrap();
                let score = synthesis_proximity(query, &node.vector);
                SearchResult { hash, score }
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(k);
        results
    }
    
    /// Greedy search within a layer using synthesis proximity
    async fn greedy_search_layer(
        &self,
        query: &Vector,
        entry_points: &[ContentHash],
        layer: usize,
        ef: usize
    ) -> Vec<ContentHash> {
        let mut visited = HashSet::new();
        let mut candidates: BinaryHeap<(f32, ContentHash)> = BinaryHeap::new();
        let mut results: Vec<ContentHash> = Vec::new();
        
        // Initialize with entry points
        for hash in entry_points {
            if let Some(node) = self.get_node(hash, layer) {
                let score = synthesis_proximity(query, &node.vector);
                candidates.push((score, hash.clone()));
            }
        }
        
        // Greedy expansion
        while let Some((score, current)) = candidates.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            results.push(current.clone());
            
            if results.len() >= ef {
                break;
            }
            
            // Expand neighbors
            if let Some(node) = self.get_node(&current, layer) {
                for edge in &node.edges {
                    if !visited.contains(&edge.target) {
                        if let Some(neighbor) = self.get_node(&edge.target, layer) {
                            let neighbor_score = synthesis_proximity(
                                query, 
                                &neighbor.vector
                            );
                            // Weight by edge strength
                            let weighted_score = neighbor_score * edge.strength;
                            candidates.push((weighted_score, edge.target.clone()));
                        }
                    }
                }
            }
        }
        
        results
    }
}
```

---

## Unique Capabilities

### 1. Semantic Navigation

```python
# Navigate by concept relationships, not just distance
results = await db.synthesis_navigate(
    start="king",
    operations=[
        ("subtract", "man"),      # king - man
        ("add", "woman"),         # + woman
    ],
    top_k=5
)
# Returns: ["queen", "princess", "matriarch", ...]
```

### 2. Explainable Similarity

```python
# WHY are these vectors similar?
explanation = await db.explain_similarity(vec_a, vec_b)

# Returns:
{
    "geometric_similarity": 0.85,
    "shared_distinctions": ["programming", "python", "async"],
    "synthesis_path": ["programming" → "languages" → "python"],
    "causal_proximity": "Stored 2 minutes apart in same session"
}
```

### 3. Abstraction Search

```python
# Search at different levels of abstraction
specific = await db.search("golden retriever puppy", abstraction=0)  # Specific
general = await db.search("golden retriever puppy", abstraction=2)    # "Dogs"
very_general = await db.search("golden retriever puppy", abstraction=4)  # "Animals"
```

### 4. Causal Time-Travel Search

```python
# Search at a specific point in time
results_then = await db.synthesis_search_at(
    query="machine learning",
    timestamp="2024-01-01T00:00:00Z",
    top_k=10
)
# Returns what was similar at that time (concept drift tracking!)
```

---

## Advantages Over HNSW

| Feature | HNSW | SNSW (Distinction-Based) |
|---------|------|--------------------------|
| **Search Complexity** | O(log n) | O(log n) |
| **Build Complexity** | O(n log n) | O(n log n) |
| **Memory** | ~1.5x | ~1.2x (deduplication) |
| **Deduplication** | ❌ No | ✅ Automatic |
| **Explainability** | ❌ Distance only | ✅ Synthesis paths |
| **Time Travel** | ❌ Not possible | ✅ Versioned graph |
| **Semantic Nav** | ❌ No | ✅ Concept traversal |
| **Abstraction** | ❌ No | ✅ Multi-layer semantic |
| **Causal Links** | ❌ No | ✅ Built-in |

---

## Research Questions

1. **Optimal Synthesis Function**: How to best combine geometric, semantic, and causal factors?

2. **Abstraction Detection**: How to automatically detect abstraction levels from vectors?

3. **Dynamic Rebalancing**: How to maintain graph structure as new distinctions arrive?

4. **Cross-Modal Synthesis**: How to relate text, image, and audio embeddings through distinctions?

---

## Next Steps

1. **Literature Review**: Has anyone applied category theory/distinction calculus to ANN?

2. **Prototype**: Implement basic SNSW for 10K vectors, compare recall vs HNSW

3. **Abstraction Detection**: Research automatic abstraction level computation

4. **Integration**: Connect to koru-lambda-core's distinction engine

---

**Key Insight:** We're not just building a faster vector index - we're building a **semantic cognitive map** that mirrors how human memory organizes concepts through distinctions and relationships.

