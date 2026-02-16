# JavaScript API Reference

Complete API reference for KoruDelta JavaScript bindings v3.0.0 (LCA Architecture).

## Core Class: `KoruDeltaWasm`

The main database interface providing access to all KoruDelta functionality.

### Constructor

#### `KoruDeltaWasm.new()`

Creates a new database instance.

**Returns:** `Promise<KoruDeltaWasm>` - A new database instance

**Example:**
```javascript
const db = await KoruDeltaWasm.new();
```

#### `KoruDeltaWasm.newPersistent()`

Creates a new persistent database instance (browser-only, uses IndexedDB).

**Returns:** `Promise<KoruDeltaWasm>` - A new persistent database instance

**Example:**
```javascript
const db = await KoruDeltaWasm.newPersistent();
console.log(db.isPersistent()); // true
```

---

## Storage Operations

### `put(namespace, key, value)`

Store a value in the database.

**Parameters:**
- `namespace` (string): The namespace/collection name
- `key` (string): The key to store under
- `value` (any): JSON-serializable value to store

**Returns:** `Promise<VersionedValue>` - Version information for the stored value

**Example:**
```javascript
const versioned = await db.put('users', 'alice', {
  name: 'Alice',
  email: 'alice@example.com'
});
console.log(versioned.versionId);  // Unique version identifier
console.log(versioned.timestamp);  // ISO 8601 timestamp
```

### `get(namespace, key)`

Retrieve a value from the database.

**Parameters:**
- `namespace` (string): The namespace to retrieve from
- `key` (string): The key to retrieve

**Returns:** `Promise<VersionedValue>` - The stored value with metadata

**Throws:** Error if key not found

**Example:**
```javascript
const result = await db.get('users', 'alice');
console.log(result.value);      // { name: 'Alice', ... }
console.log(result.timestamp);  // When stored
console.log(result.versionId);  // Unique ID
```

### `delete(namespace, key)`

Delete a key from the database.

**Parameters:**
- `namespace` (string): The namespace
- `key` (string): The key to delete

**Returns:** `Promise<boolean>` - True if deleted, false if not found

**Example:**
```javascript
const wasDeleted = await db.delete('users', 'alice');
```

### `contains(namespace, key)`

Check if a key exists.

**Parameters:**
- `namespace` (string): The namespace
- `key` (string): The key to check

**Returns:** `Promise<boolean>` - True if exists

**Example:**
```javascript
if (await db.contains('users', 'alice')) {
  console.log('User exists!');
}
```

---

## History & Time Travel

### `history(namespace, key)`

Get complete version history for a key (newest first).

**Parameters:**
- `namespace` (string): The namespace
- `key` (string): The key to get history for

**Returns:** `Promise<HistoryEntry[]>` - Array of history entries

**Example:**
```javascript
const history = await db.history('users', 'alice');
history.forEach(entry => {
  console.log(`${entry.timestamp}:`, entry.value);
});
```

### `getAt(namespace, key, timestamp)`

Time-travel: get value at a specific point in time.

**Parameters:**
- `namespace` (string): The namespace
- `key` (string): The key to retrieve
- `timestamp` (string): ISO 8601 timestamp (e.g., "2024-01-15T10:30:00Z")

**Returns:** `Promise<any>` - The value at that point in time

**Throws:** Error if no value exists at that timestamp

**Example:**
```javascript
const value = await db.getAt('users', 'alice', '2024-01-08T10:00:00Z');
```

---

## Query Operations

### `query(namespace, filters, limit)`

Query the database with filters.

**Parameters:**
- `namespace` (string): The namespace to query
- `filters` (object): Filter conditions (e.g., `{ status: 'active' }`)
- `limit` (number, optional): Maximum number of results

**Returns:** `Promise<QueryResult>` - Query results

**Example:**
```javascript
const result = await db.query('users', { status: 'active' }, 10);
console.log(`Found ${result.total} users`);
result.records.forEach(record => {
  console.log(record.value);
});
```

---

## Semantic Search (Vector Operations)

### `putSimilar(namespace, key, content, metadata)`

Store content with automatic semantic embedding.

**Parameters:**
- `namespace` (string): The namespace
- `key` (string): The key to store under
- `content` (string): Text content to embed
- `metadata` (object, optional): Additional metadata

**Returns:** `Promise<VersionedValue>` - Version information

**Example:**
```javascript
await db.putSimilar('docs', 'article1', 
  'Machine learning is transforming software development',
  { author: 'Alice', category: 'AI' }
);
```

### `findSimilar(namespace, query, topK, threshold)`

Find semantically similar content.

**Parameters:**
- `namespace` (string): The namespace to search
- `query` (string): Query text
- `topK` (number): Maximum results (default: 5)
- `threshold` (number): Minimum similarity score 0-1 (default: 0)

**Returns:** `Promise<SimilarityResult[]>` - Similar items with scores

**Example:**
```javascript
const results = await db.findSimilar('docs', 'programming', 3);
results.forEach(r => {
  console.log(`${r.key}: ${r.score.toFixed(2)}`);
});
```

### `embed(namespace, key, vector, metadata)`

Store an explicit vector embedding.

**Parameters:**
- `namespace` (string): The namespace
- `key` (string): The key
- `vector` (number[]): Vector embedding (array of floats)
- `metadata` (object, optional): Additional metadata

**Returns:** `Promise<VersionedValue>` - Version information

**Example:**
```javascript
await db.embed('vectors', 'doc1', [0.1, 0.2, 0.3, ...], { text: 'Hello' });
```

### `embedSearch(namespace, query, topK)`

Search using an explicit vector.

