# KoruDelta WASM Runtime Abstraction Design

**Version**: 2.0.0  
**Status**: Design Phase  
**Date**: 2026-02-08  
**Author**: KoruDelta Team  

---

## Executive Summary

This document outlines a comprehensive runtime abstraction layer that enables KoruDelta to run seamlessly on both native platforms (using Tokio) and WebAssembly (using browser APIs). This approach eliminates the need for scattered `#[cfg]` conditional compilation, resulting in cleaner, more maintainable code.

### Goals

1. **Universal Binary**: Single codebase runs natively and in browsers
2. **Full Feature Parity**: All features work on both platforms
3. **Zero Runtime Cost**: Abstraction compiles away at build time
4. **Clean Architecture**: No platform-specific code scattered throughout

---

## The Problem

### Current State (Messy)

```rust
// Current approach - scattered conditionals
#[cfg(not(target_arch = "wasm32"))]
tokio::spawn(async move {
    // native code
});

#[cfg(target_arch = "wasm32")]
wasm_bindgen_futures::spawn_local(async move {
    // wasm code
});
```

**Issues:**
- ❌ `#[cfg]` attributes everywhere (50+ locations)
- ❌ Hard to test both paths
- ❌ Code duplication
- ❌ Easy to miss edge cases

### Desired State (Clean)

```rust
// Runtime abstraction - single code path
self.runtime.spawn(async move {
    // works everywhere
});
```

---

## Architecture

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Layer                           │
│         (CLI, HTTP API, JavaScript bindings)                     │
├─────────────────────────────────────────────────────────────────┤
│                      KoruDelta Core                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   Storage   │  │    Query    │  │     Causal Graph        │ │
│  │  ( causal ) │  │  ( filter ) │  │    ( distinctions )     │ │
│  └──────┬──────┘  └──────┬──────┘  └────────────┬────────────┘ │
│         │                │                       │              │
│         └────────────────┼───────────────────────┘              │
│                          │                                      │
│         ┌────────────────┴──────────────────┐                   │
│         │      Runtime Abstraction Layer     │                   │
│         │  ┌──────────────────────────────┐  │                   │
│         │  │    Runtime Trait Definition  │  │                   │
│         │  │                              │  │                   │
│         │  │  - spawn()                   │  │                   │
│         │  │  - sleep()                   │  │                   │
│         │  │  - interval()                │  │                   │
│         │  │  - timeout()                 │  │                   │
│         │  │  - channel()                 │  │                   │
│         │  │  - now()                     │  │                   │
│         │  └──────────────────────────────┘  │                   │
│         └────────────────────────────────────┘                   │
├─────────────────────────────────────────────────────────────────┤
│                      Runtime Implementations                     │
│  ┌────────────────────────┐      ┌────────────────────────┐     │
│  │    TokioRuntime        │      │     WasmRuntime        │     │
│  │  (native platforms)    │      │  (browser/edge)        │     │
│  │                        │      │                        │     │
│  │  tokio::spawn          │      │  spawn_local           │     │
│  │  tokio::time::sleep    │      │  js_sys::setTimeout    │     │
│  │  tokio::sync::mpsc     │      │  js_sys::Promise       │     │
│  │  std::time::Instant    │      │  js_sys::Date          │     │
│  └────────────────────────┘      └────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Runtime Trait Definition

### Core Trait

```rust
/// Platform-agnostic async runtime abstraction.
/// 
/// Implementations:
/// - `TokioRuntime`: Native platforms (Linux, macOS, Windows)
/// - `WasmRuntime`: WebAssembly (browsers, edge compute)
#[async_trait]
pub trait Runtime: Send + Sync + Clone + 'static {
    /// Spawn a new task.
    /// 
    /// On native: Uses tokio::spawn
    /// On WASM: Uses wasm_bindgen_futures::spawn_local
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    /// Sleep for a duration.
    /// 
    /// On native: tokio::time::sleep
    /// On WASM: js_sys::Promise with setTimeout
    async fn sleep(&self, duration: Duration);

    /// Create an interval stream.
    /// 
    /// On native: tokio::time::interval
    /// On WASM: Custom implementation with setInterval
    fn interval(&self, period: Duration) -> Interval;

    /// Create a channel.
    /// 
    /// On native: tokio::sync::mpsc
    /// On WASM: futures-channel or custom impl
    fn channel<T>(&self, capacity: usize) -> (Sender<T>, Receiver<T>);

    /// Get current time.
    /// 
    /// On native: std::time::Instant
    /// On WASM: js_sys::Date::now()
    fn now(&self) -> Instant;

    /// Create a timeout wrapper.
    /// 
    /// On native: tokio::time::timeout
    /// On WASM: Custom timeout via Promise.race
    async fn timeout<F>(&self, duration: Duration, future: F) -> Result<F::Output, TimeoutError>
    where
        F: Future;
}
```

