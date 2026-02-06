# Phase 8: Production Hardening & Validation Plan

**Status:** Ready to begin  
**Goal:** Zero gaps. It just works.  
**Success:** Real local usage validates every feature.

---

## Overview

Phase 8 is not about new features. It's about ensuring everything we built actually works in the real world. This means:

1. **Crash Recovery** - Data survives power loss
2. **Resource Safety** - Memory/disk don't grow unbounded  
3. **Real Usage** - Actually install it, use it, find the gaps
4. **Multi-Node** - Cluster setup works without hand-holding
5. **Performance** - Meets benchmarks under load
6. **Security** - Auth is bulletproof
7. **Platforms** - Works everywhere we claim

---

## Phase 8.1: Crash Recovery & Durability

**Goal:** Data survives any crash.

### 8.1.1 Write-Ahead Logging (WAL)
- All mutations logged to `wal/` directory before applying
- WAL is append-only, checksummed
- Async flush with fsync on critical paths

### 8.1.2 Recovery on Startup
- Detect unclean shutdown (lock file + state marker)
- Replay WAL from last checkpoint
- Verify integrity before serving requests

### 8.1.3 Corruption Handling
- Checksums on all writes (Blake3)
- Detect corrupt segments on read
- Reconstruct from replicas if available
- Otherwise: mark bad data, log error, continue

### 8.1.4 Atomicity
- Multi-key operations: all succeed or all fail
- Implement with WAL + temporary staging

**Tests:**
- `test_crash_recovery` - kill -9 mid-write, verify data integrity
- `test_corruption_detection` - flip bits, verify error handling
- `test_atomic_write` - multi-key, verify all-or-nothing

---

## Phase 8.2: Resource Limits & Safety

**Goal:** System stays stable under any load.

### 8.2.1 Memory Caps
```rust
pub struct ResourceLimits {
    pub max_memory_mb: usize,      // Default: 512
    pub max_disk_gb: usize,        // Default: 10
    pub max_open_files: usize,     // Default: 256
    pub max_concurrent_connections: usize, // Default: 100
}
```

- Hot memory bounded by LRU (already done)
- Warm memory: max entries, evict to cold when full
- Total RSS tracked, backpressure when nearing limit

### 8.2.2 Disk Limits
- Track total DB size
- Auto-trigger distillation when > 80% of limit
- Reject writes when > 95% (graceful error: "disk limit reached")

### 8.2.3 Network Timeouts
- Peer connection timeout: 5s
- Peer read timeout: 30s
- Unresponsive peer? Mark stale, retry with backoff

### 8.2.4 Backpressure
- Memory full? Slow down writes (return `RetryLater`)
- Disk full? Stop accepting writes until compaction
- Too many connections? Reject with clear error

**Tests:**
- `test_memory_limit_enforced` - fill memory, verify eviction
- `test_disk_limit_enforced` - fill disk, verify rejection
- `test_timeout_handling` - unresponsive peer, verify recovery

---

## Phase 8.3: Error Handling Hardening

**Goal:** Every error is handled gracefully with clear messages.

### 8.3.1 No Panics
- Audit all `unwrap()`, `expect()`, `panic!()`
- Convert to proper `Result` types
- Panic only on unrecoverable internal invariant violation

### 8.3.2 Error Messages
```rust
// Bad
Err(DeltaError::IoError(e))

// Good
Err(DeltaError::StorageCorrupted {
    file: "data/epoch_3.json",
    offset: 1024,
    suggestion: "Run with --repair to attempt recovery",
})
```

### 8.3.3 Logging
- `tracing` crate for structured logging
- Levels: ERROR, WARN, INFO, DEBUG, TRACE
- Configurable via env: `KORU_LOG=info`

### 8.3.4 Error Recovery Paths
Every error type needs a recovery strategy:
- `IoError` → retry with backoff, then fail
- `SerializationError` → log bad data, skip record
- `AuthError` → return 401 with clear message
- `SyncError` → mark peer stale, retry later

**Tests:**
- `test_all_error_paths` - inject errors, verify handling
- `test_error_messages` - verify clarity and actionability

---

## Phase 8.4: Local Installation & Real Usage

**Goal:** Actually use the damn thing.

### 8.4.1 Installation
```bash
git clone https://github.com/swyrknt/koru-delta.git
cd koru-delta
cargo install --path .
kdelta --version  # Should show 2.0.0
```

### 8.4.2 First-Run Experience
```bash
# Terminal 1
kdelta start
# → Server started on :7878
# → Data directory: ~/.korudelta
# → Press Ctrl+C to stop

# Terminal 2
kdelta set users/alice '{"name": "Alice", "role": "admin"}'
# → OK: version abc123

kdelta get users/alice
# → {"name": "Alice", "role": "admin", "_version": "abc123"}

kdelta history users/alice
# → abc123 | 2026-02-06T09:14:10Z | initial

kdelta query --filter "role=admin"
# → users/alice: {"name": "Alice", "role": "admin"}
```

