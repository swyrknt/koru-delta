# Contributing to KoruDelta

Thank you for your interest in contributing to KoruDelta! This guide will help you understand the codebase and contribute effectively.

## Code Style

KoruDelta follows strict code style guidelines to maintain consistency and quality:

### Rust Style

- **Format**: Use `rustfmt` (run `cargo fmt` before committing)
- **Lint**: Use `clippy` (run `cargo clippy` to check)
- **Naming**: Follow Rust conventions
  - Types: `PascalCase`
  - Functions/variables: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`

### Documentation Style

Every public item must have documentation:

```rust
/// One-line summary in imperative mood (e.g., "Store a value").
///
/// Detailed explanation including:
/// - What the function does
/// - When to use it
/// - Important notes (thread safety, performance, etc.)
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

### Code Organization

- **One concept per module**: Each file should have a clear, single responsibility
- **Short functions**: Aim for < 50 lines per function
- **Clear naming**: Function names should describe what they do
- **Minimal dependencies**: Only add crates when truly needed

## Testing Standards

All code must be thoroughly tested:

### Unit Tests

Located in the same file as the implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_success_case() {
        // Arrange
        let input = setup_input();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_feature_error_case() {
        let result = function_under_test(invalid_input);
        assert!(matches!(result, Err(DeltaError::InvalidData { .. })));
    }
}
```

### Integration Tests

Located in `tests/` directory:

```rust
#[tokio::test]
async fn test_end_to_end_workflow() {
    let db = KoruDelta::start().await.unwrap();

    // Test complete workflow
    db.put("ns", "key", json!(1)).await.unwrap();
    let value = db.get("ns", "key").await.unwrap();
    assert_eq!(value, json!(1));
}
```

### Test Coverage Requirements

- **New features**: Must have tests covering success and error cases
- **Bug fixes**: Must include a regression test
- **Public APIs**: Must have integration tests
- **Edge cases**: Empty inputs, boundary conditions, concurrent access

## Development Workflow

### 1. Setup

```bash
# Clone the repository
git clone https://github.com/swyrknt/koru-delta.git
cd koru-delta

# Build the project
cargo build

# Run tests
cargo test
```

### 2. Making Changes

```bash
# Create a feature branch
git checkout -b feature/my-feature

# Make your changes
# ... edit files ...

# Format code
cargo fmt

# Check for issues
cargo clippy

# Run tests
cargo test

# Commit
git add .
git commit -m "Add feature: description"
```

### 3. Before Submitting

Checklist before creating a pull request:

- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation is complete
- [ ] New features have tests
- [ ] Examples are updated if needed
- [ ] ARCHITECTURE.md is updated if needed

## Project Structure

```
koru-delta/
├── src/
│   ├── lib.rs            # Public API exports
│   ├── core.rs           # KoruDelta main implementation
│   ├── storage.rs        # Causal storage layer
│   ├── mapper.rs         # Document mapping
│   ├── types.rs          # Data structures
│   ├── error.rs          # Error types
│   ├── query.rs          # Query engine (filter, sort, aggregate)
│   ├── views.rs          # Materialized views
│   ├── subscriptions.rs  # Real-time change notifications
│   ├── cluster.rs        # Multi-node clustering
│   ├── network.rs        # TCP networking and protocols
│   ├── persistence.rs    # Disk persistence
│   └── wasm.rs           # WebAssembly bindings
├── tests/                # Integration tests
├── examples/             # Runnable demo examples
├── benches/              # Performance benchmarks
├── DESIGN.md             # Design philosophy
├── ARCHITECTURE.md       # Technical architecture
├── README.md             # User documentation
├── CLI_GUIDE.md          # CLI reference
└── Cargo.toml            # Dependencies
```

## Adding New Features

### Step 1: Design

Before writing code:

1. Read [DESIGN.md](DESIGN.md) to understand the product vision
2. Read [ARCHITECTURE.md](ARCHITECTURE.md) to understand the structure
3. Write a design doc or open an issue describing your feature
4. Get feedback from maintainers

### Step 2: API Design

Design the API first:

```rust
// Good: Simple, intuitive, consistent with existing API
db.scan("users", |key, value| { ... }).await?;

// Bad: Too complex, inconsistent naming
db.iterate_namespace_with_callback("users", Box::new(|k, v| { ... }));
```

Principles:
- **Simplicity**: Can you explain it in one sentence?
- **Consistency**: Does it match existing patterns?
- **Future-proof**: Will it work with distribution and clustering?

### Step 3: Implementation

1. Write tests first (TDD)
2. Implement the feature
3. Ensure all tests pass
4. Add documentation
5. Update examples if needed

### Step 4: Review

1. Self-review your code
2. Check the contribution checklist
3. Submit a pull request
4. Address review feedback

## Common Patterns

### Error Handling

Always use `DeltaResult<T>` for fallible operations:

```rust
pub fn operation() -> DeltaResult<Value> {
    let data = get_data().map_err(|e| DeltaError::StorageError(e.to_string()))?;
    Ok(process(data))
}
```

### Async Functions

Use `async` for all public APIs, even if currently synchronous:

```rust
// Good: Future-proof for distribution
pub async fn get(&self, namespace: &str, key: &str) -> DeltaResult<JsonValue> {
    self.storage.get(namespace, key)
}

// Bad: Would require breaking change later
pub fn get(&self, namespace: &str, key: &str) -> DeltaResult<JsonValue> {
    self.storage.get(namespace, key)
}
```

### Thread Safety

Use `Arc` for shared ownership:

```rust
// Good: Cheap cloning, thread-safe
#[derive(Clone)]
pub struct KoruDelta {
    storage: Arc<CausalStorage>,
}

// Bad: Expensive cloning
pub struct KoruDelta {
    storage: CausalStorage,
}
```

## Documentation

### Module-Level Docs

Every module should have a top-level doc comment:

```rust
//! Brief module description.
//!
//! Detailed explanation of what this module provides,
//! key concepts, and how it fits into the overall architecture.
```

### Type Documentation

```rust
/// Brief description.
///
/// Detailed explanation including:
/// - Purpose and use cases
/// - Thread safety
/// - Examples
///
/// # Example
///
/// ```ignore
/// let thing = Thing::new();
/// ```
pub struct Thing { }
```

### Function Documentation

Required sections:
- Summary (first line)
- Detailed description
- Arguments (if any)
- Returns
- Errors (if fallible)
- Example

## Performance Considerations

When adding features, consider:

1. **Time complexity**: What's the Big O?
2. **Space complexity**: How much memory?
3. **Concurrency**: Safe under concurrent access?
4. **Caching**: Can we avoid recomputation?

Document performance characteristics in function docs:

```rust
/// Get value (O(1) lookup).
pub fn get() { }

/// Get history (O(n) where n = number of versions).
pub fn history() { }
```

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: Open a GitHub Issue with reproduction steps
- **Features**: Open an issue to discuss before implementing

## Code Review Process

Pull requests are reviewed for:

1. **Correctness**: Does it work? Are there tests?
2. **Style**: Follows code style guidelines?
3. **Design**: Is the API simple and intuitive?
4. **Documentation**: Are docs complete and clear?
5. **Performance**: Are there obvious inefficiencies?

## Release Process

(For maintainers)

1. Update version in `Cargo.toml`
2. Run full test suite: `cargo test --release`
3. Run benchmarks: `cargo bench`
4. Tag release: `git tag v0.x.0`
5. Push: `git push --tags`
6. Publish: `cargo publish`

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
