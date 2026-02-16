/* tslint:disable */
/* eslint-disable */
/**
 * KoruDelta - The Causal Database for JavaScript
 * 
 * Content-addressed storage with natural time-travel queries and
 * distinction-based semantic search. Like Git for your data, directly
 * in the browser or Node.js via WASM.
 * 
 * LCA Architecture v3.0.0 - Local Causal Agent pattern throughout.
 * 
 * @version 3.0.0
 * @example
 * ```typescript
 * import { KoruDeltaWasm } from 'koru-delta';
 * 
 * const db = await KoruDeltaWasm.new();
 * 
 * // Basic operations
 * await db.put('users', 'alice', { name: 'Alice', age: 30 });
 * const user = await db.get('users', 'alice');
 * 
 * // Semantic storage (auto-generated embeddings)
 * await db.putSimilar('docs', 'article1', 'Hello world', { author: 'Alice' });
 * const results = await db.findSimilar('docs', 'hello query', 5);
 * 
 * // Identity management
 * const identity = await db.createIdentity('User Name', 'Bio');
 * const valid = await db.verifyIdentity(identity.id);
 * ```
 */

/**
 * A versioned value returned from the database
 */
export interface VersionedValue {
  /** The stored value */
  value: any;
  /** ISO 8601 timestamp of when this version was created */
  timestamp: string;
  /** Unique version identifier */
  versionId: string;
  /** Previous version ID (undefined if first version) */
  previousVersion?: string;
}

/**
 * A history entry for time-travel queries
 */
export interface HistoryEntry {
  /** The value at this point in history */
  value: any;
  /** ISO 8601 timestamp */
  timestamp: string;
  /** Version identifier */
  versionId: string;
}

/**
 * Database statistics
 */
export interface DatabaseStats {
  /** Number of keys in the database */
  keyCount: number;
  /** Total number of versions across all keys */
  totalVersions: number;
  /** Number of namespaces */
  namespaceCount: number;
}

/**
 * Search result from semantic similarity search
 */
export interface SimilarityResult {
  /** Namespace of the matched document */
  namespace: string;
  /** Key of the matched document */
  key: string;
  /** Similarity score (0-1, higher is better) */
  score: number;
}

/**
 * Batch item for putBatch operations
 */
export interface BatchItem {
  /** The namespace */
  namespace: string;
  /** The key within the namespace */
  key: string;
  /** JSON-serializable value */
  value: any;
}

/**
 * Namespace-scoped batch item for putBatchInNs
 */
export interface NsBatchItem {
  /** The key */
  key: string;
  /** JSON-serializable value */
  value: any;
}

/**
 * Identity information
 */
export interface Identity {
  /** Public key (identity ID) */
  id: string;
  /** Secret key (keep secure!) */
  secretKey: string;
  /** ISO 8601 creation timestamp */
  createdAt: string;
}

/**
 * Identity info (from getIdentity)
 */
export interface IdentityInfo {
  /** Public key (identity ID) */
  id: string;
  /** ISO 8601 creation timestamp */
  createdAt: string;
  /** Mining difficulty */
  difficulty: number;
}

/**
 * View definition
 */
export interface View {
  /** View name */
  name: string;
  /** Source namespace/collection */
  source: string;
}

/**
 * Query result record
 */
export interface QueryRecord {
  /** The key */
  key: string;
  /** The stored value */
  value: any;
  /** ISO 8601 timestamp */
  timestamp: string;
}

/**
 * Query result
 */
export interface QueryResult {
  /** Matching records */
  records: QueryRecord[];
  /** Total count (for pagination) */
  totalCount: number;
  /** Optional aggregation data */
  aggregation?: any;
}

/**
 * Workspace handle for namespace-scoped operations
 */
export class WorkspaceHandle {
  /** The workspace namespace */
  readonly namespace: string;
  
  /** String representation */
  toString(): string;
}

