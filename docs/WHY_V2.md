# Why Evolve KoruDelta?

> **The Short Answer:** We're capturing what emerges from koru-lambda-core to unlock capabilities that ad-hoc patterns can't achieve.

---

## The User Experience

### The Goal

**More capable internally** → **simpler for users**

How? By building the complexity that *belongs* in the system so users don't have to deal with it.

### Current Limitations (To Address)

```bash
# Database growing forever?
$ du -sh ~/.korudelta/
10G    # Needs automatic distillation

# Compaction?
$ kdelta compact --retain 30d
# (Should happen automatically, intelligently)

# Auth?
# (Needs capability-based, not bolted-on)

# Sync?
$ kdelta sync --target node2
# (Should only send what's needed)

# Raspberry Pi?
$ kdelta start
# (Should manage memory automatically)
```

### After (Evolved)

```bash
# Database size?
$ du -sh ~/.korudelta/
500M   # Automatic distillation

# Compaction?
# (Natural selection of distinctions)

# Auth?
$ kdelta auth init
$ kdelta auth add-user alice
# (Capability graph, zero config)

# Sync?
$ kdelta start --join node2
# (Set reconciliation, instant)

# Raspberry Pi?
$ kdelta start
# (Tiered memory, runs anywhere)
```

---

## The Five Principles (Emerging from Use)

### 1. Everything is a Distinction (Captured)

**What it means:** Every piece of information—data, user, capability, configuration—is a "distinction" with identity, causality, and relationships.

**Why you care:**
- No separate auth system (users are distinctions)
- No separate audit log (every action is a distinction)
- No separate backup system (genome extraction)
- Everything uses the same primitives

**The experience:**
```rust
// Data
db.put("users", "alice", data).await?;

// Auth (same API!)
db.grant("alice", "read:users").await?;

// Both are just distinctions in the graph
```

---

### 2. Causality is Primary

**What it means:** Every change has causal parents. The system knows not just *what* changed, but *why* (what caused it).

**Why you care:**
- Time travel actually works (causal, not just timestamps)
- Merges are intelligent (find common ancestor)
- Conflicts become branches (not errors)
- Full audit trail for free

**The experience:**
```bash
# What was the state at 2pm?
$ kdelta get users/alice --at "2026-02-05T14:00:00Z"
{ "name": "Alice" }

# Why did it change?
$ kdelta trace users/alice
Commit by: bob
Reason: "Updated contact info"
Previous: users/alice@v47
```

---

### 3. Memory is Layered (Like a Brain)

**What it means:** Hot (working), Warm (recent), Cold (consolidated), Deep (genomic). Data moves automatically based on access patterns.

**Why you care:**
- Bounded RAM regardless of database size
- Frequently accessed data is fast
- Old data is compressed but available
- System runs on tiny devices

**The experience:**
```bash
# 1 million keys
$ kdelta status
Keys: 1,000,000
Memory: 64MB (hot) + 256MB (warm index)
Disk: 2GB (cold) + 50MB (deep/genome)

# Same speed as 100 keys
$ kdelta get users/alice
(10ms)
```

---

### 4. The System is Self-Managing

**What it means:** Compaction, retention, and optimization happen automatically through "natural selection" of distinctions.

**Why you care:**
- No manual compaction
- No retention policy configuration
- No performance tuning
- Database just... stays healthy

**The experience:**
```bash
# Write 1 million versions
$ for i in {1..1000000}; do
    kdelta set counter/value $i
done

# Database size?
$ du -sh ~/.korudelta/
150M  # Distilled to essence

# Old versions?
$ kdelta log counter/value | wc -l
100   # Kept 100 most significant
```

---

### 5. Simplicity Through Depth

**What it means:** The internal architecture is sophisticated (causal graphs, distinction calculus, layered memory) so the user experience can be simple.

**Why you care:**
- Zero configuration
- No tuning needed
- Intuitive operations
- "It just works"

**The experience:**
```rust
// Complex distributed system, simple API
let db = KoruDelta::start().await?;  // Zero config

db.put("users", "alice", data).await?;  // Local
// (Automatically synced to cluster)

let value = db.get("users", "alice").await?;  // Fast
// (From hot memory if recent)

let old = db.get_at("users", "alice", yesterday).await?;  // Time travel
// (Traverses causal graph)

// Auth? Built in.
// Compaction? Automatic.
// Sync? Automatic.
```

---

## The Technical-to-User Mapping

| Technical Feature | User Benefit |
|-------------------|--------------|
| Causal Graph | Time travel just works |
| Reference Graph | Automatic memory management |
| Distinction Calculus | Unified model (no separate auth, audit, etc) |
| Hot/Warm/Cold/Deep | Runs on anything, any scale |
| Distillation | Never manually compact |
| Set Reconciliation | Instant sync |
| Genome | Portable backups |
| Capability Graph | Zero-config auth |

---

## The Comparison

| Feature | Traditional DB | KoruDelta (Current) | KoruDelta (Evolved) |
|---------|---------------|----------------|----------------|
| **Setup** | Complex | Simple | Zero config |
| **Time Travel** | No | Yes | Yes + causal trace |
| **Compaction** | Manual | Manual | Automatic |
| **Sync** | Complex | Full copy | Set reconciliation |
| **Auth** | Separate system | Basic | Capability graph |
| **Scale** | Limited | Memory-bound | Unlimited tiers |
| **Backup** | Dump/restore | Full snapshot | Genome (1KB) |
| **Magic** | None | Some | Lots |

---

## The Bottom Line

**The evolution is worth it because:**

1. **Users never think about:**
   - Compaction
   - Memory limits
   - Auth complexity
   - Sync tuning
   - Backup strategies

2. **Users just do:**
   - Store data
   - Query data
   - Sync data
   - Control access

3. **The system handles:**
   - Everything else

**That's the revolution.**

---

## FAQ

### Q: Why evolve rather than patch?
**A:** The current architecture can't support intelligent compaction, optimal sync, or unified auth without the causal and reference tracking. The distinction calculus provides the foundation.

### Q: Is this backward compatible?
**A:** Yes. Existing WAL files can be imported. The API remains the same.

### Q: How long until completion?
**A:** ~8 weeks. The foundation (causal graph, reference graph) is already done and tested.

### Q: Will this be slower?
**A:** No. Hot memory ensures recent data is fast. Benchmarks show comparable or better performance.

### Q: Is this just academic?
**A:** No. Every feature solves real user problems: unbounded growth, slow sync, auth complexity, memory limits.

---

*Simplicity is the ultimate sophistication. — Leonardo da Vinci*
