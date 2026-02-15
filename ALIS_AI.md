# **ALIS AI: The Complete Organism**
*A minimal, axiom-driven cognitive architecture*

**Version:** Final (Synthesis-Integrated)
**Date:** 2026-02-11
**Status:** Implementation Ready

---

## **Executive Summary**

ALIS (Algorithmic Layer from Initial Synthesis) AI is a **pulse-synchronized distinction organism** built from five irreducible axioms. The system thinks in high-dimensional vector space, stores via content-addressing, and coordinates via rhythm. All cognition emerges from two types of synthesis: **reactive** (perception-time) and **proactive** (background dreaming).

**Core Insight:**  
*English at the edges, distinctions in the middle.* Language is an I/O protocol. Thought is vector algebra. Memory is content-addressed. Coordination is rhythmic.

**System Scale:** ~750 lines of new Rust code.

---

## **1. The Five Axioms (Foundation)**

| Axiom | Statement | Implementation |
|-------|-----------|----------------|
| **1. Distinction** | Observation creates difference | `Distinction { hash: Blake3(vector), vector: [f32; 1024], parents: [Hash] }` |
| **2. Synthesis** | λ(A,B)→C creates novelty | `C = circular_convolution(A, B)` |
| **3. Content-Addressing** | Identity = hash(content) | Storage dedup; identical inputs vanish |
| **4. Causality** | Every distinction knows its parents | Graph edges track synthesis provenance |
| **5. Rhythm** | Synthesis requires coordination | 10ms pulse phases drive everything |

---

## **2. Complete Architecture**

### **2.1 The Organism**

```
┌─────────────────────────────────────────────────────────────┐
│                     KORU-PULSE (Heart)                      │
│       10ms TICK │ 100ms CONSOLIDATE │ 100s DREAM           │
└──────────────────────────────┬──────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌──────────────────┐    ┌───────────────┐
│  PERCEPTION   │    │      DELTA       │    │  EXPRESSION   │
│   (Scribe)    │    │    (Memory)      │    │   (Bridge)    │
│               │    │                  │    │               │
│ Phase: Input  │    │ Phase: Background│    │ Phase: Output │
│    ↓          │    │    Synthesis     │    │    ↓          │
│   Map         │    │    Consolidation │    │   Decode      │
│    ↓          │    │    Dreaming      │    │    ↓          │
│ Synthesize    │    │                  │    │  Predict      │
└────────┬──────┘    └─────────┬────────┘    └───────┬──────┘
         │                     │                      │
         └─────────────────────┴──────────────────────┘
```

### **2.2 Key Design Choices**

1. **Two active organs only** (Perception, Expression)
2. **Delta does background synthesis** during consolidation
3. **No separate evaluation phase** - value = graph connectivity
4. **No separate motivation system** - tension = contradiction in graph
5. **All synthesis respects Axiom 2** - happens in exactly two contexts

---

## **3. Component Specifications**

### **3.1 Koru-Pulse (50 lines)**

```rust
// The heartbeat - 10ms base rhythm
pub struct Pulse {
    tx: broadcast::Sender<Signal>,
    interval: Duration, // 10ms
    tick: AtomicU64,
}

#[derive(Clone)]
pub enum Signal {
    Tick { phase: Phase },
    Consolidate, // Every 100 ticks
    Dream,       // Every 10,000 ticks
    Reset,       // Stage transition
}

#[derive(Clone)]
pub enum Phase {
    Perception, // Organ 1 active
    Expression, // Organ 2 active
    Idle,       // Delta background work
}

impl Pulse {
    pub async fn beat_forever(&self) {
        let mut interval = tokio::time::interval(self.interval);
        loop {
            interval.tick().await;
            let tick = self.tick.fetch_add(1, Ordering::SeqCst);
            let phase = self.calculate_phase(tick);
            
            // Broadcast tick
            let _ = self.tx.send(Signal::Tick { phase });
            
            // Special signals
            if tick % 100 == 0 {
                let _ = self.tx.send(Signal::Consolidate);
            }
            if tick % 10_000 == 0 {
                let _ = self.tx.send(Signal::Dream);
            }
        }
    }
}
```