### 8.4.3 CLI Validation Script
Create `scripts/validate_cli.sh`:
```bash
#!/bin/bash
set -e

echo "=== KoruDelta CLI Validation ==="

# Start server
echo "Starting server..."
kdelta start &
SERVER_PID=$!
sleep 2

# Test basic operations
echo "Testing put/get..."
kdelta set test/key1 '{"value": 1}'
RESULT=$(kdelta get test/key1)
[[ "$RESULT" == *"value\"":1* ]] || exit 1

# Test history
echo "Testing history..."
kdelta set test/key1 '{"value": 2}'
HISTORY=$(kdelta history test/key1)
[[ $(echo "$HISTORY" | wc -l) -eq 2 ]] || exit 1

# Test query
echo "Testing query..."
kdelta set users/alice '{"name": "Alice"}'
kdelta set users/bob '{"name": "Bob"}'
QUERY=$(kdelta query --filter "namespace=users")
[[ $(echo "$QUERY" | grep -c "users/") -eq 2 ]] || exit 1

# Stop server
echo "Stopping server..."
kill $SERVER_PID

echo "=== All CLI tests passed ==="
```

### 8.4.4 Data Survivability
```bash
kdelta start &
kdelta set important/data '{"critical": true}'
kill %1  # Hard kill
kdelta start &
kdelta get important/data
# → Should return {"critical": true}
```

### 8.4.5 Large Dataset Test
```bash
# Create 10k keys
echo "Creating 10k keys..."
for i in $(seq 1 10000); do
    kdelta set "loadtest/key$i" "{\"n\": $i}"
done

# Query all
echo "Querying..."
kdelta query --filter "namespace=loadtest" | wc -l
# → Should be 10000

# Check memory
echo "Memory usage:"
ps aux | grep kdelta | grep -v grep
```

---

## Phase 8.5: Multi-Node Cluster Validation

**Goal:** Cluster setup works without hand-holding.

### 8.5.1 Two-Node Setup
```bash
# Terminal 1
kdelta start --port 7878 --data ~/.korudelta/node1

# Terminal 2  
kdelta start --port 7879 --data ~/.korudelta/node2 --join localhost:7878
```

### 8.5.2 Replication Test
```bash
# Write to node 1
kdelta --url http://localhost:7878 set shared/key '{"from": "node1"}'

# Read from node 2 (should replicate)
sleep 1
kdelta --url http://localhost:7879 get shared/key
# → {"from": "node1"}
```

### 8.5.3 Conflict Resolution
```bash
# Partition: both nodes think they're alone
# Write to both
kdelta --url http://localhost:7878 set conflict/key '{"node": 1}'
kdelta --url http://localhost:7879 set conflict/key '{"node": 2}'

# Heal partition
# Both should agree on value (LWW or causal merge)
```

### 8.5.4 Failure Handling
```bash
# Kill node 2
# Node 1 continues working
# Start node 2 again
# Should sync missed writes and rejoin
```

**Tests:**
- `test_two_node_cluster` - setup, write, replicate
- `test_conflict_resolution` - concurrent writes, verify merge
- `test_failure_recovery` - node death and rejoin
- `test_partition_healing` - split brain resolution

---

## Phase 8.6: Performance Validation

**Goal:** Meets benchmarks under real load.

### Benchmarks
| Metric | Target | Test |
|--------|--------|------|
| Write throughput | 10k/sec sustained | `bench_write_10k` |
| Read throughput (hot) | 50k/sec | `bench_read_hot` |
| Read throughput (cold) | 1k/sec | `bench_read_cold` |
| Startup (100k keys) | < 1s | `bench_startup` |
| Sync (1k diff) | < 100ms | `bench_sync` |
| Memory (10k active) | < 100MB RSS | `bench_memory` |
| Memory growth | Stable over time | `bench_memory_stability` |

### Load Testing
```bash
cargo run --example load_test -- --writes 100000 --reads 1000000 --concurrency 10
```

---

## Phase 8.7: Security Hardening

**Goal:** Auth is bulletproof.

### 8.7.1 End-to-End Auth Test
```bash
# Create identity
kdelta auth create-identity --name "test-user"
# → Identity: did:koru:abc123...
# → Secret: (saved to ~/.korudelta/secrets/test-user.key)

# Create capability
kdelta auth grant --to did:koru:grantee123 --resource "users/*" --permission read
# → Capability: cap_abc123...

# Use capability
kdelta --identity test-user get users/alice
# → OK (authorized)

kdelta --identity test-user set users/alice '{"test": true}'
# → UNAUTHORIZED (only has read)
```

