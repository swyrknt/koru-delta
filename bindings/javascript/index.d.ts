/* tslint:disable */
/* eslint-disable */
/**
 * KoruDelta - The Causal Database
 * 
 * Content-addressed storage with natural time-travel queries.
 * Like Git for your data, directly in the browser.
 * 
 * @version 2.0.0
 * @example
 * ```typescript
 * import { KoruDeltaWasm } from 'koru-delta';
 * 
 * const db = await KoruDeltaWasm.new();
 * await db.put('users', 'alice', { name: 'Alice', age: 30 });
 * const user = await db.get('users', 'alice');
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
 * Main database class for JavaScript/TypeScript environments.
 * 
 * Provides content-addressed storage with causal consistency and time-travel queries.
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