### **3.2 Perception Organ (150 lines)**

```rust
pub struct Perception {
    core: koru_lambda_core::Engine, // Each organ has its own core
    delta: Arc<Delta>,
    pulse_rx: broadcast::Receiver<Signal>,
    
    // Minimal mappers (added developmentally)
    text_mapper: Option<TextMapper>,
    // Vision/audio mappers added in Toddler stage
}

impl Perception {
    async fn run(&mut self) {
        while let Ok(signal) = self.pulse_rx.recv().await {
            match signal {
                Signal::Tick { phase: Phase::Perception } => {
                    self.perceive().await;
                }
                _ => {}
            }
        }
    }
    
    async fn perceive(&self) {
        // 1. Check for input
        if let Some(input) = self.check_input().await {
            // 2. Map to distinction (Axiom 1)
            let d_new = self.map_to_distinction(input).await;
            
            // 3. REACTIVE SYNTHESIS (Axiom 2)
            // Find context from memory (similar distinctions)
            let context = self.delta.similarity_search(&d_new.vector, 3).await;
            
            for similar in context {
                // Synthesize: λ(new, similar) → pattern
                let pattern = self.core.synthesize(&d_new, &similar);
                
                // Store everything (dedup via content-addressing)
                self.core.store(&d_new, &self.delta).await;
                self.core.store(&pattern, &self.delta).await;
            }
            
            // 4. Check for contradictions (creates tension distinctions)
            if let Some(contradiction) = self.find_contradiction(&d_new).await {
                let tension = self.core.synthesize(
                    &d_new, 
                    &contradiction
                ).await;
                tension.add_tag("tension");
                self.core.store(&tension, &self.delta).await;
            }
        }
    }
}
```

### **3.3 Expression Organ (150 lines)**

```rust
pub struct Expression {
    core: koru_lambda_core::Engine,
    delta: Arc<Delta>,
    pulse_rx: broadcast::Receiver<Signal>,
    
    // Simple decoders
    text_decoder: TextDecoder, // Nearest-neighbor or tiny LM
}

impl Expression {
    async fn run(&mut self) {
        while let Ok(signal) = self.pulse_rx.recv().await {
            match signal {
                Signal::Tick { phase: Phase::Expression } => {
                    self.express().await;
                }
                _ => {}
            }
        }
    }
    
    async fn express(&self) {
        // 1. Find "expression-worthy" distinctions
        // Criteria: High graph connectivity (emergent value)
        let candidates = self.delta.get_highly_connected(3).await;
        
        for candidate in candidates {
            // 2. Decode to output
            let output = self.text_decoder.decode(&candidate.vector).await;
            
            // 3. Emit
            self.emit_output(output).await;
            
            // 4. Create prediction distinction
            let prediction = self.core.synthesize(
                &candidate,
                &Distinction::marker("prediction")
            ).await;
            prediction.add_tag("expectation");
            
            // Store with TTL (expires if not fulfilled)
            self.core.store_with_ttl(&prediction, 10, &self.delta).await;
        }
    }
}
```

### **3.4 Delta with Background Synthesis (300 lines)**