### Supporting Types

```rust
/// Handle to a spawned task.
pub struct JoinHandle<T> {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::task::JoinHandle<T>,
    #[cfg(target_arch = "wasm32")]
    inner: futures::future::RemoteHandle<T>,
}

/// Interval stream for periodic tasks.
pub struct Interval {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::time::Interval,
    #[cfg(target_arch = "wasm32")]
    inner: WasmInterval,
}

/// Channel sender.
pub struct Sender<T> {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::sync::mpsc::Sender<T>,
    #[cfg(target_arch = "wasm32")]
    inner: futures::channel::mpsc::Sender<T>,
}

/// Channel receiver.
pub struct Receiver<T> {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::sync::mpsc::Receiver<T>,
    #[cfg(target_arch = "wasm32")]
    inner: futures::channel::mpsc::Receiver<T>,
}

/// Platform-agnostic instant.
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    inner: f64, // milliseconds since epoch
}
```

---

## Implementation Details

### TokioRuntime (Native)

```rust
#[derive(Clone)]
pub struct TokioRuntime;

impl TokioRuntime {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Runtime for TokioRuntime {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        JoinHandle {
            inner: tokio::spawn(future),
        }
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    fn interval(&self, period: Duration) -> Interval {
        Interval {
            inner: tokio::time::interval(period),
        }
    }

    fn channel<T>(&self, capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        (Sender { inner: tx }, Receiver { inner: rx })
    }

    fn now(&self) -> Instant {
        Instant {
            inner: std::time::Instant::now(),
        }
    }

    async fn timeout<F>(&self, duration: Duration, future: F) -> Result<F::Output, TimeoutError>
    where
        F: Future,
    {
        tokio::time::timeout(duration, future).await
            .map_err(|_| TimeoutError)
    }
}
```

### WasmRuntime (Browser)

```rust
#[derive(Clone)]
pub struct WasmRuntime;

impl WasmRuntime {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Runtime for WasmRuntime {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        // Use spawn_local for WASM (single-threaded)
        let (remote, handle) = future.remote_handle();
        wasm_bindgen_futures::spawn_local(remote);
        
        JoinHandle {
            inner: handle,
        }
    }

    async fn sleep(&self, duration: Duration) {
        let millis = duration.as_millis() as i32;
        
        // Create a Promise that resolves after the duration
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let window = web_sys::window().unwrap();
            let closure = Closure::once_into_js(move || {
                resolve.call0(&JsValue::UNDEFINED).unwrap();
            });
            window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                millis,
            ).unwrap();
        });
        
        // Convert Promise to Future
        JsFuture::from(promise).await.unwrap();
    }

    fn interval(&self, period: Duration) -> Interval {
        Interval {
            inner: WasmInterval::new(period),
        }
    }

    fn channel<T>(&self, capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (tx, rx) = futures::channel::mpsc::channel(capacity);
        (Sender { inner: tx }, Receiver { inner: rx })
    }

    fn now(&self) -> Instant {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        Instant {
            inner: performance.now(),
        }
    }

    async fn timeout<F>(&self, duration: Duration, future: F) -> Result<F::Output, TimeoutError>
    where
        F: Future,
    {
        let timeout_future = self.sleep(duration);
        
        match futures::future::select(Box::pin(future), Box::pin(timeout_future)).await {
            Either::Left((result, _)) => Ok(result),
            Either::Right(_) => Err(TimeoutError),
        }
    }
}
```

---

## Integration with KoruDelta

### Core Struct Changes

```rust
pub struct KoruDelta<R: Runtime> {
    /// The async runtime
    runtime: Arc<R>,
    
    /// Storage (runtime-agnostic)
    storage: Arc<CausalStorage>,
    
    /// Memory tiers (runtime-agnostic)
    hot: Arc<RwLock<HotMemory>>,
    warm: Arc<WarmMemory>,
    cold: Arc<ColdMemory>,
    deep: Arc<DeepMemory>,
    
    /// Lifecycle manager (uses runtime)
    lifecycle: Arc<LifecycleManager<R>>,
    
    /// Vector index (runtime-agnostic)
    vector_index: VectorIndex,
    
    /// Other fields...
    // ...
}
```

### Usage Example

```rust
impl<R: Runtime> KoruDelta<R> {
    pub async fn start_background_tasks(&self) {
        let runtime = Arc::clone(&self.runtime);
        let lifecycle = Arc::clone(&self.lifecycle);
        
        // Spawn works on both platforms!
        self.runtime.spawn(async move {
            let mut interval = runtime.interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                lifecycle.run_consolidation().await;
            }
        });
    }
}
```

---

## JavaScript API Surface

