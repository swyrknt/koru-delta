# KoruDelta WASM Quick Start

Get started with KoruDelta in the browser in 5 minutes.

## Installation

### Option 1: npm (Recommended)

```bash
npm install koru-delta
```

### Option 2: CDN (via unpkg)

```html
<script type="module">
  import init, { KoruDeltaWasm } from 'https://unpkg.com/koru-delta@latest/pkg/koru_delta.js';
  await init();
  // Use KoruDelta...
</script>
```

### Option 3: Build from Source

```bash
# Clone the repository
git clone https://github.com/swyrknt/koru-delta.git
cd koru-delta

# Build WASM package
wasm-pack build --target web --out-dir bindings/javascript/pkg

# Link locally
cd bindings/javascript/pkg
npm link

# Use in your project
cd /path/to/your/project
npm link koru-delta
```

---

## Basic Usage

### 1. Initialize the Module

```javascript
import init, { KoruDeltaWasm } from 'koru-delta';

// Must call init() before using any KoruDelta APIs
await init();
```

### 2. Create a Database

```javascript
// Option A: In-memory (data lost on refresh)
const db = await KoruDeltaWasm.new();

// Option B: Persistent (data survives refresh)
const db = await KoruDeltaWasm.newPersistent();
```

### 3. Store and Retrieve Data

```javascript
// Store data
await db.put('users', 'alice', {
    name: 'Alice',
    email: 'alice@example.com',
    age: 30
});

// Retrieve data
const { value, timestamp, versionId } = await db.get('users', 'alice');
console.log(value.name);  // 'Alice'
```

### 4. Work with History

```javascript
// Get all versions
const history = await db.history('users', 'alice');
console.log(`${history.length} versions`);

// Time travel - get value at specific time
const yesterday = new Date(Date.now() - 86400000).toISOString();
const pastValue = await db.getAt('users', 'alice', yesterday);
```

---

## Complete Example

```html
<!DOCTYPE html>
<html>
<head>
    <title>KoruDelta Demo</title>
</head>
<body>
    <h1>KoruDelta Browser Demo</h1>
    <div id="output"></div>

    <script type="module">
        import init, { KoruDeltaWasm } from 'koru-delta';

        async function main() {
            // Initialize WASM
            await init();

            // Create persistent database
            const db = await KoruDeltaWasm.newPersistent();

            // Store some data
            await db.put('users', 'alice', {
                name: 'Alice',
                role: 'admin',
                lastLogin: new Date().toISOString()
            });

            await db.put('users', 'bob', {
                name: 'Bob',
                role: 'user',
                lastLogin: new Date().toISOString()
            });

            // Retrieve and display
            const alice = await db.get('users', 'alice');
            const bob = await db.get('users', 'bob');

            document.getElementById('output').innerHTML = `
                <h2>Users</h2>
                <p><strong>Alice:</strong> ${alice.value.name} (${alice.value.role})</p>
                <p><strong>Bob:</strong> ${bob.value.name} (${bob.value.role})</p>
                <p><em>Data persists across page refreshes!</em></p>
            `;

            // Show persistence status
            console.log('Persistent:', db.isPersistent());
        }

        main().catch(console.error);
    </script>
</body>
</html>
```

---

## Batch Operations

For better performance with multiple writes:

```javascript
// Batch write is 10-50x faster than individual puts
const items = [
    { namespace: 'products', key: 'p1', value: { name: 'Widget', price: 9.99 } },
    { namespace: 'products', key: 'p2', value: { name: 'Gadget', price: 19.99 } },
    { namespace: 'products', key: 'p3', value: { name: 'Tool', price: 29.99 } }
];

const results = await db.putBatch(items);
console.log(`Stored ${results.length} items`);
```

---

## Vector Search (AI Embeddings)

Store and search vector embeddings:

```javascript
// Store embedding (e.g., from OpenAI API)
const embedding = [0.1, 0.2, 0.3, 0.4, 0.5];
await db.embed('documents', 'doc1', embedding, 'text-embedding-3-small', {
    title: 'Introduction to AI'
});

// Search for similar documents
const query = [0.1, 0.2, 0.3, 0.4, 0.5];
const results = await db.embedSearch('documents', query, 'text-embedding-3-small', 5);

// Display results
for (const result of results) {
    console.log(`${result.key}: similarity = ${result.score.toFixed(3)}`);
}
```

---

## Materialized Views

Create cached views for fast queries:

```javascript
// Create a view of active users
await db.createView('active_users', 'users', {
    filter: 'status = "active"',
    sort: 'created_at',
    desc: true
});

// Query the view (instant, cached results)
const activeUsers = await db.queryView('active_users');

// Views auto-refresh on writes, but can manually refresh
await db.refreshView('active_users');
```

---

## Query Engine

Filter, sort, and aggregate data:

```javascript
// Simple query
const allUsers = await db.query('users');

// With filter
const adults = await db.query('users', { filter: 'age >= 18' });

// With sorting
const sorted = await db.query('users', { sort: 'name', desc: true });

// Pagination
const page2 = await db.query('users', { limit: 10, offset: 10 });

// Count only
const count = await db.query('users', { count: true });
console.log(`${count} users`);
```

---

## Working with Multiple Tabs

When using `newPersistent()`, all tabs share the same IndexedDB database:

```javascript
// Tab 1
const db = await KoruDeltaWasm.newPersistent();
await db.put('shared', 'key1', { data: 'from tab 1' });

// Tab 2 (simultaneously)
const db2 = await KoruDeltaWasm.newPersistent();
const value = await db2.get('shared', 'key1');
console.log(value.data);  // 'from tab 1'
```

---

## Error Handling

```javascript
try {
    const value = await db.get('users', 'nonexistent');
} catch (error) {
    console.error('Error:', error.message);
    // Handle: Key not found
}

try {
    await db.put('users', 'key', 'not valid json');
} catch (error) {
    console.error('Error:', error.message);
    // Handle: Invalid JSON
}
```

---

## Performance Tips

### Use Batch Writes

```javascript
// ❌ Slow: 100 individual puts
for (let i = 0; i < 100; i++) {
    await db.put('items', `key${i}`, { index: i });
}

// ✅ Fast: Single batch put
const items = Array.from({ length: 100 }, (_, i) => ({
    namespace: 'items',
    key: `key${i}`,
    value: { index: i }
}));
await db.putBatch(items);
```

### Use Views for Repeated Queries

```javascript
// ❌ Slow: Full scan every time
const activeUsers = await db.query('users', { filter: 'status = "active"' });

// ✅ Fast: Cached view
await db.createView('active_users', 'users', { filter: 'status = "active"' });
const activeUsers = await db.queryView('active_users');
```

### Check Persistence Support

```javascript
if (KoruDeltaWasm.isIndexedDbSupported()) {
    const db = await KoruDeltaWasm.newPersistent();
} else {
    const db = await KoruDeltaWasm.new();
    console.warn('IndexedDB not supported, using memory-only mode');
}
```

---

## TypeScript Support

```typescript
import { 
    KoruDeltaWasm, 
    VersionedValue, 
    HistoryEntry,
    VectorSearchResult 
} from 'koru-delta';

const db: KoruDeltaWasm = await KoruDeltaWasm.newPersistent();
const value: VersionedValue = await db.get('users', 'alice');
```

---

## Framework Integration

### React Hook Example

```typescript
import { useEffect, useState } from 'react';
import init, { KoruDeltaWasm } from 'koru-delta';

function useKoruDelta() {
    const [db, setDb] = useState<KoruDeltaWasm | null>(null);

    useEffect(() => {
        async function initDb() {
            await init();
            const database = await KoruDeltaWasm.newPersistent();
            setDb(database);
        }
        initDb();
    }, []);

    return db;
}

// Usage
function UserProfile({ userId }: { userId: string }) {
    const db = useKoruDelta();
    const [user, setUser] = useState(null);

    useEffect(() => {
        if (!db) return;
        db.get('users', userId).then(result => setUser(result.value));
    }, [db, userId]);

    return user ? <div>{user.name}</div> : <div>Loading...</div>;
}
```

---

## Browser Compatibility

| Feature | Chrome | Firefox | Safari | Edge |
|---------|--------|---------|--------|------|
| Basic Operations | ✅ 90+ | ✅ 90+ | ✅ 14+ | ✅ 90+ |
| IndexedDB Persistence | ✅ 90+ | ✅ 90+ | ✅ 14+ | ✅ 90+ |
| Vector Search | ✅ 90+ | ✅ 90+ | ✅ 14+ | ✅ 90+ |

---

## Troubleshooting

### "Module not found" error

Make sure you're importing correctly:

```javascript
// ✅ Correct
import init, { KoruDeltaWasm } from 'koru-delta';

// ❌ Incorrect
import { KoruDeltaWasm } from 'koru-delta';
```

### "IndexedDB not working"

Check if IndexedDB is supported and not blocked:

```javascript
console.log('Supported:', KoruDeltaWasm.isIndexedDbSupported());
console.log('Persistent:', db.isPersistent());
```

### "Data not persisting"

Private/incognito browsing modes often disable IndexedDB. Use regular browsing mode for persistence.

### WASM Loading Issues

If using a bundler (Webpack, Vite, etc.), you may need to configure WASM loading:

**Vite:**
```javascript
// vite.config.js
export default {
    optimizeDeps: {
        exclude: ['koru-delta']
    }
};
```

**Webpack:**
```javascript
// webpack.config.js
module.exports = {
    experiments: {
        asyncWebAssembly: true
    }
};
```

---

## Next Steps

- **[JS_API.md](JS_API.md)** - Complete API reference
- **[ARCHITECTURE.md](../../ARCHITECTURE.md)** - Technical architecture
- **[Examples](../../examples/)** - More code examples

---

## License

MIT OR Apache-2.0
