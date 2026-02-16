# KoruDelta for JavaScript

[![npm version](https://badge.fury.io/js/koru-delta.svg)](https://www.npmjs.com/package/koru-delta)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

> The causal database for browsers and edge computing.

KoruDelta brings content-addressed storage with time-travel queries directly to JavaScript environments. Like Git for your data, but designed for real-time applications.

## Features

- **🕐 Time-Travel Queries** - Ask "what was the value last Tuesday?"
- **🔗 Causal Consistency** - Content-addressed, automatic deduplication
- **⚡ Zero Configuration** - Works out of the box, no setup required
- **🌐 Browser & Edge** - Runs in browsers, Cloudflare Workers, Deno Deploy
- **🔒 Immutable History** - Complete audit trail of all changes
- **📦 Small Footprint** - ~8MB WASM binary, runs on resource-constrained devices

## Installation

```bash
npm install koru-delta
```

For bundlers (webpack, vite, rollup):
```javascript
import { KoruDeltaWasm } from 'koru-delta';
```

For Node.js:
```javascript
const { KoruDeltaWasm } = require('koru-delta');
```

## Quick Start

```javascript
import { KoruDeltaWasm } from 'koru-delta';

// Create a database
const db = await KoruDeltaWasm.new();

// Store data
await db.put('users', 'alice', { 
  name: 'Alice', 
  email: 'alice@example.com',
  role: 'admin'
});

// Retrieve current value
const user = await db.get('users', 'alice');
console.log(user.value.name); // 'Alice'
console.log(user.versionId);  // Unique version identifier
console.log(user.timestamp);  // ISO 8601 timestamp

// Update the user
await db.put('users', 'alice', { 
  name: 'Alice Smith', 
  email: 'alice@example.com',
  role: 'admin'
});

// Get history (time-travel!)
const history = await db.history('users', 'alice');
history.forEach(entry => {
  console.log(`${entry.timestamp}: ${entry.value.name}`);
});
// 2024-01-15T10:30:00Z: Alice Smith
// 2024-01-15T10:00:00Z: Alice

// Query past state
const lastWeek = await db.getAt(
  'users', 
  'alice', 
  '2024-01-08T10:00:00Z'
);
console.log(lastWeek.name); // 'Alice' (before the update)
```

## API Reference

### `KoruDeltaWasm.new()`

Creates a new database instance.

```javascript
const db = await KoruDeltaWasm.new();
```

### `db.put(namespace, key, value)`

Stores a value. Creates a new version if the key already exists.

```javascript
const versioned = await db.put('posts', 'hello', {
  title: 'Hello World',
  content: 'My first post!'
});
console.log(versioned.versionId); // Unique version identifier
```

### `db.get(namespace, key)`

Retrieves the current value.

```javascript
const versioned = await db.get('users', 'alice');
console.log(versioned.value);     // { name: 'Alice', ... }
console.log(versioned.timestamp); // When it was stored
console.log(versioned.versionId); // Unique identifier
```

### `db.history(namespace, key)`

Gets complete version history (newest first).

```javascript
const history = await db.history('users', 'alice');
// [
//   { value: {...}, timestamp: '...', versionId: '...' },
//   { value: {...}, timestamp: '...', versionId: '...' }
// ]
```

### `db.getAt(namespace, key, timestamp)`

Time-travel query: get value at a specific point in time.

```javascript
const value = await db.getAt(
  'users', 
  'alice', 
  '2024-01-10T09:00:00Z'
);
```

### `db.listNamespaces()`

List all namespaces.

```javascript
const namespaces = await db.listNamespaces();
// ['users', 'posts', 'config']
```

### `db.listKeys(namespace)`

List all keys in a namespace.

```javascript
const keys = await db.listKeys('users');
// ['alice', 'bob', 'charlie']
```

### `db.stats()`

Get database statistics.

```javascript
const stats = await db.stats();
console.log(stats.keyCount);        // Number of keys
console.log(stats.totalVersions);   // Total versions
console.log(stats.namespaceCount);  // Number of namespaces
```

## Use Cases

### Audit Logs

```javascript
// Every change is automatically versioned
await db.put('audit', 'transaction-123', {
  action: 'payment',
  amount: 100,
  user: 'alice'
});

// Complete audit trail
const audit = await db.history('audit', 'transaction-123');
```

### Undo/Redo

```javascript
class DocumentEditor {
  constructor(db, docId) {
    this.db = db;
    this.docId = docId;
    this.namespace = 'documents';
  }

  async save(content) {
    await this.db.put(this.namespace, this.docId, { content });
  }

  async undo() {
    const history = await this.db.history(this.namespace, this.docId);
    if (history.length > 1) {
      const previous = history[1]; // [0] is current, [1] is previous
      return previous.value;
    }
    return null;
  }
}
```

### Time-Travel Debugging

```javascript
// Store application state
await db.put('state', 'app', currentState);

// Later, debug what went wrong
const history = await db.history('state', 'app');
for (const entry of history) {
  if (hasBug(entry.value)) {
    console.log(`Bug introduced at ${entry.timestamp}`);
    break;
  }
}
```

## Browser Example

```html
<!DOCTYPE html>
<html>
<head>
  <script type="module">
    import { KoruDeltaWasm } from 'https://unpkg.com/koru-delta@latest';
    
    async function init() {
      const db = await KoruDeltaWasm.new();
      
      document.getElementById('save').onclick = async () => {
        const key = document.getElementById('key').value;
        const value = document.getElementById('value').value;
        await db.put('data', key, { content: value });
        
        const history = await db.history('data', key);
        document.getElementById('versions').innerHTML = 
          history.map(h => `<li>${h.timestamp}: ${h.value.content}</li>`).join('');
      };
    }
    
    init();
  </script>
</head>
<body>
  <input id="key" placeholder="Key" />
  <input id="value" placeholder="Value" />
  <button id="save">Save</button>
  <ul id="versions"></ul>
</body>
</html>
```

## Edge Runtime Example (Cloudflare Workers)

```javascript
import { KoruDeltaWasm } from 'koru-delta';

export default {
  async fetch(request, env, ctx) {
    const db = await KoruDeltaWasm.new();
    
    const url = new URL(request.url);
    const key = url.pathname.slice(1);
    
    if (request.method === 'PUT') {
      const value = await request.json();
      await db.put('kv', key, value);
      return new Response('OK');
    }
    
    if (request.method === 'GET') {
      try {
        const result = await db.get('kv', key);
        return Response.json(result.value);
      } catch (e) {
        return new Response('Not found', { status: 404 });
      }
    }
  }
};
```

## TypeScript

Full TypeScript support included:

```typescript
import { KoruDeltaWasm, VersionedValue, HistoryEntry } from 'koru-delta';

const db: KoruDeltaWasm = await KoruDeltaWasm.new();

const result: VersionedValue = await db.get('users', 'alice');
const history: HistoryEntry[] = await db.history('users', 'alice');
```

## Performance

- **Bundle size**: ~8MB WASM (compressed)
- **Memory usage**: Depends on data size (in-memory storage)
- **Query latency**: Sub-millisecond for most operations
- **Startup time**: ~100-200ms on modern devices

## Limitations

- **In-memory only**: Data persists only for the session (no filesystem in browser)
- **Single-user**: No built-in multi-user synchronization (use the Rust version for that)
- **WASM overhead**: First load requires downloading the WASM binary

## Documentation

- **[API Reference](./docs/API.md)** - Complete API documentation
- **[LCA Architecture](./docs/LCA_ARCHITECTURE.md)** - Understanding the Local Causal Agent pattern
- **[TypeScript Types](./index.d.ts)** - Complete type definitions

## Building from Source

```bash
git clone https://github.com/swyrknt/koru-delta.git
cd koru-delta/bindings/javascript
npm run build
```

Requires:
- Rust toolchain
- wasm-pack: `cargo install wasm-pack`

## License

Dual-licensed under MIT OR Apache-2.0

## Contributing

See the [main repository](https://github.com/swyrknt/koru-delta) for contribution guidelines.
