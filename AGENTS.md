# KoruDelta — Agent Development Guide

This document provides essential information for AI coding agents working on the KoruDelta project.

## Project Overview

**KoruDelta** is a zero-configuration causal database built in Rust that combines:
- **Git-like versioning** - Every change is tracked in an immutable history
- **Redis-like simplicity** - Simple key-value API with zero setup
- **Distributed consistency** - Multi-node clusters that sync automatically
- **Cross-platform** - Runs on Linux, macOS, Windows, and WASM (browsers/edge)

The project is built on top of [koru-lambda-core](https://github.com/swyrknt/koru-lambda-core), which provides a mathematical foundation via distinction calculus.

## Technology Stack

- **Language**: Rust (Edition 2021)
- **Async Runtime**: Tokio
- **Key Dependencies**:
  - `koru-lambda-core` - Distinction engine for content-addressed versioning
  - `serde` / `serde_json` - Serialization
  - `dashmap` - Lock-free concurrent hash maps
  - `clap` - CLI argument parsing
  - `axum` - HTTP API server
  - `chrono` - Time handling
  - `thiserror` / `anyhow` - Error handling

## Project Structure

```
koru-delta/
├── Cargo.toml            # Main package configuration
├── src/
│   ├── lib.rs            # Public API exports, crate root
│   ├── core.rs           # KoruDelta main database implementation
│   ├── storage.rs        # CausalStorage - versioned key-value storage
│   ├── mapper.rs         # DocumentMapper - JSON to distinction mapping
│   ├── types.rs          # Core data types (FullKey, VersionedValue, etc.)
│   ├── error.rs          # DeltaError and DeltaResult types
│   ├── query.rs          # Query engine (Filter, Query, Aggregation)
│   ├── views.rs          # Materialized views implementation
│   ├── subscriptions.rs  # Real-time change notifications
│   ├── cluster.rs        # Multi-node clustering
│   ├── network.rs        # TCP networking and peer protocol
│   ├── persistence.rs    # Disk persistence (JSON format)
│   ├── http.rs           # HTTP API server
│   ├── wasm.rs           # WebAssembly bindings
│   └── bin/kdelta.rs     # CLI binary implementation
├── tests/                # Integration tests
│   ├── integration_tests.rs
│   ├── cluster_tests.rs
│   ├── phase3_tests.rs
│   └── falsification_tests.rs
├── examples/             # Runnable demos
│   ├── ecommerce_demo.rs
│   └── cluster_demo.rs
├── benches/              # Performance benchmarks
│   └── core_operations.rs
├── .github/workflows/    # CI/CD configuration
│   └── ci.yml
├── README.md             # User-facing documentation
├── ARCHITECTURE.md       # Detailed technical architecture
├── DESIGN.md             # Design philosophy and principles
├── CLI_GUIDE.md          # Complete CLI reference
└── CONTRIBUTING.md       # Contribution guidelines
```

## Build Commands

```bash
# Build the project
cargo build

# Build for release (optimized)
cargo build --release

# Run all tests
cargo test
cargo test --release

# Run a specific test
cargo test test_basic_put_get_workflow

# Build and run examples
cargo build --examples
cargo run --example ecommerce_demo
cargo run --example cluster_demo

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open

# Install the CLI locally
cargo install --path .
```

## Code Style Guidelines

### Formatting and Linting

Always run these before committing:

```bash
cargo fmt           # Format code
cargo clippy        # Run linter
```

The CI will reject code that doesn't pass `cargo fmt --check` or `cargo clippy --all-targets --all-features -- -D warnings`.

### Naming Conventions

- **Modules**: `snake_case` (e.g., `causal_storage`)
- **Types/Structs/Enums**: `PascalCase` (e.g., `VersionedValue`)
- **Functions/Variables**: `snake_case` (e.g., `get_at`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_PORT`)

### Documentation Standards

Every public item must have documentation:

```rust
/// One-line summary in imperative mood (e.g., "Store a value").
///
/// Detailed explanation including:
/// - What the function does
/// - When to use it
/// - Thread safety notes
/// - Performance characteristics (Big O)
///
/// # Arguments
///
/// * `namespace` - The namespace for the key
/// * `key` - The key identifier
///
/// # Returns
///
/// Returns the versioned value that was stored.
///
/// # Errors
///
/// Returns `DeltaError::SerializationError` if the value cannot be serialized.
///
/// # Example
///
/// ```ignore
/// db.put("users", "alice", json!({"name": "Alice"})).await?;
/// ```
pub async fn put(...) -> DeltaResult<VersionedValue> { }
```

### Key Principles

1. **Simplicity over cleverness** - Straightforward code beats clever code
2. **Documentation first** - Every public item has docs
3. **Test everything** - Comprehensive unit + integration tests
4. **Hide complexity** - Mathematical concepts stay internal
5. **Thread-safe by default** - All structures support concurrent access

## Testing Instructions

### Test Organization

**Unit Tests**: Located inline with implementation (`#[cfg(test)] mod tests`):
- Test individual functions and edge cases
- Fast, isolated, deterministic

**Integration Tests**: Located in `tests/` directory:
- Test end-to-end workflows
- Test concurrency scenarios
- Test error conditions

### Test Naming

```rust
#[tokio::test]
async fn test_<feature>_<scenario>() {
    // Arrange
    let db = KoruDelta::start().await.unwrap();

    // Act
    let result = db.operation().await;

    // Assert
    assert_eq!(result, expected);
}
```

### Running Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_basic_put_get_workflow

# Tests matching pattern
cargo test test_time_travel

# Release mode (faster for large test suites)
cargo test --release
```

### Property-Based Testing

The project uses `proptest` for property-based testing (see `tests/falsification_tests.rs`). When adding new features, consider adding property-based tests to catch edge cases.

## Architecture Overview

KoruDelta is architected in layers:

```
┌─────────────────────────────────────────┐
│         KoruDelta Public API            │  ← Simple, async interface
│    (put, get, history, get_at)          │
├─────────────────────────────────────────┤
│          Query Layer                    │  ← Filter, sort, aggregate
├─────────────────────────────────────────┤
│          Views Layer                    │  ← Materialized views
├─────────────────────────────────────────┤
│       Subscriptions Layer               │  ← Real-time notifications
├─────────────────────────────────────────┤
│           Cluster Layer                 │  ← Multi-node clustering
├─────────────────────────────────────────┤
│        Causal Storage Layer             │  ← Versioning & history
├─────────────────────────────────────────┤
│      Distinction Engine (core)          │  ← Mathematical foundation
└─────────────────────────────────────────┘
```

### Key Data Types

- **`FullKey`** - Combines namespace + key: `"users:alice"`
- **`VersionedValue`** - Value with metadata (timestamp, version_id, previous_version)
- **`HistoryEntry`** - Simplified view for history queries
- **`DeltaError`** - Comprehensive error enum for all failure modes

### Concurrency Model

All core structures are thread-safe via:
- `DistinctionEngine` - Uses `DashMap` for lock-free synthesis
- `CausalStorage` - Uses `DashMap` for lock-free state updates
- `KoruDelta` - Uses `Arc` for cheap cloning across threads

## Platform Support

The codebase supports both native and WASM targets:

**Native (non-WASM)**: Full feature set including:
- Persistence to disk
- Network clustering
- HTTP API
- File system operations

**WASM**: Limited feature set:
- Core database operations (put, get, history)
- Query engine
- Views
- No persistence, networking, or subscriptions

Use conditional compilation for platform-specific code:

```rust
#[cfg(not(target_arch = "wasm32"))]
pub mod persistence;

#[cfg(feature = "wasm")]
pub mod wasm;
```

## CLI Binary

The `kdelta` CLI provides both local and remote operation modes:

```bash
# Local mode (direct database access)
kdelta set users/alice '{"name": "Alice"}'
kdelta get users/alice

# Server mode
kdelta start --port 7878

# Remote mode (via HTTP API)
kdelta --url http://localhost:7878 get users/alice
```

Key CLI implementation notes:
- Uses `clap` for argument parsing with derive macros
- Supports both local database operations and HTTP client mode
- Database path defaults to `~/.korudelta/db`
- JSON values must be valid (strings quoted: `'"value"'` not `'value'`)

## Security Considerations

1. **No authentication** - The current implementation has no built-in auth. In production, protect access via:
   - Network firewalls
   - Reverse proxy with authentication
   - VPN/private networks for clustering

2. **Data persistence** - Database files are stored as JSON:
   - Default location: `~/.korudelta/db`
   - File permissions are system defaults
   - No encryption at rest

3. **Network security** - Cluster communication:
   - Uses plain TCP (no TLS currently)
   - Suitable for trusted networks only
   - Consider VPN/tunneling for untrusted networks

4. **Input validation**:
   - All JSON inputs are validated before storage
   - Keys must follow `namespace/key` format
   - Filter expressions are parsed safely

## Common Development Tasks

### Adding a New Feature

1. **Design** - Update ARCHITECTURE.md with your design
2. **Tests** - Write tests before implementation (TDD)
3. **Implementation** - Add code following existing patterns
4. **Documentation** - Add comprehensive docs
5. **Verify** - Run `cargo fmt`, `cargo clippy`, `cargo test`

### Adding a New Public API Method

```rust
/// Brief description.
///
/// Detailed description...
///
/// # Example
///
/// ```ignore
/// let result = db.new_method().await?;
/// ```
pub async fn new_method(&self, ...) -> DeltaResult<T> {
    // Implementation
}
```

Don't forget to:
- Add to `lib.rs` exports if it should be public
- Add integration tests in `tests/`
- Update relevant documentation (README, CLI_GUIDE if applicable)

### Adding a New CLI Command

1. Add the command variant to `Commands` enum in `src/bin/kdelta.rs`
2. Add argument definitions with doc comments
3. Implement the command handler in the main match statement
4. Add to `handle_remote_command` if it should work via HTTP
5. Update `CLI_GUIDE.md` with documentation

## Performance Characteristics

Benchmarked performance (see `benches/core_operations.rs`):

| Operation | Performance |
|-----------|-------------|
| Read latency | ~340ns |
| Write throughput | ~27K ops/sec |
| History query | 4.3M elements/sec |

Complexity:
- `put()` - O(n) where n = bytes in value
- `get()` - O(1) hashmap lookup
- `get_at()` - O(h) where h = history size
- `history()` - O(h) where h = history size

## CI/CD Pipeline

The GitHub Actions workflow (`.github/workflows/ci.yml`) runs:

1. **Format check** - `cargo fmt -- --check`
2. **Clippy lint** - `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests** - `cargo test --release`
4. **Benchmark compile check** - `cargo bench --no-run`

Optional jobs test cross-platform compatibility (macOS, Windows) and run example demos.

## Useful References

- **ARCHITECTURE.md** - Detailed technical architecture and design decisions
- **DESIGN.md** - Product vision and core principles
- **CLI_GUIDE.md** - Complete CLI command reference
- **CONTRIBUTING.md** - Detailed contribution guidelines
- **README.md** - User-facing project overview

---

*When in doubt, prioritize simplicity and consistency with existing code patterns.*
