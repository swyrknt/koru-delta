# KoruDelta JavaScript Bindings

The invisible database for JavaScript environments - browsers, Node.js, Deno, and edge workers.

## Features

- 🚀 **Zero Configuration** - Start using immediately
- 📝 **Versioned Data** - Complete history of every change
- 🔍 **Vector Search** - Built-in embedding similarity search
- 🌐 **Universal** - Runs anywhere JavaScript runs
- 📦 **Small** - ~2MB WASM binary

## Installation

```bash
npm install koru-delta
```

## Quick Start

```javascript
import { KoruDeltaWasm } from 'koru-delta';

// Initialize
const db = await KoruDeltaWasm.new();

// Store data
await db.put('users', 'alice', { name: 'Alice', age: 30 });

// Retrieve data
const user = await db.get('users', 'alice');
console.log(user.value); // { name: 'Alice', age: 30 }

// View history
const history = await db.history('users', 'alice');
console.log(history); // Array of all versions
```

## API Reference

### Basic Operations

#### `put(namespace, key, value)`
Store a value. Creates a new version.

```javascript
await db.put('products', 'laptop', {
    name: 'Gaming Laptop',
    price: 1299.99
});
```

#### `get(namespace, key)`
Retrieve the current value.

```javascript
const result = await db.get('products', 'laptop');
console.log(result.value);      // The stored value
console.log(result.timestamp);  // When it was stored
console.log(result.versionId);  // Unique version identifier
```

#### `delete(namespace, key)`
Delete a key.

```javascript
await db.delete('products', 'laptop');
```

#### `history(namespace, key)`
Get complete version history.

```javascript
const history = await db.history('users', 'alice');
history.forEach(entry => {
    console.log(entry.timestamp, entry.value);
});
```

#### `getAt(namespace, key, timestampISO)`
Time travel - get value at a specific time.

```javascript
const pastValue = await db.getAt(
    'users', 
    'alice', 
    '2024-01-01T00:00:00Z'
);
```

### Query Engine

#### `query(namespace, filter, limit)`
Query with filters.

```javascript
// Find users aged 30
const results = await db.query('users', { age: 30 }, 10);

// Results array contains: { key, value, timestamp, versionId }
results.forEach(record => {
    console.log(record.key, record.value);
});
```

### Vector Embeddings

#### `embed(namespace, key, vector, model?)`
Store a vector embedding.

```javascript
const embedding = [0.1, 0.2, 0.3, 0.4]; // Your vector data
await db.embed('documents', 'doc1', embedding, 'text-embedding-3-small');
```

#### `embedSearch(namespace, queryVector, limit?)`
Search for similar vectors.

```javascript
const query = [0.1, 0.2, 0.3, 0.4];
const results = await db.embedSearch('documents', query, 5);

results.forEach(result => {
    console.log(result.key, result.score); // score is similarity (higher = more similar)
});
```

#### `deleteEmbed(namespace, key)`
Delete an embedding.

```javascript
await db.deleteEmbed('documents', 'doc1');
```

### Views (Materialized Queries)

#### `createView(name, sourceNamespace)`
Create a materialized view.

```javascript
await db.createView('all-users', 'users');
```

#### `queryView(name)`
Query a view.

```javascript
const result = await db.queryView('all-users');
console.log(result.records);     // Array of records
console.log(result.totalCount);  // Total record count
console.log(result.aggregation); // Optional aggregation result
```

#### `listViews()`
List all views.

```javascript
const views = await db.listViews();
```

#### `refreshView(name)`
Refresh a view (update with latest data).

```javascript
await db.refreshView('all-users');
```

#### `deleteView(name)`
Delete a view.

```javascript
await db.deleteView('all-users');
```

### Utility

#### `listNamespaces()`
List all namespaces.

```javascript
const namespaces = await db.listNamespaces();
// ['users', 'products', ...]
```

#### `listKeys(namespace)`
List all keys in a namespace.

```javascript
const keys = await db.listKeys('users');
// ['alice', 'bob', ...]
```

#### `contains(namespace, key)`
Check if a key exists.

```javascript
const exists = await db.contains('users', 'alice');
```

#### `stats()`
Get database statistics.

```javascript
const stats = await db.stats();
console.log(stats.keyCount);        // Number of keys
console.log(stats.totalVersions);   // Total versions across all keys
console.log(stats.namespaceCount);  // Number of namespaces
```

## Platform-Specific Usage

### Browser (ES Modules)

```html
<script type="module">
    import init, { KoruDeltaWasm } from './koru_delta.js';
    
    await init();
    const db = await KoruDeltaWasm.new();
    // ... use db
</script>
```

### Node.js

```javascript
const { KoruDeltaWasm } = require('koru-delta');

async function main() {
    const db = await KoruDeltaWasm.new();
    // ... use db
}
```

### Deno

```typescript
import init, { KoruDeltaWasm } from "koru-delta";

await init();
const db = await KoruDeltaWasm.new();
// ... use db
```

### Cloudflare Workers

```javascript
import { KoruDeltaWasm } from 'koru-delta';

let db;
async function getDb() {
    if (!db) db = await KoruDeltaWasm.new();
    return db;
}

export default {
    async fetch(request) {
        const db = await getDb();
        // ... handle request
    }
};
```

## Building from Source

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for different targets
wasm-pack build --target web        # Browser
wasm-pack build --target nodejs     # Node.js
wasm-pack build --target bundler    # webpack/vite
```

## Examples

See the `/examples` directory for complete examples:

- `browser/` - Interactive HTML demo
- `nodejs/` - Node.js CLI example
- `cloudflare-worker/` - Edge worker API
- `deno/` - Deno TypeScript example

## License

MIT OR Apache-2.0