### Module Structure

```
bindings/javascript/
├── pkg/                          # WASM build output
│   ├── koru_delta.js            # JS glue code
│   ├── koru_delta_bg.wasm       # WASM binary
│   └── koru_delta.d.ts          # TypeScript definitions
├── src/                          # Source (if needed)
├── DESIGN.md                     # This document
├── package.json                  # NPM manifest
├── README.md                     # User documentation
└── examples/
    ├── browser.html              # Browser example
    ├── node.js                   # Node.js example
    └── cloudflare-worker.js      # Edge example
```

### JavaScript API

```typescript
// Main entry point
export class KoruDelta {
    /**
     * Create a new database instance.
     * 
     * In browser: Uses in-memory storage
     * In Node.js: Optional file persistence
     * In edge: Pure ephemeral
     */
    static new(): Promise<KoruDelta>;
    
    /**
     * Store a value.
     */
    put(namespace: string, key: string, value: any): Promise<VersionedValue>;
    
    /**
     * Retrieve current value.
     */
    get(namespace: string, key: string): Promise<VersionedValue>;
    
    /**
     * Get version history (time-travel).
     */
    history(namespace: string, key: string): Promise<HistoryEntry[]>;
    
    /**
     * Get value at specific time.
     */
    getAt(namespace: string, key: string, timestamp: string): Promise<any>;
    
    /**
     * Search similar vectors (if vector index enabled).
     */
    searchVectors(
        namespace: string, 
        query: number[], 
        options: SearchOptions
    ): Promise<VectorResult[]>;
    
    /**
     * Semantic navigation (SNSW feature).
     */
    synthesizeNavigate(
        startId: string,
        operations: NavigationOp[],
        k: number
    ): Promise<SearchResult[]>;
    
    /**
     * List namespaces.
     */
    listNamespaces(): Promise<string[]>;
    
    /**
     * List keys in namespace.
     */
    listKeys(namespace: string): Promise<string[]>;
    
    /**
     * Database statistics.
     */
    stats(): Promise<DatabaseStats>;
    
    /**
     * Export all data (for backup).
     */
    export(): Promise<Uint8Array>;
    
    /**
     * Import data (from backup).
     */
    import(data: Uint8Array): Promise<void>;
}

// TypeScript interfaces
export interface VersionedValue {
    value: any;
    timestamp: string;
    versionId: string;
    previousVersion?: string;
}

export interface HistoryEntry {
    value: any;
    timestamp: string;
    versionId: string;
}

export interface SearchOptions {
    topK?: number;
    threshold?: number;
}

export interface VectorResult {
    key: string;
    score: number;
    vector: number[];
}

export interface NavigationOp {
    type: 'add' | 'subtract' | 'toward';
    targetId: string;
    weight?: number;
}

export interface DatabaseStats {
    keyCount: number;
    totalVersions: number;
    namespaceCount: number;
    memoryBytes: number;
}
```

---

## Build Configuration

### Cargo.toml

```toml
[features]
default = ["native"]
native = ["tokio/full", "persistence", "clustering"]
wasm = [
    "wasm-bindgen",
    "wasm-bindgen-futures", 
    "js-sys",
    "web-sys",
    "console_error_panic_hook",
    "getrandom/js"
]

# Platform-specific dependencies
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Performance", "Window"] }
console_error_panic_hook = "0.1"
getrandom = { version = "0.2", features = ["js"] }
futures = "0.3"  # for channels on WASM

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.0", features = ["full"] }
```

### Build Scripts

```bash
# Build for native (default)
cargo build --release

# Build for browser (WASM)
wasm-pack build --target web --features wasm --no-default-features

# Build for Node.js (WASM)
wasm-pack build --target nodejs --features wasm --no-default-features

# Build for bundlers (webpack, vite, etc.)
wasm-pack build --target bundler --features wasm --no-default-features
```

### NPM Package Structure

```json
{
  "name": "koru-delta",
  "version": "2.0.0",
  "files": [
    "pkg/koru_delta.js",
    "pkg/koru_delta_bg.wasm",
    "pkg/koru_delta.d.ts"
  ],
  "main": "pkg/koru_delta.js",
  "types": "pkg/koru_delta.d.ts",
  "exports": {
    ".": {
      "browser": "./pkg/koru_delta.js",
      "node": "./pkg-node/koru_delta.js",
      "default": "./pkg/koru_delta.js"
    }
  }
}
```

---

## Migration Strategy

### Phase 1: Runtime Trait (Week 1)

1. Create `src/runtime/mod.rs` with trait definition
2. Implement `TokioRuntime`
3. Implement `WasmRuntime`
4. Add runtime parameter to `KoruDelta` struct

### Phase 2: Core Module Updates (Week 1-2)