**Parameters:**
- `namespace` (string): The namespace
- `query` (number[]): Query vector
- `topK` (number): Maximum results

**Returns:** `Promise<SimilarityResult[]>` - Similar items

**Example:**
```javascript
const results = await db.embedSearch('vectors', [0.1, 0.2, 0.3], 5);
```

---

## Batch Operations

### `putBatch(items)`

Store multiple items in a batch (cross-namespace).

**Parameters:**
- `items` (BatchItem[]): Array of items to store

**Returns:** `Promise<VersionedValue[]>` - Version information for each item

**Example:**
```javascript
const items = [
  { namespace: 'users', key: 'alice', value: { name: 'Alice' } },
  { namespace: 'users', key: 'bob', value: { name: 'Bob' } },
];
await db.putBatch(items);
```

### `putBatchInNs(namespace, items)`

Store multiple items in a single namespace.

**Parameters:**
- `namespace` (string): The namespace
- `items` (NsBatchItem[]): Array of {key, value} objects

**Returns:** `Promise<VersionedValue[]>` - Version information

**Example:**
```javascript
await db.putBatchInNs('users', [
  { key: 'alice', value: { name: 'Alice' } },
  { key: 'bob', value: { name: 'Bob' } },
]);
```

---

## Views

### `createView(name, sourceNamespace)`

Create a materialized view.

**Parameters:**
- `name` (string): View name
- `sourceNamespace` (string): Source namespace

**Returns:** `Promise<View>` - The created view

**Example:**
```javascript
const view = await db.createView('active_users', 'users');
```

### `queryView(name)`

Query a materialized view.

**Parameters:**
- `name` (string): View name

**Returns:** `Promise<QueryResult>` - View results

**Example:**
```javascript
const result = await db.queryView('active_users');
```

### `refreshView(name)`

Refresh a materialized view.

**Parameters:**
- `name` (string): View name

**Returns:** `Promise<void>`

### `listViews()`

List all views.

**Returns:** `Promise<ViewInfo[]>` - Array of view information

### `deleteView(name)`

Delete a view.

**Parameters:**
- `name` (string): View name

**Returns:** `Promise<boolean>` - True if deleted

---

## Identity Management

### `createIdentity(displayName, description)`

Create a new self-sovereign identity.

**Parameters:**
- `displayName` (string): Display name for the identity
- `description` (string, optional): Description

**Returns:** `Promise<Identity>` - The created identity

**Example:**
```javascript
const identity = await db.createIdentity('Alice Admin', 'System administrator');
console.log(identity.id);       // Public key
console.log(identity.secret);   // Keep secure!
```

### `verifyIdentity(identityId)`

Verify an identity's validity.

**Parameters:**
- `identityId` (string): The identity ID to verify

**Returns:** `Promise<boolean>` - True if valid

**Example:**
```javascript
const isValid = await db.verifyIdentity(identity.id);
```

### `getIdentity(identityId)`

Get identity information.

**Parameters:**
- `identityId` (string): The identity ID

**Returns:** `Promise<IdentityInfo | null>` - Identity info or null

---

## Workspace

### `workspace(name)`

Get a workspace handle for isolated memory space.

**Parameters:**
- `name` (string): Workspace name

**Returns:** `WorkspaceHandle` - Workspace interface

**Example:**
```javascript
const ws = db.workspace('my_project');
await ws.put('config', { setting: 'value' });
const config = await ws.get('config');
```

### WorkspaceHandle Methods

#### `put(key, value)`
Store in workspace.

#### `get(key)`
Retrieve from workspace.

#### `delete(key)`
Delete from workspace.

#### `listKeys()`
List keys in workspace.

---

## Utility Methods

### `listNamespaces()`

List all namespaces.

**Returns:** `Promise<string[]>` - Array of namespace names

### `listKeys(namespace)`

List all keys in a namespace.

**Parameters:**
- `namespace` (string): The namespace

**Returns:** `Promise<string[]>` - Array of keys

### `stats()`

Get database statistics.

**Returns:** `Promise<DatabaseStats>` - Statistics object

**Example:**
```javascript
const stats = await db.stats();
console.log(`Keys: ${stats.keyCount}`);
console.log(`Versions: ${stats.totalVersions}`);
console.log(`Namespaces: ${stats.namespaceCount}`);
```

### `isPersistent()`

Check if database is persistent (IndexedDB-backed).

**Returns:** `boolean` - True if persistent

### `clearPersistence()`

Clear all persisted data (browser-only).

**Returns:** `Promise<void>`

---

## Types

### VersionedValue

```typescript
interface VersionedValue {
  value: any;              // The stored value
  timestamp: string;       // ISO 8601 timestamp
  versionId: string;       // Unique version ID
  previousVersion?: string; // Previous version ID
}
```

### HistoryEntry

```typescript
interface HistoryEntry {
  value: any;
  timestamp: string;
  versionId: string;
}
```

### SimilarityResult

```typescript
interface SimilarityResult {
  namespace: string;
  key: string;
  score: number;  // 0-1, higher is better
}
```

### BatchItem

```typescript
interface BatchItem {
  namespace: string;
  key: string;
  value: any;
}
```

### NsBatchItem

```typescript
interface NsBatchItem {
  key: string;
  value: any;
}
```

### Identity

```typescript
interface Identity {
  id: string;        // Public key
  secret: string;    // Private key (keep secure!)
  displayName: string;
  createdAt: string; // ISO 8601 timestamp
}
```

### DatabaseStats

```typescript
interface DatabaseStats {
  keyCount: number;
  totalVersions: number;
  namespaceCount: number;
}
```