### 8.7.2 HTTP Protection
```bash
# Without auth
curl http://localhost:7878/api/v1/users/alice
# → 401 Unauthorized

# With auth header
curl -H "Authorization: Bearer $CAPABILITY" http://localhost:7878/api/v1/users/alice
# → 200 OK
```

### 8.7.3 Expiration & Revocation
```bash
# Create expiring capability
kdelta auth grant --to did:koru:grantee --expires "1h"

# Wait 1 hour (or test with past date)
# Try to use
# → 401 Token expired

# Revoke
kdelta auth revoke --capability cap_abc123

# Try to use
# → 401 Token revoked
```

---

## Phase 8.8: Platform Testing

**Goal:** Works everywhere.

### 8.8.1 macOS
- [ ] Native build: `cargo build --release`
- [ ] Install: `cargo install --path .`
- [ ] Run: `kdelta start` → works
- [ ] All tests pass

### 8.8.2 Linux
- [ ] Native build
- [ ] Install
- [ ] Run
- [ ] All tests pass
- [ ] Container image builds

### 8.8.3 Windows
- [ ] Native build (MSVC)
- [ ] Install
- [ ] Run
- [ ] All tests pass

### 8.8.4 WASM
- [ ] `wasm-pack build` succeeds
- [ ] Runs in browser (basic put/get)

---

## Phase 8.9: Documentation

**Goal:** Complete, clear, accurate.

### Required Docs
- [ ] `README.md` - Quickstart (5 min to running)
- [ ] `API.md` - Every public function
- [ ] `CLI.md` - Every command with examples
- [ ] `ARCHITECTURE.md` - Deep dive (current, accurate)
- [ ] `TROUBLESHOOTING.md` - Common issues
- [ ] `CHANGELOG.md` - v1.0 → v2.0 changes
- [ ] `SECURITY.md` - Threat model, best practices

### README Quickstart
```markdown
# KoruDelta

Zero-config causal database.

## Quickstart

```bash
cargo install koru-delta
kdelta start
kdelta set users/alice '{"name": "Alice"}'
kdelta get users/alice
```

Done. Data is versioned, synced, and preserved forever.
```

---

## Phase 8.10: Final Validation

### Pre-Release Checklist
- [ ] Zero compiler warnings
- [ ] Zero clippy warnings
- [ ] All 321+ tests passing
- [ ] Test coverage > 80%
- [ ] No TODO/FIXME in code
- [ ] No unwrap() in production paths
- [ ] CHANGELOG.md updated
- [ ] Version = "2.0.0" in Cargo.toml
- [ ] Git tag v2.0.0

### Validation Day
Before release, spend a full day using it:

**Morning:**
- Install fresh
- Create database
- Write 1000 keys
- Query, filter, history
- Restart, verify data

**Afternoon:**
- Set up 3-node cluster
- Write to all nodes
- Kill one node, verify others work
- Bring node back, verify sync
- Fill disk, verify graceful handling

**Evening:**
- Enable auth
- Create identities
- Test capabilities
- Verify unauthorized access blocked
- Document any friction

### Release Criteria
**"It just works" checklist:**
1. ✅ Install with one command
2. ✅ Start with zero config
3. ✅ Put/get work immediately
4. ✅ Survives crashes without data loss
5. ✅ Memory stays bounded
6. ✅ Sync works between nodes
7. ✅ Auth optional but bulletproof
8. ✅ Performance is predictable
9. ✅ Errors are clear
10. ✅ Documentation answers everything

---

## Timeline

| Week | Focus | Deliverable |
|------|-------|-------------|
| 8.1 | Crash Recovery | WAL + recovery |
| 8.2 | Resource Limits | Bounded memory/disk |
| 8.3 | Error Handling | No panics, good errors |
| 8.4 | Local Usage | CLI validated |
| 8.5 | Cluster | Multi-node works |
| 8.6 | Performance | Benchmarks met |
| 8.7 | Security | Auth hardened |
| 8.8 | Platforms | All tested |
| 8.9 | Documentation | Complete docs |
| 8.10 | Validation Day | Release readiness |

**Total:** 10 weeks for bulletproof v2.0.0

---

## Success Metrics

Phase 8 is **DONE** when:

1. **I've used it for a full day** without hitting a bug
2. **All 10 "it just works" criteria** validated
3. **Zero panics** in any test scenario
4. **Crash recovery** works 100% of the time
5. **Resource limits** enforced perfectly
6. **Cluster** setup is copy-paste easy
7. **Performance** meets or exceeds targets
8. **Documentation** is complete and clear
9. **No gaps** - every feature production-ready
10. **Confident to show users**

---

*Phase 8: Making it real.*