1. Update `core.rs` to use `Runtime` instead of direct tokio calls
2. Update `lifecycle/mod.rs`
3. Update `views.rs` (background refresh)
4. Update `auth/manager.rs` (if needed)

### Phase 3: Feature Parity (Week 2)

1. Ensure clustering works on native
2. Disable clustering gracefully on WASM
3. Test all features on both platforms

### Phase 4: JavaScript Bindings (Week 3)

1. Update `wasm.rs` to use `WasmRuntime`
2. Generate TypeScript definitions
3. Create example projects
4. Write documentation

### Phase 5: Testing & Release (Week 4)

1. Comprehensive test suite
2. Performance benchmarks
3. Documentation review
4. v2.0.0 release

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test with Tokio runtime (native)
    #[tokio::test]
    async fn test_with_tokio() {
        let runtime = TokioRuntime::new();
        test_runtime_common(runtime).await;
    }
    
    // Test with mock runtime (for CI)
    #[test]
    fn test_with_mock() {
        let runtime = MockRuntime::new();
        // synchronous tests
    }
}

async fn test_runtime_common<R: Runtime>(runtime: R) {
    // Common test logic works for all runtimes
    let (tx, mut rx) = runtime.channel::<i32>(10);
    
    runtime.spawn(async move {
        tx.send(42).await.unwrap();
    });
    
    assert_eq!(rx.recv().await, Some(42));
}
```

### Integration Tests

```rust
// Test full database with different runtimes
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_native_database() {
    let runtime = TokioRuntime::new();
    let db = KoruDelta::with_runtime(runtime).await.unwrap();
    test_database_common(db).await;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_wasm_database() {
    let runtime = WasmRuntime::new();
    let db = KoruDelta::with_runtime(runtime).await.unwrap();
    test_database_common(db).await;
}
```

### JavaScript Tests

```javascript
// test.js
const { KoruDelta } = require('./pkg-node/koru_delta.js');

async function runTests() {
    const db = await KoruDelta.new();
    
    // Test put/get
    await db.put('test', 'key', { foo: 'bar' });
    const result = await db.get('test', 'key');
    console.assert(result.value.foo === 'bar');
    
    // Test history
    await db.put('test', 'key', { foo: 'baz' });
    const history = await db.history('test', 'key');
    console.assert(history.length === 2);
    
    console.log('✅ All tests passed');
}

runTests();
```

---

## Performance Considerations

### Native (Tokio)

| Metric | Expected |
|--------|----------|
| Task spawn | ~100ns |
| Channel send | ~50ns |
| Context switch | ~1µs |
| Timer resolution | ~1ms |

### WASM (Browser)

| Metric | Expected |
|--------|----------|
| Task spawn | ~1µs (spawn_local overhead) |
| setTimeout | ~4ms minimum (browser constraint) |
| Memory access | Same as JS (~2-3× native) |

### Optimizations

1. **Batch operations**: Reduce JS/WASM boundary crossings
2. **Use Uint8Array**: For binary data (zero-copy)
3. **Lazy loading**: Don't load full database at startup
4. **Web Workers**: For CPU-intensive operations

---

## Open Questions

### 1. Persistence in Browser

Options:
- IndexedDB (async, complex API)
- LocalStorage (sync, 5MB limit)
- OPFS (Origin Private File System - modern, promising)
- In-memory only (simplest, data lost on refresh)

**Recommendation**: Start with in-memory, add OPFS later.

### 2. Vector Search in WASM

SNSW requires significant computation. Options:
- Run in WASM (portable, slower)
- Use WebGL/GPU (fast, complex)
- Server-side (fastest, requires network)

**Recommendation**: WASM implementation first, optimize later.

### 3. Multi-tab Sync

If user has multiple tabs open:
- BroadcastChannel API (modern browsers)
- localStorage events (older support)
- No sync (simplest)

**Recommendation**: No sync for v2.0, add later.

---

## Success Criteria

| Criterion | Target |
|-----------|--------|
| Build time | < 2 minutes for WASM |
| Binary size | < 10MB WASM |
| Startup time | < 200ms in browser |
| Test coverage | > 90% |
| API compatibility | Same API native + WASM |
| Documentation | Complete JS examples |

---

## Conclusion

The Runtime trait abstraction provides a **clean, maintainable path** to full WASM support without sacrificing native performance. By abstracting the async runtime, we:

1. **Eliminate** scattered `#[cfg]` conditionals
2. **Enable** full feature parity across platforms
3. **Maintain** zero-cost abstraction
4. **Improve** testability with pluggable runtimes

This approach requires **4 weeks of focused development** but results in a professional, maintainable codebase that runs everywhere.

---

**Next Steps**: 
1. Review this design with team
2. Create proof-of-concept (runtime trait + one module)
3. Proceed with full implementation
