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
 * A highly-connected distinction in the causal graph
 */
export interface ConnectedDistinction {
  /** The namespace containing this distinction */
  namespace: string;
  /** The key (distinction ID) */
  key: string;
  /** Total connectivity score (parents + children + synthesis events) */
  connectionScore: number;
  /** Causal parents (what caused this distinction) */
  parents: string[];
  /** Causal children (what this distinction caused) */
  children: string[];
}

/**
 * A pair of similar but causally unconnected distinctions
 */
export interface UnconnectedPair {
  /** First distinction's namespace */
  namespaceA: string;
  /** First distinction's key */
  keyA: string;
  /** Second distinction's namespace */
  namespaceB: string;
  /** Second distinction's key */
  keyB: string;
  /** Cosine similarity score (0.0 - 1.0) */
  similarityScore: number;
}

/**
 * A random combination discovered through dream-phase random walks
 */
export interface RandomCombination {
  /** Starting distinction's namespace */
  startNamespace: string;
  /** Starting distinction's key */
  startKey: string;
  /** Ending distinction's namespace */
  endNamespace: string;
  /** Ending distinction's key */
  endKey: string;
  /** Intermediate distinction IDs in the walk */
  path: string[];
  /** Novelty score - higher means more distant/interesting (0.0 - 1.0) */
  noveltyScore: number;
}

/**
 * TTL information for expiring keys
 */
export interface ExpiringKey {
  /** The namespace */
  namespace: string;
  /** The key */
  key: string;
  /** Seconds remaining until expiration */
  secondsRemaining: number;
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

  // ============================================================================
  // TTL (Time-To-Live) Methods
  // ============================================================================

  /**
   * Store a value with TTL (time-to-live) in seconds.
   * 
   * The value will be automatically deleted after the specified number of seconds.
   * This is useful for temporary data, sessions, cache entries, etc.
   * 
   * @param namespace - The namespace
   * @param key - The key
   * @param value - Any JSON-serializable value
   * @param ttlSeconds - Time-to-live in seconds
   * @returns A Promise that resolves to the versioned stored value
   * @example
   * ```typescript
   * // Store a session that expires in 1 hour
   * await db.putWithTtl('sessions', 'user_123', { user: 'alice' }, 3600);
   * ```
   */
  putWithTtl(namespace: string, key: string, value: any, ttlSeconds: number): Promise<VersionedValue>;

  /**
   * Store content with TTL and automatic semantic embedding.
   * 
   * Combines semantic storage with automatic expiration.
   * 
   * @param namespace - The namespace
   * @param key - The key
   * @param content - Content to store (will be embedded)
   * @param ttlSeconds - Time-to-live in seconds
   * @param metadata - Optional metadata object
   * @returns A Promise that resolves when stored
   * @example
   * ```typescript
   * await db.putSimilarWithTtl('cache', 'article1', 'Hello world', 1800, { type: 'greeting' });
   * ```
   */
  putSimilarWithTtl(namespace: string, key: string, content: any, ttlSeconds: number, metadata?: any): Promise<void>;

  /**
   * Clean up all expired TTL values.
   * 
   * @returns A Promise that resolves to the count of items removed
   */
  cleanupExpired(): Promise<number>;

  /**
   * Get remaining TTL for a key in seconds.
   * 
   * @param namespace - The namespace
   * @param key - The key
   * @returns A Promise that resolves to seconds remaining, or null if no TTL
   */
  getTtlRemaining(namespace: string, key: string): Promise<number | null>;

  /**
   * List keys expiring soon.
   * 
   * @param withinSeconds - Return keys expiring within this many seconds
   * @returns A Promise that resolves to an array of expiring key info
   * @example
   * ```typescript
   * // Find all keys expiring in the next hour
   * const expiring = await db.listExpiringSoon(3600);
   * expiring.forEach(k => {
   *   console.log(`${k.namespace}/${k.key}: ${k.secondsRemaining}s remaining`);
   * });
   * ```
   */
  listExpiringSoon(withinSeconds: number): Promise<ExpiringKey[]>;

  // ============================================================================
  // Graph Connectivity Methods
  // ============================================================================

