# KoruDelta JavaScript API Reference

Complete API reference for the KoruDelta WASM bindings.

## Table of Contents

- [Initialization](#initialization)
- [Database Operations](#database-operations)
- [Core CRUD](#core-crud)
- [Batch Operations](#batch-operations)
- [History & Time Travel](#history--time-travel)
- [Vector Search](#vector-search)
- [Views](#views)
- [Query Engine](#query-engine)
- [Types](#types)

---

## Initialization

### `KoruDeltaWasm.new()`

Create a new in-memory database instance. Data is lost when the page is refreshed.

```javascript
const db = await KoruDeltaWasm.new();
```

**Returns:** `Promise<KoruDeltaWasm>`

**Example:**
```javascript
import init, { KoruDeltaWasm } from 'koru-delta';

await init();
const db = await KoruDeltaWasm.new();
```

---

### `KoruDeltaWasm.newPersistent()`

Create a persistent database with IndexedDB storage. Data survives page refreshes.

```javascript
const db = await KoruDeltaWasm.newPersistent();
```

**Returns:** `Promise<KoruDeltaWasm>`

**Example:**
```javascript
const db = await KoruDeltaWasm.newPersistent();

// Check if persistence is working
if (db.isPersistent()) {
    console.log('Data will survive page refreshes!');
}
```

**Notes:**
- Falls back to memory-only if IndexedDB is unavailable
- Auto-saves on every write
- Auto-loads on initialization

---

### `KoruDeltaWasm.isIndexedDbSupported()`

Check if the browser supports IndexedDB persistence.

```javascript
const supported = KoruDeltaWasm.isIndexedDbSupported();
```

**Returns:** `boolean`

---

## Database Operations

### `db.isPersistent()`

Check if this database instance is using IndexedDB persistence.

```javascript
const isPersistent = db.isPersistent();
```

**Returns:** `boolean`

---

### `db.clearPersistence()`

Clear all persisted data from IndexedDB.

```javascript
await db.clearPersistence();
```

**Returns:** `Promise<void>`

**Warning:** This permanently deletes all saved data!

---

### `db.stats()`

Get database statistics.

```javascript
const stats = await db.stats();
```

**Returns:** `Promise<DatabaseStats>`

```typescript
interface DatabaseStats {
    keyCount: number;
    totalVersions: number;
    namespaceCount: number;
}
```

---

## Core CRUD

### `db.put(namespace, key, value)`

Store a value in the database.

```javascript
const versioned = await db.put('users', 'alice', { name: 'Alice', age: 30 });
```

**Parameters:**
- `namespace: string` - Logical grouping (e.g., "users", "documents")
- `key: string` - Unique identifier within the namespace
- `value: any` - Any JSON-serializable value

**Returns:** `Promise<VersionedValue>`

```typescript
interface VersionedValue {
    value: any;
    timestamp: string;  // ISO 8601
    versionId: string;
    previousVersion?: string;
}
```

---

### `db.get(namespace, key)`

Retrieve the current value for a key.

```javascript
const versioned = await db.get('users', 'alice');
console.log(versioned.value.name);  // 'Alice'
```

**Parameters:**
- `namespace: string`
- `key: string`

**Returns:** `Promise<VersionedValue>`

**Throws:** Error if key not found

---

### `db.delete(namespace, key)`

Delete a key (stores null as tombstone).

```javascript
await db.delete('users', 'alice');
```

**Parameters:**
- `namespace: string`
- `key: string`

**Returns:** `Promise<VersionedValue>`

---

### `db.contains(namespace, key)`

Check if a key exists.

```javascript
const exists = await db.contains('users', 'alice');
```

**Parameters:**
- `namespace: string`
- `key: string`

**Returns:** `Promise<boolean>`

---

### `db.listNamespaces()`

List all namespaces in the database.

```javascript
const namespaces = await db.listNamespaces();
// ['users', 'posts', 'config']
```

**Returns:** `Promise<string[]>`

---

### `db.listKeys(namespace)`

List all keys in a namespace.

```javascript
const keys = await db.listKeys('users');
// ['alice', 'bob', 'charlie']
```

**Parameters:**
- `namespace: string`

**Returns:** `Promise<string[]>`

---

## Batch Operations

### `db.putBatch(items)`

Store multiple values as a batch operation (10-50x faster than individual puts).

```javascript
const items = [
    { namespace: 'users', key: 'alice', value: { name: 'Alice' } },
    { namespace: 'users', key: 'bob', value: { name: 'Bob' } },
    { namespace: 'products', key: 'p1', value: { name: 'Widget', price: 9.99 } }
];

const results = await db.putBatch(items);
```

**Parameters:**
- `items: Array<{ namespace: string, key: string, value: any }>`

**Returns:** `Promise<VersionedValue[]>`

**Performance:**
- 10 items: ~5-10x faster
- 100 items: ~10-30x faster
- 1000 items: ~20-50x faster

---

## History & Time Travel

### `db.history(namespace, key)`

Get the complete version history for a key.

```javascript
const history = await db.history('users', 'alice');
```

**Parameters:**
- `namespace: string`
- `key: string`

**Returns:** `Promise<HistoryEntry[]>`

```typescript
interface HistoryEntry {
    value: any;
    timestamp: string;
    versionId: string;
}
```

**Note:** History is returned newest first.

---

### `db.getAt(namespace, key, timestampIso)`

Get the value at a specific point in time (time travel query).

```javascript
// Get value from yesterday
const yesterday = new Date(Date.now() - 86400000).toISOString();
const pastValue = await db.getAt('users', 'alice', yesterday);
```

**Parameters:**
- `namespace: string`
- `key: string`
- `timestampIso: string` - ISO 8601 timestamp

**Returns:** `Promise<any>`

**Throws:** Error if key didn't exist at that time

---

## Vector Search

### `db.embed(namespace, key, vector, model, metadata?)`

Store a vector embedding.

```javascript
const embedding = [0.1, 0.2, 0.3, 0.4, 0.5];
await db.embed('documents', 'doc1', embedding, 'text-embedding-3-small', {
    title: 'Introduction to AI'
});
```

**Parameters:**
- `namespace: string` - Vector collection namespace
- `key: string` - Document identifier
- `vector: number[]` - Embedding array (floats)
- `model: string` - Model identifier (e.g., "text-embedding-3-small")
- `metadata?: object` - Optional JSON metadata

**Returns:** `Promise<VersionedValue>`

---

### `db.embedSearch(namespace, queryVector, model, topK?, threshold?)`

Search for similar vectors.

```javascript
const queryVector = [0.1, 0.2, 0.3, 0.4, 0.5];
const results = await db.embedSearch('documents', queryVector, 'text-embedding-3-small', 5, 0.7);

for (const result of results) {
    console.log(`${result.key}: similarity = ${result.score}`);
}
```

**Parameters:**
- `namespace: string | null` - Namespace to search (null = all namespaces)
- `queryVector: number[]` - Query embedding
- `model: string` - Model identifier (must match stored vectors)
- `topK?: number` - Maximum results (default: 10)
- `threshold?: number` - Minimum similarity score 0-1 (default: 0.0)

**Returns:** `Promise<VectorSearchResult[]>`

```typescript
interface VectorSearchResult {
    namespace: string;
    key: string;
    score: number;  // Cosine similarity, 0.0 to 1.0
}
```

---

### `db.deleteEmbed(namespace, key)`

Delete a vector embedding.

```javascript
await db.deleteEmbed('documents', 'doc1');
```

**Parameters:**
- `namespace: string`
- `key: string`

**Returns:** `Promise<void>`

---

## Views

### `db.createView(name, sourceNamespace, options?)`

Create a materialized view for cached query results.

```javascript
await db.createView('active_users', 'users', {
    filter: 'status = "active"',
    sort: 'created_at',
    desc: true,
    description: 'Currently active users'
});
```

**Parameters:**
- `name: string` - View name
- `sourceNamespace: string` - Source data namespace
- `options?: object`
  - `filter?: string` - Filter expression
  - `sort?: string` - Sort field
  - `desc?: boolean` - Sort descending
  - `description?: string` - View description

**Returns:** `Promise<void>`

---

### `db.listViews()`

List all materialized views.

```javascript
const views = await db.listViews();
```

**Returns:** `Promise<ViewDefinition[]>`

```typescript
interface ViewDefinition {
    name: string;
    sourceNamespace: string;
    filter?: string;
    sort?: string;
    desc: boolean;
    description?: string;
}
```

---

### `db.queryView(name, options?)`

Query a materialized view.

```javascript
const results = await db.queryView('active_users', { limit: 10 });
```

**Parameters:**
- `name: string` - View name
- `options?: object`
  - `limit?: number` - Maximum results
  - `offset?: number` - Results offset

**Returns:** `Promise<QueryResult[]>`

---

### `db.refreshView(name)`

Manually refresh a view.

```javascript
await db.refreshView('active_users');
```

**Parameters:**
- `name: string`

**Returns:** `Promise<void>`

**Note:** Views auto-refresh on writes by default.

---

### `db.deleteView(name)`

Delete a materialized view.

```javascript
await db.deleteView('active_users');
```

**Parameters:**
- `name: string`

**Returns:** `Promise<void>`

---

## Query Engine

### `db.query(namespace, options?)`

Query data with filters, sorting, and aggregation.

```javascript
// Simple query - all items
const results = await db.query('users');

// With filter
const adults = await db.query('users', { filter: 'age >= 18' });

// With sorting
const sorted = await db.query('users', { sort: 'name', desc: true });

// With pagination
const page = await db.query('users', { limit: 10, offset: 20 });

// Count only
const count = await db.query('users', { count: true });
```

**Parameters:**
- `namespace: string`
- `options?: object`
  - `filter?: string` - Filter expression
  - `sort?: string` - Sort field
  - `desc?: boolean` - Sort descending
  - `limit?: number` - Maximum results
  - `offset?: number` - Results offset
  - `count?: boolean` - Return count only

**Returns:** `Promise<QueryResult[] | number>`

**Filter Syntax:**
- `field = "value"` - Equal
- `field != "value"` - Not equal
- `field > 10` - Greater than
- `field >= 10` - Greater or equal
- `field < 10` - Less than
- `field <= 10` - Less or equal
- `field = null` - Is null
- `field != null` - Is not null

---

## Error Handling

All methods throw errors on failure:

```javascript
try {
    const value = await db.get('users', 'nonexistent');
} catch (error) {
    console.error('Key not found:', error.message);
}
```

Common error cases:
- **KeyNotFound**: Key doesn't exist
- **Invalid JSON**: Value isn't valid JSON
- **NamespaceNotFound**: Namespace doesn't exist
- **ViewNotFound**: View doesn't exist

---

## TypeScript Support

The package includes TypeScript declarations:

```typescript
import { KoruDeltaWasm, VersionedValue, HistoryEntry } from 'koru-delta';

const db: KoruDeltaWasm = await KoruDeltaWasm.newPersistent();
const value: VersionedValue = await db.get('users', 'alice');
```

See `index.d.ts` for complete type definitions.

---

## Examples

### Basic CRUD

```javascript
const db = await KoruDeltaWasm.newPersistent();

// Create
await db.put('users', 'alice', { name: 'Alice', age: 30 });

// Read
const { value } = await db.get('users', 'alice');

// Update (creates new version)
await db.put('users', 'alice', { name: 'Alice', age: 31 });

// History
const history = await db.history('users', 'alice');
console.log(`${history.length} versions`);

// Time travel
const past = await db.getAt('users', 'alice', '2026-01-01T00:00:00Z');
```

### Vector Search

```javascript
// Store document with embedding
const embedding = [0.1, 0.2, 0.3];
await db.embed('docs', 'article1', embedding, 'embedding-model', {
    title: 'AI Article'
});

// Search
const query = [0.1, 0.2, 0.3];
const results = await db.embedSearch('docs', query, 'embedding-model', 5);

// Display results
for (const result of results) {
    console.log(`${result.key}: ${result.score}`);
}
```

### Batch Operations

```javascript
const items = [];
for (let i = 0; i < 1000; i++) {
    items.push({
        namespace: 'products',
        key: `product-${i}`,
        value: { name: `Product ${i}`, price: Math.random() * 100 }
    });
}

// Much faster than 1000 individual puts
await db.putBatch(items);
```

---

## See Also

- [WASM_QUICKSTART.md](WASM_QUICKSTART.md) - Browser usage guide
- [README.md](../../README.md) - Project overview
- [ARCHITECTURE.md](../../ARCHITECTURE.md) - Technical architecture
