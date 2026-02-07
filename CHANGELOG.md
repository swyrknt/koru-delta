# Changelog

All notable changes to KoruDelta will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-02-06

### Overview
Production-ready single-node causal database with memory tiering, crash recovery, and comprehensive observability.

### Added

#### Core Features
- **Memory Tiering**: Hot/Warm/Cold/Deep automatic memory management inspired by human brain
  - Hot: LRU cache for working set (configurable capacity)
  - Warm: Recent chronicle with promotion/demotion
  - Cold: Consolidated epochs with fitness filtering
  - Deep: Genomic storage for portable 1KB backups
  
- **Crash Recovery**: Write-ahead logging (WAL) with durability guarantees
  - Append-only WAL format with checksums (CRC32)
  - Corruption detection and graceful handling
  - Unclean shutdown detection via lock files
  - Automatic recovery on startup

- **Background Processes**: Self-managing evolutionary processes
  - Consolidation: Moves data between memory tiers (5 min interval)
  - Distillation: Fitness-based natural selection (1 hour interval)
  - GenomeUpdate: Extracts portable genome (daily interval)

- **Resource Limits**: Configurable bounds for production deployments
  - Memory caps (default: 512MB)
  - Disk limits (default: 10GB)
  - File descriptor limits (default: 256)
  - Connection limits (default: 100)

- **Structured Logging**: `tracing` integration for observability
  - Log levels: ERROR, WARN, INFO, DEBUG, TRACE
  - Configurable via `KORU_LOG` environment variable
  - Instrumented core operations

#### Performance
- Sub-microsecond reads from hot memory (~400ns)
- ~50µs writes with WAL persistence
- 20,000+ operations per second throughput
- Linear scalability for sequential operations

#### API
- `KoruDelta::start_with_path()` for persistence
- `put()` with automatic memory tier promotion
- `get()` with tiered lookup (Hot → Warm → Cold → Storage)
- `history()` with causal graph traversal
- `get_at()` for time-travel queries

#### CLI
- Full validation script (`scripts/validate_cli.sh`)
- All commands work with persistence
- Data survives process restarts
- Large value support (tested 2.6KB+)

### Changed

- **Storage Format**: Migrated to WAL-based persistence
- **Error Handling**: Audited and removed unwraps from production paths
- **Documentation**: Comprehensive updates across all docs

### Known Limitations

#### Multi-Node Clustering
- **Status**: Infrastructure exists, HTTP broadcast gap
- **Impact**: Single-node only for production use
- **Details**: 
  - Node discovery works
  - Initial sync on join works
  - Live replication via HTTP does not work
  - Fix planned for v2.1.0

#### Auth CLI
- **Status**: Auth module complete, CLI not integrated
- **Impact**: Auth available via Rust API only
- **Fix planned for v2.1.0

### Validation

- **321 tests passing** (0 failures)
- **0 compiler warnings**
- **Performance benchmarks validated**
- **CLI end-to-end tests passing**

### Documentation

- Updated README.md with current status
- Added PERFORMANCE_REPORT.md
- Added PHASE8_STATUS.md
- Added CLUSTER_SYNC_STATUS.md (gap documentation)
- Updated V2_TODO.md with accurate completion status

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