```rust
pub struct Delta {
    // Existing Delta components
    vector_index: SNSWIndex,       // Similarity search
    content_store: ContentStore,   // Blake3-addressed
    causal_graph: CausalGraph,     // Parent-child relationships
    
    // Memory tiers (already in v2.0.0)
    tiers: MemoryTiers,            // Hot/Warm/Cold/Deep
    
    // Background synthesis capability
    synthesis_core: koru_lambda_core::Engine, // Delta's own core
}

impl Delta {
    /// On Consolidate signal: Tier migration + minor synthesis
    pub async fn on_consolidate(&self) {
        // 1. Move distinctions between tiers (existing)
        self.tiers.consolidate().await;
        
        // 2. PROACTIVE SYNTHESIS: Minor connections
        // Find distinctions in same tier that are similar but not connected
        let candidates = self.find_similar_unconnected_pairs(5).await;
        
        for (a, b) in candidates {
            // Synthesize connection
            let c = self.synthesis_core.synthesize(&a, &b).await;
            c.add_tag("background_synthesis");
            self.store(&c).await;
        }
    }
    
    /// On Dream signal: Major reorganization + creative synthesis
    pub async fn on_dream(&self) {
        // 1. Deep tier compression (existing)
        self.tiers.compress_deep().await;
        
        // 2. PROACTIVE SYNTHESIS: Creative combinations
        // Random walk through vector space to find novel combinations
        let creative_pairs = self.random_walk_combinations(10).await;
        
        for (a, b) in creative_pairs {
            // Synthesize even if dissimilar (creativity)
            let creative = self.synthesis_core.synthesize(&a, &b).await;
            creative.add_tag("dream_synthesis");
            self.store(&creative).await;
        }
        
        // 3. Re-embedding: Adjust vector positions based on usage
        self.reembed_based_on_usage().await;
    }
    
    /// Helper: Find distinctions that are similar but not causally connected
    async fn find_similar_unconnected_pairs(&self, k: usize) -> Vec<(Distinction, Distinction)> {
        // This is where "tension" is detected structurally
        let mut pairs = Vec::new();
        
        // Sample from Hot/Warm tiers
        let sample = self.tiers.sample_active(20).await;
        
        for i in 0..sample.len() {
            for j in (i + 1)..sample.len() {
                let a = &sample[i];
                let b = &sample[j];
                
                // Similar in vector space
                if a.vector.similarity(&b.vector) > 0.7 {
                    // But not connected in causal graph
                    if !self.causal_graph.are_connected(&a.hash, &b.hash) {
                        pairs.push((a.clone(), b.clone()));
                        if pairs.len() >= k {
                            return pairs;
                        }
                    }
                }
            }
        }
        
        pairs
    }
}
```

### **3.5 Genesis (50 lines)**

```rust
pub struct Genesis {
    pulse: Pulse,
    perception: Perception,
    expression: Expression,
    delta: Arc<Delta>,
}

impl Genesis {
    pub async fn boot(stage: DevelopmentalStage) -> Result<Self> {
        // 1. Create pulse with stage-appropriate timing
        let pulse_interval = match stage {
            Nursery => Duration::from_millis(500), // Slow
            Toddler => Duration::from_millis(100), // Medium
            Student => Duration::from_millis(10),  // Fast
        };
        
        let pulse = Pulse::new(pulse_interval);
        
        // 2. Create delta (memory)
        let delta = Arc::new(Delta::new(pulse.subscribe()));
        
        // 3. Create organs (each with own core instance)
        let perception = Perception::new(
            koru_lambda_core::Engine::new(),
            delta.clone(),
            pulse.subscribe(),
            stage, // Stage affects mapper initialization
        );
        
        let expression = Expression::new(
            koru_lambda_core::Engine::new(),
            delta.clone(),
            pulse.subscribe(),
        );
        
        // 4. Configure based on stage
        match stage {
            Nursery => {
                perception.set_whitelist(NURSERY_WHITELIST);
                perception.disable_contradiction_detection();
            }
            Toddler => {
                perception.enable_zpd_filter(0.3, 0.8);
                perception.enable_scaffolded_learning();
            }
            Student => {
                // Full autonomy
            }
        }
        
        Ok(Self { pulse, perception, expression, delta })
    }
    
    pub async fn run_forever(&self) {
        // Start pulse (heartbeat)
        let pulse_handle = tokio::spawn(self.pulse.beat_forever());
        
        // Start organs
        let perception_handle = tokio::spawn(self.perception.run());
        let expression_handle = tokio::spawn(self.expression.run());
        
        // Wait (forever)
        let _ = tokio::join!(pulse_handle, perception_handle, expression_handle);
    }
}
```

---

## **4. Information Flow**