/**
 * Main database class for JavaScript/TypeScript environments.
 * 
 * Provides content-addressed storage with causal consistency, time-travel queries,
 * semantic search, identity management, and materialized views.
 * 
 * All operations are asynchronous and return Promises.
 */
export class KoruDeltaWasm {
  /**
   * Create a new KoruDelta database instance.
   * 
   * @returns A Promise that resolves to a new database instance
   * @example
   * ```typescript
   * const db = await KoruDeltaWasm.new();
   * ```
   */
  static new(): Promise<KoruDeltaWasm>;

  /**
   * Create a new persistent KoruDelta database instance with IndexedDB.
   * 
   * Data will be automatically saved to IndexedDB and loaded on startup.
   * Falls back to memory-only if IndexedDB is unavailable.
   * 
   * @returns A Promise that resolves to a persistent database instance
   * @example
   * ```typescript
   * const db = await KoruDeltaWasm.newPersistent();
   * 
   * if (db.isPersistent()) {
   *   console.log("Data will survive page refreshes!");
   * }
   * ```
   */
  static newPersistent(): Promise<KoruDeltaWasm>;

  /**
   * Store a value in the database.
   * 
   * @param namespace - The namespace (e.g., "users", "posts")
   * @param key - The key within the namespace
   * @param value - Any JSON-serializable value
   * @returns A Promise that resolves to the versioned stored value
   * @example
   * ```typescript
   * await db.put('users', 'alice', { name: 'Alice', age: 30 });
   * ```
   */
  put(namespace: string, key: string, value: any): Promise<VersionedValue>;

  /**
   * Store content with automatic semantic embedding.
   * 
   * This is the simplified API for semantic storage. The embedding is
   * synthesized from the content structure using distinction calculus.
   * 
   * @param namespace - The namespace
   * @param key - The key
   * @param content - Content to store (will be embedded)
   * @param metadata - Optional metadata object
   * @returns A Promise that resolves when stored
   * @example
   * ```typescript
   * await db.putSimilar('docs', 'article1', 'Hello world', { author: 'Alice' });
   * await db.putSimilar('docs', 'article2', { title: 'Test', body: 'Content' });
   * ```
   */
  putSimilar(namespace: string, key: string, content: any, metadata?: any): Promise<void>;

  /**
   * Find similar content using semantic search.
   * 
   * Searches for content similar to the provided query using
   * distinction-based embeddings and cosine similarity.
   * 
   * @param namespace - Namespace to search (null for all namespaces)
   * @param query - Query content to search for
   * @param topK - Maximum number of results (default: 10)
   * @returns A Promise that resolves to an array of similarity results
   * @example
   * ```typescript
   * // Search within a namespace
   * const results = await db.findSimilar('docs', 'hello world', 5);
   * 
   * // Search all namespaces
   * const results = await db.findSimilar(null, 'query string', 10);
   * 
   * results.forEach(r => {
   *   console.log(`${r.namespace}/${r.key}: ${r.score}`);
   * });
   * ```
   */
  findSimilar(namespace: string | null, query: any, topK?: number): Promise<SimilarityResult[]>;

  /**
   * Store multiple values as a batch operation.
   * 
   * This is significantly faster than calling put() multiple times,
   * especially when persistence is enabled.
   * 
   * @param items - Array of objects with namespace, key, and value properties
   * @returns A Promise that resolves to an array of versioned stored values
   * @example
   * ```typescript
   * const items = [
   *   { namespace: 'users', key: 'alice', value: { name: 'Alice' } },
   *   { namespace: 'users', key: 'bob', value: { name: 'Bob' } }
   * ];
   * const results = await db.putBatch(items);
   * ```
   */
  putBatch(items: BatchItem[]): Promise<VersionedValue[]>;

