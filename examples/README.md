# KoruDelta JavaScript Examples

This directory contains example implementations for various JavaScript environments.

## Prerequisites

First, build the WASM bindings:

```bash
# From the project root
wasm-pack build --target web --out-dir bindings/js/pkg-web
wasm-pack build --target nodejs --out-dir bindings/js/pkg-nodejs
wasm-pack build --target bundler --out-dir bindings/js/pkg-bundler
```

## Examples

### Browser Example

Interactive HTML demo showing all KoruDelta features.

```bash
cd browser
# Serve with any static file server
python3 -m http.server 8080
# Or: npx serve .
```

Then open `http://localhost:8080`

### Node.js Example

Command-line example demonstrating database operations.

```bash
cd nodejs
npm install koru-delta
node index.js
```

### Cloudflare Worker Example

Edge-deployed API using KoruDelta.

```bash
cd cloudflare-worker
# Install wrangler
npm install -g wrangler

# Configure your account
wrangler login

# Deploy
wrangler deploy
```

### Deno Example

TypeScript example for Deno runtime.

```bash
cd deno
deno run --allow-read --allow-net index.ts
```

## Common Patterns

### Initialization

All examples follow this pattern:

```javascript
import { KoruDeltaWasm } from 'koru-delta';

// In browser, you need to initialize the WASM module first
import init from 'koru-delta';
await init();

// Create database instance
const db = await KoruDeltaWasm.new();
```

### Error Handling

```javascript
try {
    const result = await db.get('users', 'alice');
} catch (err) {
    if (err.message.includes('Key not found')) {
        // Handle missing key
    } else {
        // Handle other errors
    }
}
```

### Batch Operations

```javascript
// Store multiple items
const items = [
    { ns: 'users', key: 'alice', value: { name: 'Alice' } },
    { ns: 'users', key: 'bob', value: { name: 'Bob' } },
];

await Promise.all(items.map(item => 
    db.put(item.ns, item.key, item.value)
));
```

### Vector Similarity Search

```javascript
// Store documents with embeddings
await db.embed('docs', 'doc1', [0.1, 0.2, 0.3], 'model');
await db.embed('docs', 'doc2', [0.9, 0.8, 0.7], 'model');

// Search for similar documents
const query = [0.1, 0.2, 0.3]; // Query embedding
const results = await db.embedSearch('docs', query, 10);
// Returns: [{ key: 'doc1', score: 1.0 }, { key: 'doc2', score: 0.5 }]
```

## Environment Differences

| Feature | Browser | Node.js | Deno | Cloudflare |
|---------|---------|---------|------|------------|
| Persistence | None (memory) | None (memory) | None (memory) | None (memory) |
| Storage Limit | ~2GB | System RAM | System RAM | 128MB |
| Multi-threading | No | No | No | No |
| Startup Time | ~100ms | ~50ms | ~50ms | ~50ms |

## Troubleshooting

### WASM module not found

Make sure you've built the WASM package:
```bash
wasm-pack build --target <target>
```

### Initialization error in browser

You must call `init()` before using the database:
```javascript
import init, { KoruDeltaWasm } from 'koru-delta';
await init();
const db = await KoruDeltaWasm.new();
```

### Key not found errors

Always wrap `get()` in try-catch:
```javascript
try {
    const value = await db.get('ns', 'key');
} catch (e) {
    // Key doesn't exist
}
```

Or use `contains()` to check first:
```javascript
if (await db.contains('ns', 'key')) {
    const value = await db.get('ns', 'key');
}
```