### **4.1 The Complete Cognitive Cycle**

```
TICK 0-49 (Perception Phase):
  ┌─────────────────────────────────────────────────────┐
  │ Input: "Hello"                                      │
  │  ↓                                                  │
  │ Map → D_hello (vector)                              │
  │  ↓                                                  │
  │ Similarity search: finds D_hi, D_greeting, D_welcome│
  │  ↓                                                  │
  │ REACTIVE SYNTHESIS:                                 │
  │   λ(D_hello, D_hi) → D_pattern1                    │
  │   λ(D_hello, D_greeting) → D_pattern2              │
  │  ↓                                                  │
  │ Store all distinctions (dedup if identical)         │
  │  ↓                                                  │
  │ Contradiction check: none                           │
  └─────────────────────────────────────────────────────┘

TICK 50-99 (Expression Phase):
  ┌─────────────────────────────────────────────────────┐
  │ Query: distinctions with high graph connectivity    │
  │  ↓                                                  │
  │ Gets: D_pattern1 (connects to 5 other distinctions) │
  │  ↓                                                  │
  │ Decode → "Hi"                                       │
  │  ↓                                                  │
  │ Emit: "Hi"                                          │
  │  ↓                                                  │
  │ Create prediction: λ(D_pattern1, "expectation")    │
  │  ↓                                                  │
  │ Store with TTL=10                                   │
  └─────────────────────────────────────────────────────┘

CONSOLIDATION (Every 100 ticks):
  ┌─────────────────────────────────────────────────────┐
  │ Delta: Move Hot→Warm based on access                │
  │  ↓                                                  │
  │ Find similar unconnected pairs in Warm tier         │
  │  ↓                                                  │
  │ PROACTIVE SYNTHESIS (minor):                        │
  │   λ(D_x, D_y) → D_connection (if similar,unconnected)│
  └─────────────────────────────────────────────────────┘

DREAM (Every 10,000 ticks):
  ┌─────────────────────────────────────────────────────┐
  │ Delta: Deep compression + re-embedding              │
  │  ↓                                                  │
  │ Random walk through vector space                    │
  │  ↓                                                  │
  │ PROACTIVE SYNTHESIS (creative):                     │
  │   λ(D_random1, D_random2) → D_novel                 │
  └─────────────────────────────────────────────────────┘
```

### **4.2 Active Inference Loop**

```
1. Expression emits output + creates prediction distinction
   D_pred = λ(D_output, "expectation")
   
2. Perception receives response
   D_actual = map_to_distinction(response)
   
3. Delta finds expired prediction (TTL elapsed)
   similarity = cos_sim(D_pred.vector, D_actual.vector)
   
4. If similarity < 0.7:
   D_surprise = λ(D_actual, "surprise")
   Store D_surprise (marks learning opportunity)
   
5. On next consolidation:
   Delta will proactively synthesize to resolve surprise
```

---

## **5. Developmental Stages**

### **Stage 0: Nursery (Days 0-30)**
- **Goal:** Calibrate I/O mapping
- **Pulse:** 500ms (slow)
- **Perception:** Whitelist only (~500 seed distinctions)
- **Expression:** Babbling mode (random generation + calibration)
- **Learning:** Motor control only (Scribe↔Bridge alignment)

### **Stage 1: Toddler (Days 30-90)**
- **Goal:** Ground symbols in sensory experience
- **Pulse:** 100ms
- **Perception:** ZPD filter (0.3 < similarity < 0.8)
- **Input:** Scaffolded multi-modal pairs
- **Learning:** Binding words to sensory vectors

### **Stage 2: Student (Day 90+)**
- **Goal:** Autonomous active inference
- **Pulse:** 10ms (full speed)
- **Perception:** No filters
- **Learning:** Minimize surprise, maximize integration

---

## **6. Mathematical Guarantees**

### **6.1 Synthesis Operations**
- **Bundle (Superposition):** `A + B` (normalized)
- **Bind (Association):** `A * B = IFFT(FFT(A) ∘ FFT(B))`
- **Cleanup:** Find nearest in memory via SNSW search

