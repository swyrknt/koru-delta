# LCA Architecture for JavaScript

This guide explains how the Local Causal Agent (LCA) architecture works in the JavaScript/WASM bindings.

## Overview

KoruDelta for JavaScript uses the same LCA architecture as the Rust core, compiled to WebAssembly. This means:

- All operations follow the synthesis formula: `ΔNew = ΔLocal ⊕ ΔAction`
- Content-addressed storage with automatic deduplication
- Complete causal history for time-travel queries
- Vector embeddings for semantic search

## Architecture Flow

```
JavaScript Code
      ↓
WASM Bindings (wasm-bindgen)
      ↓
KoruDelta Core (Rust)
      ↓
Distinction Engine (koru-lambda-core)
```

## The Synthesis Formula

Every operation in KoruDelta follows:

```
ΔNew = ΔLocal_Root ⊕ ΔAction_Data
```

Where:
- **ΔLocal_Root**: The agent's current causal perspective
- **ΔAction_Data**: The canonicalized action being performed
- **⊕**: Synthesis (XOR-based content addressing)
- **ΔNew**: The new causal state

### Example: Storing Data

When you call:

```javascript
await db.put('users', 'alice', { name: 'Alice' });
```

The following happens:

1. A `StorageAction::Store` is created with the data
2. The action is serialized to canonical bytes
3. Bytes are folded through synthesis from the Storage root
4. The result becomes the new Storage agent root
5. The value is stored with its distinction ID (content hash)

## Content Addressing

Every value is identified by its SHA256 hash:

```javascript
// These two stores create the same distinction ID
await db.put('ns1', 'key1', { name: 'Alice' });
await db.put('ns2', 'key2', { name: 'Alice' });

// Both point to the same underlying storage (deduplication)
```

The distinction ID is computed as:
1. Serialize value to canonical JSON
2. Compute SHA256 hash
3. The hash is the distinction_id

## Agent Roots

All operations anchor to canonical roots:

| Root | Purpose |
|------|---------|
| FIELD | The unified field |
| STORAGE | Memory operations |
| TEMPERATURE | Activity tracking |
| CHRONICLE | Recent history |
| ARCHIVE | Long-term storage |
| ESSENCE | Causal topology |
| SLEEP | Consolidation |
| EVOLUTION | Selection |
| LINEAGE | Ancestry |
| PERSPECTIVE | Views |
| IDENTITY | Authentication |
| NETWORK | Distribution |
| VECTOR | Embeddings |

## In-Memory vs Persistent

### In-Memory (Default)

```javascript
const db = await KoruDeltaWasm.new();
// Data exists only for the session
// Fast, no IndexedDB overhead
```

### Persistent (Browser Only)

```javascript
const db = await KoruDeltaWasm.newPersistent();
// Data persists across sessions using IndexedDB
// Slight performance overhead
```

## Vector Search in WASM

The WASM build includes full vector search capabilities:

```javascript
// Automatic embedding from text
await db.putSimilar('docs', 'article1', 'Machine learning transforms software');

// Semantic search
const results = await db.findSimilar('docs', 'AI programming', 5);
```

The embedding is computed deterministically from the text content using the distinction calculus - no external ML models required.

## Thread Safety

Since JavaScript is single-threaded, the WASM bindings don't need the same Arc-based sharing as Rust. However, the internal Rust code uses the same thread-safe structures for consistency.

## Bundle Size

- **WASM binary**: ~8MB uncompressed
- **Compressed**: ~2-3MB (gzip)
- **Parsed**: ~100-200ms on modern devices

## Performance Considerations

1. **First Load**: WASM must be downloaded and compiled
2. **Subsequent Loads**: IndexedDB caching helps (for persistent mode)
3. **Operations**: Sub-millisecond for most queries
4. **Vector Search**: Depends on index size, typically <10ms

## Error Handling

All errors are thrown as JavaScript exceptions:

```javascript
try {
  const value = await db.get('users', 'nonexistent');
} catch (e) {
  console.error(e.message); // Key not found
}
```

Common errors:
- Key not found
- Invalid data (non-serializable)
- Invalid timestamp format
- Out of memory

## Best Practices

1. **Use batch operations** for multiple inserts
2. **Leverage semantic search** for content discovery
3. **Use workspaces** for isolation
4. **Enable persistence** for long-lived data
5. **Handle errors** gracefully

## Browser Compatibility

- Chrome/Edge: Full support
- Firefox: Full support
- Safari: Full support (14+)
- Node.js: 16+ with WASM support

## Further Reading

- [Main KoruDelta Docs](https://github.com/swyrknt/koru-delta)
- [API Reference](./API.md)
- [Rust Architecture](https://github.com/swyrknt/koru-delta/blob/main/ARCHITECTURE.md)