  /**
   * Check if two distinctions are causally connected.
   * 
   * Returns true if there is a causal path between the two distinctions
   * through the causal graph (ancestor/descendant relationship).
   * 
   * @param namespace - The namespace containing both keys
   * @param keyA - First distinction key
   * @param keyB - Second distinction key
   * @returns A Promise that resolves to boolean indicating connection
   * @example
   * ```typescript
   * const connected = await db.areConnected('docs', 'article1', 'article2');
   * if (connected) {
   *   console.log('These documents are causally related');
   * }
   * ```
   */
  areConnected(namespace: string, keyA: string, keyB: string): Promise<boolean>;

  /**
   * Get the causal connection path between two distinctions.
   * 
   * Returns an array of distinction IDs representing the path from keyA to keyB,
   * or null if they are not connected.
   * 
   * @param namespace - The namespace containing both keys
   * @param keyA - Starting distinction key
   * @param keyB - Target distinction key
   * @returns A Promise that resolves to the path array or null
   * @example
   * ```typescript
   * const path = await db.getConnectionPath('docs', 'article1', 'article2');
   * if (path) {
   *   console.log(`Connection path: ${path.join(' -> ')}`);
   * }
   * ```
   */
  getConnectionPath(namespace: string, keyA: string, keyB: string): Promise<string[] | null>;

  /**
   * Get the most highly-connected distinctions.
   * 
   * Returns distinctions ranked by their connectivity score (parents + children).
   * Useful for finding "central" or "important" distinctions in the causal graph.
   * 
   * @param namespace - Optional namespace filter (null for all)
   * @param k - Maximum number of results (default: 10)
   * @returns A Promise that resolves to an array of connected distinctions
   * @example
   * ```typescript
   * // Get top 5 most connected distinctions in the 'docs' namespace
   * const central = await db.getHighlyConnected('docs', 5);
   * central.forEach(d => {
   *   console.log(`${d.key}: score ${d.connectionScore} (${d.parents.length} parents, ${d.children.length} children)`);
   * });
   * ```
   */
  getHighlyConnected(namespace: string | null, k: number): Promise<ConnectedDistinction[]>;

  // ============================================================================
  // Similar Unconnected Pairs (Consolidation Agent)
  // ============================================================================

  /**
   * Find similar distinctions that are not causally connected.
   * 
   * These pairs are candidates for synthesis by the Consolidation agent.
   * Uses vector similarity search combined with causal graph analysis.
   * 
   * @param namespace - Optional namespace filter (null for all)
   * @param k - Maximum number of pairs to return (default: 10)
   * @param threshold - Minimum similarity threshold 0.0-1.0 (default: 0.7)
   * @returns A Promise that resolves to an array of unconnected pairs
   * @example
   * ```typescript
   * // Find similar but disconnected document pairs
   * const pairs = await db.findSimilarUnconnectedPairs('docs', 5, 0.8);
   * pairs.forEach(p => {
   *   console.log(`${p.keyA} <-> ${p.keyB}: ${p.similarityScore.toFixed(2)} similarity`);
   * });
   * ```
   */
  findSimilarUnconnectedPairs(namespace: string | null, k: number, threshold: number): Promise<UnconnectedPair[]>;

  // ============================================================================
  // Random Walk (Dream Phase)
  // ============================================================================

  /**
   * Generate random walk combinations for dream-phase creative synthesis.
   * 
   * Performs random walks through the causal graph to discover novel combinations
   * of distant distinctions. Used by the Sleep agent during REM phase.
   * 
   * @param n - Number of combinations to generate (default: 5)
   * @param steps - Number of steps per random walk (default: 10)
   * @returns A Promise that resolves to an array of random combinations
   * @example
   * ```typescript
   * // Generate 5 random walks of 10 steps each
   * const combinations = await db.randomWalkCombinations(5, 10);
   * combinations.forEach(c => {
   *   console.log(`${c.startKey} -> ${c.endKey} (novelty: ${c.noveltyScore.toFixed(2)})`);
   *   console.log(`  Path: ${c.path.join(' -> ')}`);
   * });
   * ```
   */
  randomWalkCombinations(n: number, steps: number): Promise<RandomCombination[]>;
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