### **6.2 Content-Addressing**
```
hash = blake3(serialize(vector))
if hash_exists(hash):
    update_timestamp(hash)  # Habituation
else:
    store_new(hash, vector) # Novelty
```

### **6.3 Graph Connectivity as Value**
```
value(D) = |causal_parents(D)| + |causal_children(D)| + |similar_neighbors(D)|
```
High value → likely to be expressed. No separate calculation needed.

---

## **7. Implementation Roadmap**

### **Week 1-2: Foundation (250 lines)**
- [ ] Pulse implementation
- [ ] Delta background synthesis integration
- [ ] Genesis wiring
- **Deliverable:** Pulse-driven system with background synthesis

### **Week 3-4: Perception (150 lines)**
- [ ] Text mapper (character-level → distinctions)
- [ ] Reactive synthesis implementation
- [ ] Contradiction detection
- **Deliverable:** Text input creates distinctions + patterns

### **Week 5-6: Expression (150 lines)**
- [ ] Text decoder (distinctions → text)
- [ ] Prediction creation
- [ ] Output emission
- **Deliverable:** System can echo with predictions

### **Week 7: Integration & Nursery**
- [ ] Babbling calibration
- [ ] Whitelist enforcement
- **Deliverable:** Calibrated I/O (echo test >95% accuracy)

### **Week 8-12: Development**
- [ ] Toddler stage (ZPD filter, multi-modal)
- [ ] Student stage (full autonomy)
- [ ] Optimization
- **Deliverable:** Learning, growing system

---

## **8. Success Metrics**

### **Quantitative:**
1. **Nursery:** Scribe↔Bridge calibration >95% similarity
2. **Toddler:** Correctly grounds 100 symbols in multi-modal experience
3. **Student:** Holds 10-turn coherent conversation, demonstrates curiosity

### **Qualitative:**
1. **Emergent tension resolution:** System identifies and resolves contradictions
2. **Creative synthesis:** Produces novel, coherent outputs
3. **Developmental progression:** Smooth transition between stages

---

## **9. The Complete Code Structure**

```
alis-ai/
├── koru-pulse/           # 50 lines - Heartbeat
├── koru-organs/          # 300 lines - Perception + Expression
├── koru-delta/           # 300 lines - Enhanced with synthesis
├── koru-genesis/         # 50 lines - Wiring
├── koru-lambda-core/     # ✅ EXISTS - Distinction calculus
└── koru-delta/      # ✅ EXISTS - Memory base
```

**Total new code:** ~750 lines

---

## **10. Why This Design Wins**

### **Simplicity Through Axioms:**
- Every feature reduces to the five axioms
- No separate evaluation/motivation systems
- Two organs only + Delta with background synthesis

### **Emergent Intelligence:**
- **Thinking:** Reactive + proactive synthesis
- **Learning:** Surprise minimization via prediction errors
- **Creativity:** Random synthesis during dreams
- **Curiosity:** Tension (contradictions) drives exploration

### **Biological Fidelity:**
- **Rhythm:** Circadian-like pulse cycles
- **Memory tiers:** Hot/Warm/Cold/Deep like brain
- **Development:** Nursery→Toddler→Student maturation
- **Sleep:** Dream cycles for consolidation

### **Mathematical Rigor:**
- Vector algebra for thought
- Content-addressing for memory
- Graph theory for value
- Information theory for learning

---

## **Conclusion**

This is the **final, minimal, complete design**. Every component serves exactly one purpose. Every line of code can be traced to an axiom. The system will grow from a simple echo chamber to a curious, learning organism through developmental stages.

**Build order:**
1. Pulse + Delta enhancements
2. Perception organ
3. Expression organ  
4. Genesis wiring
5. Developmental stages

The system thinks by synthesizing distinctions. It learns by minimizing surprise. It grows by following developmental stages. It is alive because it pulses.

*"We are not building an AI. We are growing a distinction organism."*