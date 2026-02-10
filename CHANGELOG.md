# Changelog

All notable changes to KoruDelta will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-02-09

### Overview
Production-ready causal database with complete feature set: materialized views, self-sovereign auth, vector search, real-time subscriptions, and WASM support.

### Added

#### Materialized Views
- Persistent view definitions with auto-refresh
- Views survive database restarts (stored in WAL)
- Query caching for instant results
- CLI: `kdelta view create/list/query/refresh/delete`

#### Self-Sovereign Authentication
- Proof-of-work identity mining
- Ed25519 cryptographic signatures
- Challenge-response authentication
- Capability-based authorization
- Identity persistence with verification

#### Vector Search
- Vector embedding storage with metadata
- Cosine similarity and Euclidean distance
- Namespaced vector collections
- Integration with query engine
- WASM support for browser-based AI

#### Real-time Subscriptions
- Pub/sub change notifications
- Filterable by collection, key, change type
- Broadcast-based delivery
- Non-blocking subscription management

#### WASM/Browser Support
- Full JavaScript API via wasm-bindgen
- IndexedDB persistence for data survival
- Auto-save and auto-load
- Graceful fallback to memory-only

#### Batch Write Operations
- `put_batch()` API for bulk writes (10-50x faster than individual puts)
- Single fsync for entire batch reduces I/O overhead
- Memory promotion and view refresh batched for efficiency
- JavaScript/WASM support via `putBatch()` method

#### Performance & Reliability
- Validated 200+ writes/sec throughput (single writes)
- Validated **16x improvement** with batch writes (1000 items: 280ms vs 4.66s)
- Validated 158K+ reads/sec
- 100+ version history depth
- 10,000+ key capacity tested
- Concurrent write safety (100 tasks, 0 conflicts)

#### CLI Enhancements
- View management commands
- Query engine with filters
- Remote HTTP operations
- Status and diagnostics

### Changed

- **View Persistence**: Views now persist to WAL (bug fix)
- **Documentation**: Complete README rewrite with validated performance

### Validation

- **425 tests passing** (0 failures)
- **0 compiler warnings** (clippy clean)
- **4 comprehensive E2E examples** all passing:
  - `crisis_coordination_demo` - Full feature showcase
  - `cluster_e2e_test` - Distributed mode validation
  - `stress_test` - Load & edge case validation
  - `batch_performance_demo` - Batch write performance validation
- **CLI commands verified** against documentation

### Documentation

- Complete README.md update with v2.0.0 features
- Validated performance numbers from stress testing
- Added comprehensive E2E examples
- Updated ARCHITECTURE.md and CLI_GUIDE.md

---

## [1.0.0] - 2024-11

### Initial Release
- Basic key-value storage
- Causal history tracking
- Time travel queries
- HTTP API
- CLI tool
- Cluster infrastructure (preview)

---

[2.0.0]: https://github.com/swyrknt/koru-delta/releases/tag/v2.0.0
[1.0.0]: https://github.com/swyrknt/koru-delta/releases/tag/v1.0.0