  /**
   * Store multiple values in a single namespace (simplified API).
   * 
   * @param namespace - The namespace to store all items in
   * @param items - Array of [key, value] pairs
   * @returns A Promise that resolves to the count of items stored
   * @example
   * ```typescript
   * const items = [
   *   ['key1', { value: 1 }],
   *   ['key2', { value: 2 }]
   * ];
   * const count = await db.putBatchInNs('my_namespace', items);
   * ```
   */
  putBatchInNs(namespace: string, items: Array<[string, any]>): Promise<number>;

  /**
   * Retrieve the current value for a key.
   * 
   * @param namespace - The namespace
   * @param key - The key within the namespace
   * @returns A Promise that resolves to the versioned value
   * @throws Error if key not found
   * @example
   * ```typescript
   * const user = await db.get('users', 'alice');
   * console.log(user.value.name); // 'Alice'
   * ```
   */
  get(namespace: string, key: string): Promise<VersionedValue>;

  /**
   * Get the complete version history for a key.
   * 
   * @param namespace - The namespace
   * @param key - The key within the namespace
   * @returns A Promise that resolves to an array of history entries (newest first)
   * @example
   * ```typescript
   * const history = await db.history('users', 'alice');
   * history.forEach(entry => {
   *   console.log(`${entry.timestamp}: ${entry.value.name}`);
   * });
   * ```
   */
  history(namespace: string, key: string): Promise<HistoryEntry[]>;

  /**
   * Get the value at a specific point in time.
   * 
   * This is the time-travel feature - query what the value was at any past timestamp.
   * 
   * @param namespace - The namespace
   * @param key - The key
   * @param timestampIso - ISO 8601 timestamp (e.g., "2024-01-15T10:30:00Z")
   * @returns A Promise that resolves to the value at that time
   * @throws Error if no value exists at that timestamp
   * @example
   * ```typescript
   * // What was Alice's data last Tuesday?
   * const pastValue = await db.getAt(
   *   'users', 
   *   'alice', 
   *   '2024-01-10T09:00:00Z'
   * );
   * ```
   */
  getAt(namespace: string, key: string, timestampIso: string): Promise<any>;

  /**
   * List all namespaces in the database.
   * 
   * @returns A Promise that resolves to an array of namespace names
   * @example
   * ```typescript
   * const namespaces = await db.listNamespaces();
   * // ['users', 'posts', 'config']
   * ```
   */
  listNamespaces(): Promise<string[]>;

  /**
   * List all keys in a namespace.
   * 
   * @param namespace - The namespace to list
   * @returns A Promise that resolves to an array of keys
   * @example
   * ```typescript
   * const users = await db.listKeys('users');
   * // ['alice', 'bob', 'charlie']
   * ```
   */
  listKeys(namespace: string): Promise<string[]>;

  /**
   * Delete a key.
   * 
   * @param namespace - The namespace
   * @param key - The key to delete
   * @returns A Promise that resolves when deleted
   */
  delete(namespace: string, key: string): Promise<void>;

  /**
   * Check if a key exists.
   * 
   * @param namespace - The namespace
   * @param key - The key to check
   * @returns A Promise that resolves to boolean
   */
  contains(namespace: string, key: string): Promise<boolean>;

  /**
   * Get database statistics.
   * 
   * @returns A Promise that resolves to database statistics
   * @example
   * ```typescript
   * const stats = await db.stats();
   * console.log(`Database has ${stats.keyCount} keys`);
   * ```
   */
  stats(): Promise<DatabaseStats>;

  /**
   * Store a vector embedding associated with a document.
   * 
   * @param namespace - Document namespace
   * @param key - Document key
   * @param vector - Array of f32 values (the embedding)
   * @param model - Optional model identifier
   * @returns A Promise that resolves when stored
   */
  embed(namespace: string, key: string, vector: number[], model?: string): Promise<void>;

  /**
   * Search for similar documents by vector.
   * 
   * @param namespace - Namespace to search
   * @param query - Array of f32 values (the query embedding)
   * @param limit - Maximum number of results (default: 10)
   * @returns A Promise that resolves to an array of search results
   */
  embedSearch(namespace: string, query: number[], limit?: number): Promise<SimilarityResult[]>;

