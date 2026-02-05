# KoruDelta v2.0 Design: The Distinction-Driven System

> *"A living system that recognizes distinctions and their causal becoming."*

## Table of Contents

1. [Philosophy](#philosophy)
2. [Core Concepts](#core-concepts)
3. [System Architecture](#system-architecture)
4. [The Distinction Engine](#the-distinction-engine)
5. [Layered Memory](#layered-memory)
6. [Evolutionary Processes](#evolutionary-processes)
7. [World Reconciliation](#world-reconciliation)
8. [Implementation Details](#implementation-details)
9. [API Design](#api-design)
10. [Future Directions](#future-directions)

---

## Philosophy

### The Problem with Traditional Databases

Traditional databases treat data as **dead objects**—things to be stored, retrieved, and eventually deleted. They have no concept of:
- How data came to be (causality)
- What data relates to what (reference)
- The flow of change over time (process)
- The essence vs. the noise (distillation)

### The Distinction Calculus Approach

KoruDelta v2.0 treats the system as **living**—constantly synthesizing new distinctions from prior ones, remembering what matters, forgetting what doesn't, and reconciling with other systems.

**Key Insight:** Everything is a distinction. Every operation, every value, every relationship is a distinction that participates in a causal graph.

### The 5 Axioms as Living Principles

1. **Existence** - Every synthesis is a birth
2. **Non-contradiction** - Identity is fundamental
3. **Causality** - Nothing comes from nothing
4. **Composition** - The whole is greater than the sum
5. **Reference** - Everything points to something

---

## Core Concepts

### Distinction

A distinction is the fundamental unit of the system—not data, but **difference**.

```rust
/// A distinction is a "what-ness" recognized by the system
pub struct Distinction {
    /// Unique identity (content hash)
    id: DistinctionId,
    
    /// When this distinction emerged
    emergence: Timestamp,
    
    /// What causally preceded this distinction
    causal_parents: Vec<DistinctionId>,
    
    /// What this distinction references
    references: Vec<DistinctionId>,
    
    /// The content (if any) that was distinguished
    content_hash: Option<DistinctionId>,
}
```

**Key Properties:**
- Immutable once emerged
- Has identity (can be referenced)
- Has causality (emerged from something)
- Has references (points to other distinctions)

### Synthesis

A synthesis is the **process** by which new distinctions emerge.

```rust
/// Synthesis is the act of distinction-creation
pub struct Synthesis {
    /// The new distinction that emerged
    emergence: Distinction,
    
    /// The context in which it emerged
    context: Context,
    
    /// The operation that caused it
    operation: Operation,
}

pub enum Operation {
    Put { namespace: String, key: String, value: Value },
    Delete { namespace: String, key: String },
    Merge { distinctions: Vec<DistinctionId> },
}
```

**The Rhythm:** Every `put()` is a synthesis. Every synthesis updates the causal graph.

### Context

A context is a **namespace for becoming**—a place where distinctions emerge.

```rust
/// Context is like a Petri dish for distinctions
pub struct Context {
    /// The namespace (e.g., "users", "sessions")
    namespace: String,
    
    /// The specific key (e.g., "alice", "session:123")
    key: String,
    
    /// The current distinction in this context
    current: DistinctionId,
    
    /// The causal chain leading to current
    lineage: Vec<DistinctionId>,
}
```

---

## System Architecture

### The Living System

```
┌─────────────────────────────────────────────────────────────────┐
│                    THE LIVING SYSTEM                             │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │  Synthesis   │───▶│   Memory     │───▶│  Evolution   │       │
│  │   (Input)    │    │   (State)    │    │  (Process)   │       │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│         │                   │                   │                │
│         │                   │                   │                │
│         ▼                   ▼                   ▼                │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              DISTINCTION ENGINE                       │       │
│  │  (Causal Graph + Reference Graph + Content Space)    │       │
│  └──────────────────────────────────────────────────────┘       │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              WORLD RECONCILIATION                     │       │
│  │         (Sync with other systems)                    │       │
│  └──────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Input (put/delete/query)
    │
    ▼
┌─────────────────┐
│   Synthesize    │ ──▶ Create distinction
│   (Operation)   │ ──▶ Update causal graph
└─────────────────┘ ──▶ Track references
    │
    ▼
┌─────────────────┐
│    Hot Memory   │ ──▶ Immediate access
│   (Working)     │
└─────────────────┘
    │
    ├── After 5 min ──▶ Consolidate to Warm
    │
    ▼
┌─────────────────┐
│   Warm Memory   │ ──▶ Recent chronicle
│   (Recent)      │
└─────────────────┘
    │
    ├── After 1 hour ──▶ Distill to Cold
    │
    ▼
┌─────────────────┐
│   Cold Memory   │ ──▶ Compressed essence
│ (Consolidated)  │
└─────────────────┘
    │
    ├── After 1 day ──▶ Archive to Deep
    │
    ▼
┌─────────────────┐
│   Deep Memory   │ ──▶ Genomic storage
│   (Genomic)     │
└─────────────────┘
```

---

## The Distinction Engine

### The Heart of the System

```rust
pub struct DistinctionEngine {
    /// All distinctions ever emerged (the memory)
    distinctions: DashMap<DistinctionId, Distinction>,
    
    /// Causal graph: what emerged from what
    causal_graph: CausalGraph,
    
    /// Reference graph: what points to what
    reference_graph: ReferenceGraph,
    
    /// Content-addressed store
    content_space: ContentSpace,
    
    /// Current epoch (for garbage collection)
    current_epoch: Epoch,
}
```

### Causal Graph

The causal graph tracks **becoming**—how distinctions flow from prior distinctions.

```rust
pub struct CausalGraph {
    /// Edges: parent -> child
    edges: DashMap<DistinctionId, Vec<DistinctionId>>,
    
    /// Reverse edges: child -> parents
    reverse_edges: DashMap<DistinctionId, Vec<DistinctionId>>,
}

impl CausalGraph {
    /// Get all descendants of a distinction
    pub fn descendants(&self, root: DistinctionId) -> Vec<DistinctionId> {
        // BFS through causal edges
    }
    
    /// Get all ancestors (causal history)
    pub fn ancestors(&self, leaf: DistinctionId) -> Vec<DistinctionId> {
        // Reverse BFS
    }
    
    /// Find the causal frontier (latest distinctions)
    pub fn frontier(&self, context: &Context) -> Vec<DistinctionId> {
        // Distinctions with no children in this context
    }
    
    /// Compute least common ancestor
    pub fn lca(&self, a: DistinctionId, b: DistinctionId) -> Option<DistinctionId> {
        // For merge conflict resolution
    }
}
```

### Reference Graph

The reference graph tracks **pointing**—what distinctions reference what.

```rust
pub struct ReferenceGraph {
    /// What each distinction points TO
    outgoing: DashMap<DistinctionId, Vec<DistinctionId>>,
    
    /// What points TO each distinction (for GC)
    incoming: DashMap<DistinctionId, Vec<DistinctionId>>,
}

impl ReferenceGraph {
    /// Reference count for GC
    pub fn reference_count(&self, distinction: DistinctionId) -> usize {
        self.incoming.get(&distinction).map(|v| v.len()).unwrap_or(0)
    }
    
    /// Is this distinction reachable from roots?
    pub fn is_reachable(&self, distinction: DistinctionId) -> bool {
        // Traverse from roots, see if we hit this distinction
    }
}
```

### Content Space

Content-addressed storage for the **what** (as opposed to the **how** of causal graph).

```rust
pub struct ContentSpace {
    /// Distinction ID -> Content
    store: ContentAddressedStore,
}

impl ContentSpace {
    /// Store content, get distinction ID
    pub async fn store(&self, content: &Value) -> DistinctionId {
        let hash = sha256::hash(content);
        self.store.save(&hash, content).await;
        hash
    }
    
    /// Retrieve content by distinction ID
    pub async fn retrieve(&self, id: DistinctionId) -> Option<Value> {
        self.store.load(&id).await
    }
}
```

---

## Layered Memory

### The Brain Metaphor

Like the human brain, KoruDelta has layers of memory with different properties:

| Layer | Speed | Capacity | Duration | Analogy |
|-------|-------|----------|----------|---------|
| Hot | 10ns | ~1000 | Minutes | Prefrontal cortex |
| Warm | 1μs | ~1M | Hours | Hippocampus |
| Cold | 1ms | ~1B | Days | Cerebral cortex |
| Deep | 10ms | ∞ | Forever | DNA/Genome |

### Hot Memory (Working)

Immediate, conscious awareness.

```rust
pub struct HotMemory {
    /// LRU cache of recent distinctions
    cache: LruCache<DistinctionId, Distinction>,
    
    /// Current context -> distinction mapping
    current_state: DashMap<Context, DistinctionId>,
}

impl HotMemory {
    /// Access a distinction (promotes to hot)
    pub fn access(&mut self, distinction: Distinction) {
        self.cache.put(distinction.id, distinction);
    }
    
    /// Get current distinction for context
    pub fn current(&self, context: &Context) -> Option<DistinctionId> {
        self.current_state.get(context).map(|v| *v)
    }
}
```

### Warm Memory (Recent)

Recent chronicle with full causal detail.

```rust
pub struct WarmMemory {
    /// Full chronicle for recent distinctions
    chronicle: ProcessChronicle,
    
    /// Last N distinctions kept warm
    recent_window: VecDeque<DistinctionId>,
}

impl WarmMemory {
    /// Add to warm memory
    pub async fn warm(&mut self, distinction: Distinction) {
        self.chronicle.append(distinction).await;
        self.recent_window.push_back(distinction.id);
        
        // Evict old to cold
        if self.recent_window.len() > WARM_CAPACITY {
            let old = self.recent_window.pop_front().unwrap();
            self.consolidate_to_cold(old).await;
        }
    }
}
```

### Cold Memory (Consolidated)

Compressed essence of history.

```rust
pub struct ColdMemory {
    /// Consolidated epochs
    epochs: Vec<Epoch>,
    
    /// Pattern index (for reconstruction)
    patterns: PatternIndex,
}

pub struct Epoch {
    /// Time range
    start: Timestamp,
    end: Timestamp,
    
    /// Essential structure for this epoch
    essence: EssentialStructure,
    
    /// Compressed chronicle
    compressed: CompressedChronicle,
}

impl ColdMemory {
    /// Consolidate warm memory into cold
    pub async fn consolidate(&mut self, warm: &WarmMemory) {
        // Extract patterns
        let patterns = self.extract_patterns(warm);
        
        // Compress chronicle
        let compressed = self.compress(warm, &patterns);
        
        // Store epoch
        let epoch = Epoch {
            start: warm.start_time(),
            end: warm.end_time(),
            essence: warm.extract_essence(),
            compressed,
        };
        
        self.epochs.push(epoch);
    }
}
```

### Deep Memory (Genomic)

The genome—minimal information to recreate the whole.

```rust
pub struct DeepMemory {
    /// The genome: minimal self-recreation info
    genome: Genome,
    
    /// Archive of old epochs
    archive: GenomicArchive,
}

pub struct Genome {
    /// Root distinctions (stem cells)
    roots: Vec<DistinctionId>,
    
    /// Causal topology (shape of becoming)
    topology: CausalTopology,
    
    /// Reference patterns
    patterns: ReferencePatterns,
    
    /// Current epoch summary
    current_epoch: EpochSummary,
}

impl DeepMemory {
    /// Extract genome from current state
    pub fn extract_genome(&self, engine: &DistinctionEngine) -> Genome {
        Genome {
            roots: engine.find_roots(),
            topology: engine.capture_topology(),
            patterns: engine.capture_patterns(),
            current_epoch: self.summarize_current_epoch(),
        }
    }
    
    /// Express genome: recreate system
    pub async fn express(&self, genome: &Genome) -> DistinctionEngine {
        let mut engine = DistinctionEngine::new();
        
        // Express roots
        for root in &genome.roots {
            engine.synthesize_root(root).await;
        }
        
        // Follow topology
        for path in &genome.topology.paths {
            engine.synthesize_path(path).await;
        }
        
        // Establish patterns
        for pattern in &genome.patterns {
            engine.establish_pattern(pattern).await;
        }
        
        engine
    }
}
```

---

## Evolutionary Processes

### The Rhythm of the System

```rust
pub struct SystemRhythm {
    /// How long distinctions stay hot
    hot_duration: Duration,
    
    /// How often to consolidate (warm → cold)
    consolidation_interval: Duration,
    
    /// How often to archive (cold → deep)
    archival_interval: Duration,
    
    /// How often to update genome
    genome_update_interval: Duration,
}

impl Default for SystemRhythm {
    fn default() -> Self {
        SystemRhythm {
            hot_duration: Duration::minutes(5),
            consolidation_interval: Duration::hours(1),
            archival_interval: Duration::days(1),
            genome_update_interval: Duration::days(7),
        }
    }
}
```

### Synthesis (Birth)

```rust
impl DistinctionEngine {
    pub fn synthesize(&mut self, operation: Operation, context: Context) -> Synthesis {
        // 1. Create the content distinction
        let content_id = match &operation {
            Operation::Put { value, .. } => {
                self.content_space.store(value)
            }
            _ => None,
        };
        
        // 2. Find causal parents
        let causal_parents = self.find_causal_parents(&context);
        
        // 3. Find references
        let references = self.find_references(&operation);
        
        // 4. Create the distinction
        let distinction = Distinction {
            id: self.compute_distinction_id(&operation, &causal_parents),
            emergence: Timestamp::now(),
            causal_parents,
            references,
            content_hash: content_id,
        };
        
        // 5. Update graphs
        self.causal_graph.add_node(&distinction);
        self.reference_graph.add_node(&distinction);
        
        // 6. Store
        self.distinctions.insert(distinction.id, distinction.clone());
        
        Synthesis {
            emergence: distinction,
            context,
            operation,
        }
    }
}
```

### Consolidation (Sleep)

```rust
pub struct ConsolidationProcess;

impl ConsolidationProcess {
    pub async fn consolidate(&self, system: &mut KoruDelta) {
        // 1. Extract patterns from hot memory
        let patterns = system.hot_memory.extract_patterns();
        
        // 2. Move to warm
        for distinction in system.hot_memory.drain() {
            system.warm_memory.warm(distinction).await;
        }
        
        // 3. Distill warm to cold
        if system.warm_memory.should_distill() {
            system.cold_memory.consolidate(&system.warm_memory).await;
            system.warm_memory.clear();
        }
        
        // 4. Update cold patterns
        system.cold_memory.update_patterns(patterns);
    }
}
```

### Distillation (Natural Selection)

```rust
pub struct DistillationProcess {
    engine: DistinctionEngine,
}

impl DistillationProcess {
    /// Fitness function for distinctions
    fn fitness(&self, distinction: &Distinction) -> Fitness {
        Fitness {
            // Referenced by many = fit
            reference_count: self.engine.reference_count(distinction.id),
            
            // Has many descendants = fit
            descendant_count: self.engine.descendant_count(distinction.id),
            
            // Recently emerged = fit (recency bias)
            recency: self.recency_score(distinction.emergence),
            
            // Part of important pattern = fit
            pattern_importance: self.pattern_score(distinction.id),
        }
    }
    
    pub async fn distill(&self) -> DistillationResult {
        let all = self.engine.all_distinctions();
        
        // Classify by fitness
        let (fit, unfit): (Vec<_>, Vec<_>) = all
            .into_iter()
            .partition(|d| self.fitness(d).is_viable());
        
        // Fit: keep in working memory
        // Unfit: archive to deep memory
        // (But never delete—information is never destroyed)
        
        DistillationResult {
            preserved: fit,
            archived: unfit,
        }
    }
}
```

### Genome Update

```rust
pub struct GenomeUpdateProcess;

impl GenomeUpdateProcess {
    pub async fn update(&self, system: &mut KoruDelta) {
        // Extract essential structure
        let essential = EssentialStructure {
            roots: system.engine.find_roots(),
            branch_points: system.engine.find_high_out_degree(),
            convergence_points: system.engine.find_high_in_degree(),
            terminals: system.engine.find_recent_leaves(),
        };
        
        // Create new genome
        let genome = Genome {
            roots: essential.roots,
            topology: system.engine.capture_topology(),
            patterns: system.engine.capture_patterns(),
            current_epoch: system.cold_memory.current_epoch_summary(),
        };
        
        // Store in deep memory
        system.deep_memory.update_genome(genome);
    }
}
```

---

## World Reconciliation

### Set Reconciliation via Distinctions

```rust
pub struct WorldReconciliation {
    engine: DistinctionEngine,
}

impl WorldReconciliation {
    /// Reconcile with another world (peer)
    pub async fn reconcile(&self, peer: &mut Peer) -> ReconciliationResult {
        // 1. Exchange root distinction sets
        let my_roots = self.engine.root_distinctions();
        let their_roots = peer.exchange_roots(&my_roots).await;
        
        // 2. Find missing distinctions using Bloom filters
        let missing_in_me = self.find_missing(&their_roots);
        let missing_in_them = self.find_missing(&my_roots);
        
        // 3. Exchange missing distinctions
        for distinction_id in missing_in_them {
            let distinction = self.engine.get(distinction_id);
            peer.send_distinction(distinction).await;
        }
        
        // 4. Receive from peer
        for distinction_id in missing_in_me {
            let distinction = peer.receive_distinction().await;
            self.engine.integrate(distinction);
        }
        
        // 5. Merge causal graphs
        self.engine.merge_causal_graph(peer.causal_graph());
        
        ReconciliationResult {
            sent: missing_in_them.len(),
            received: missing_in_me.len(),
        }
    }
}
```

### Conflict as Causal Branching

```rust
impl DistinctionEngine {
    /// When two worlds have divergent histories
    pub fn merge_worlds(&mut self, other: &CausalGraph) -> MergeResult {
        // Find common ancestor
        let lca = self.causal_graph.least_common_ancestor(&other);
        
        // Create merge distinction that has both branches as parents
        let merge_distinction = Distinction {
            id: self.compute_merge_id(&lca, &other),
            emergence: Timestamp::now(),
            causal_parents: vec![
                self.causal_graph.frontier(),
                other.frontier(),
            ],
            references: vec![],
            content_hash: None, // Merge is a structural distinction
        };
        
        // Record the merge
        self.causal_graph.add_merge(&merge_distinction);
        
        MergeResult {
            merge_point: merge_distinction.id,
            branches: vec![
                self.causal_graph.frontier(),
                other.frontier(),
            ],
        }
    }
}
```

---

## Implementation Details

### Storage Layout

```
~/.korudelta/
├── genesis/                    # The genome
│   └── genome.dist             # Minimal self-recreation info
├── hot/                        # Working memory (may be ephemeral)
│   └── (in-memory only)
├── warm/                       # Recent chronicle
│   └── chronicle.wal
├── cold/                       # Consolidated epochs
│   ├── epoch_0001/
│   │   ├── essence.dist        # Essential structure
│   │   ├── patterns.idx        # Pattern index
│   │   └── compressed.chr      # Compressed chronicle
│   └── epoch_0002/
├── deep/                       # Genomic archive
│   └── archive/
└── content/                    # Content-addressed store
    └── values/
```

### Rust Implementation Sketch

```rust
// The main system
pub struct KoruDelta {
    /// The distinction engine (heart)
    engine: Arc<DistinctionEngine>,
    
    /// Memory layers
    hot_memory: HotMemory,
    warm_memory: WarmMemory,
    cold_memory: ColdMemory,
    deep_memory: DeepMemory,
    
    /// Evolutionary processes
    rhythm: SystemRhythm,
    consolidation: ConsolidationProcess,
    distillation: DistillationProcess,
    genome_update: GenomeUpdateProcess,
    
    /// World reconciliation
    reconciliation: WorldReconciliation,
}

impl KoruDelta {
    /// The main lifecycle
    pub async fn run(&mut self) {
        // Start rhythm tasks
        tokio::spawn(self.consolidation_task());
        tokio::spawn(self.distillation_task());
        tokio::spawn(self.genome_update_task());
        
        // Main loop: accept syntheses
        loop {
            let operation = self.receive_operation().await;
            let synthesis = self.synthesize(operation);
            self.hot_memory.accept(synthesis).await;
        }
    }
    
    async fn consolidation_task(&self) {
        loop {
            tokio::time::sleep(self.rhythm.consolidation_interval).await;
            self.consolidation.consolidate(self).await;
        }
    }
    
    async fn distillation_task(&self) {
        loop {
            tokio::time::sleep(self.rhythm.archival_interval).await;
            self.distillation.distill(self).await;
        }
    }
    
    async fn genome_update_task(&self) {
        loop {
            tokio::time::sleep(self.rhythm.genome_update_interval).await;
            self.genome_update.update(self).await;
        }
    }
}
```

---

## API Design

### User API

```rust
// High-level API (familiar but powered by distinctions)
impl KoruDelta {
    /// Store value (creates synthesis)
    pub async fn put(&self, ns: &str, key: &str, value: Value) -> Result<DistinctionId>;
    
    /// Get current value
    pub async fn get(&self, ns: &str, key: &str) -> Result<Value>;
    
    /// Get value at specific time (time travel)
    pub async fn get_at(&self, ns: &str, key: &str, time: Timestamp) -> Result<Value>;
    
    /// Get history (causal chain)
    pub async fn history(&self, ns: &str, key: &str) -> Result<Vec<Distinction>>;
    
    /// Query (traverse distinction space)
    pub async fn query(&self, ns: &str, query: Query) -> Result<QueryResult>;
}
```

### Distinction API

```rust
// Low-level distinction operations
impl KoruDelta {
    /// Get distinction by ID
    pub async fn get_distinction(&self, id: DistinctionId) -> Result<Distinction>;
    
    /// Get causal ancestors
    pub async fn ancestors(&self, id: DistinctionId) -> Result<Vec<Distinction>>;
    
    /// Get causal descendants
    pub async fn descendants(&self, id: DistinctionId) -> Result<Vec<Distinction>>;
    
    /// Get references (what this points to)
    pub async fn references(&self, id: DistinctionId) -> Result<Vec<Distinction>>;
    
    /// Get referrers (what points to this)
    pub async fn referrers(&self, id: DistinctionId) -> Result<Vec<Distinction>>;
    
    /// Extract genome from current state
    pub async fn extract_genome(&self) -> Result<Genome>;
    
    /// Express genome (recreate state)
    pub async fn express_genome(&self, genome: &Genome) -> Result<()>;
}
```

### Administration API

```rust
impl KoruDelta {
    /// Trigger consolidation
    pub async fn consolidate(&self) -> Result<()>;
    
    /// Trigger distillation
    pub async fn distill(&self) -> Result<DistillationReport>;
    
    /// Update genome
    pub async fn update_genome(&self) -> Result<Genome>;
    
    /// Reconcile with peer
    pub async fn reconcile(&self, peer_addr: &str) -> Result<ReconciliationReport>;
    
    /// Get system statistics
    pub async fn stats(&self) -> Result<SystemStats>;
}
```

---

## Future Directions

### 1. Quantum Distinctions
Explore probabilistic distinctions for uncertain data.

### 2. Neural Distinctions
Use the distinction graph as a neural network substrate—learning as pattern recognition.

### 3. Distributed Genome
Sharded genomes for planetary-scale systems.

### 4. Distinction Mining
Discover emergent patterns in the causal graph.

### 5. Temporal Distinctions
Distinguish across time itself—past and future as distinguishable dimensions.

---

## Conclusion

KoruDelta v2.0 is not just a database—it's a **living system** that:

1. **Recognizes distinctions** (the what-ness of things)
2. **Remembers causally** (how things came to be)
3. **Forgets gracefully** (preserves essence, archives noise)
4. **Reconciles worlds** (syncs via distinction exchange)
5. **Reproduces** (genome-based replication)

**The distinction engine is the heart.** Everything flows from it.

---

*"The distinction is the atom of thought. KoruDelta is the molecule of memory."*