  /**
   * Delete an embedding.
   * 
   * @param namespace - Document namespace
   * @param key - Document key
   * @returns A Promise that resolves when deleted
   */
  deleteEmbed(namespace: string, key: string): Promise<void>;

  /**
   * Query the database with filters.
   * 
   * @param namespace - Namespace to query
   * @param filter - Filter object (e.g., {status: "active", age: 30})
   * @param limit - Maximum results
   * @returns A Promise that resolves to query results
   * @example
   * ```typescript
   * const results = await db.query('users', { age: 30, status: 'active' }, 10);
   * results.records.forEach(r => {
   *   console.log(`${r.key}: ${r.value.name}`);
   * });
   * ```
   */
  query(namespace: string, filter: any, limit?: number): Promise<QueryRecord[]>;

  /**
   * Create a materialized view.
   * 
   * @param name - View name
   * @param sourceNamespace - Source collection/namespace
   * @returns A Promise that resolves when created
   */
  createView(name: string, sourceNamespace: string): Promise<void>;

  /**
   * List all views.
   * 
   * @returns A Promise that resolves to an array of views
   */
  listViews(): Promise<View[]>;

  /**
   * Query a view.
   * 
   * @param name - View name
   * @returns A Promise that resolves to query results
   */
  queryView(name: string): Promise<QueryResult>;

  /**
   * Refresh a view.
   * 
   * @param name - View name
   * @returns A Promise that resolves when refreshed
   */
  refreshView(name: string): Promise<void>;

  /**
   * Delete a view.
   * 
   * @param name - View name
   * @returns A Promise that resolves when deleted
   */
  deleteView(name: string): Promise<void>;

  /**
   * Create a new identity.
   * 
   * @param displayName - Optional display name
   * @param bio - Optional bio/description
   * @returns A Promise that resolves to the created identity
   * @example
   * ```typescript
   * const identity = await db.createIdentity('Alice', 'Software developer');
   * console.log(`Created identity: ${identity.id}`);
   * console.log(`Secret key: ${identity.secretKey}`); // Keep this secure!
   * ```
   */
  createIdentity(displayName?: string, bio?: string): Promise<Identity>;

  /**
   * Verify an identity exists and is valid.
   * 
   * @param identityId - The identity public key
   * @returns A Promise that resolves to boolean indicating validity
   */
  verifyIdentity(identityId: string): Promise<boolean>;

  /**
   * Get identity information.
   * 
   * @param identityId - The identity public key
   * @returns A Promise that resolves to identity info, or null if not found
   */
  getIdentity(identityId: string): Promise<IdentityInfo | null>;

  /**
   * Get a workspace handle.
   * 
   * @param name - Workspace name
   * @returns A WorkspaceHandle for operations within that namespace
   * @example
   * ```typescript
   * const ws = db.workspace('my_project');
   * console.log(`Working in ${ws.namespace}`);
   * ```
   */
  workspace(name: string): WorkspaceHandle;

  /**
   * Check if the database is using IndexedDB persistence.
   */
  isPersistent(): boolean;

  /**
   * Check if IndexedDB is supported in the current environment.
   */
  isIndexedDbSupported(): boolean;

  /**
   * Clear all persisted data from IndexedDB.
   * 
   * This will delete all data stored in IndexedDB. Use with caution!
   */
  clearPersistence(): Promise<void>;
}

/**
 * Initialize the WASM module.
 * 
 * This is automatically called when importing the module, but can be
 * called explicitly for custom initialization scenarios.
 */
export function init(): void;

/**
 * Default export for convenience.
 * 
 * @example
 * ```typescript
 * import KoruDelta from 'koru-delta';
 * 
 * const db = await KoruDelta.new();
 * ```
 */
export default KoruDeltaWasm;
