//! WASM bindings for KoruDelta - LCA Architecture v3.0.0
//!
//! This module provides JavaScript-friendly bindings for using KoruDelta
//! in browser and edge compute environments with the full LCA (Local Causal Agent)
//! architecture.
//!
//! # Features
//! - Full database operations (put, get, history, time-travel)
//! - Semantic storage with auto-generated embeddings (put_similar, find_similar)
//! - Namespace management
//! - Vector embedding storage and similarity search
//! - Query engine with filters, sort, and pagination
//! - Materialized views
//! - Identity management (create, verify)
//! - Workspace abstraction
//! - **IndexedDB persistence** for data survival across page reloads
//!
//! # Usage
//! ```javascript
//! import init, { KoruDeltaWasm } from 'koru-delta';
//!
//! await init();
//!
//! // Memory-only database (data lost on refresh)
//! const db = await KoruDeltaWasm.new();
//!
//! // Persistent database (data survives refresh)
//! const db = await KoruDeltaWasm.newPersistent();
//!
//! // Basic operations
//! await db.put('users', 'alice', { name: 'Alice', age: 30 });
//! const user = await db.get('users', 'alice');
//!
//! // Semantic storage (auto-generated embeddings)
//! await db.putSimilar('docs', 'doc1', 'Hello world', { type: 'greeting' });
//! const results = await db.findSimilar('docs', 'hello query', 5);
//!
//! // Identity management
//! const identity = await db.createIdentity('User Name', 'Bio');
//! const valid = await db.verifyIdentity(identity.id);
//! ```

mod storage;

use crate::auth::IdentityUserData;
use crate::vector::{Vector, VectorSearchOptions};
use crate::{DeltaError, HistoryEntry, KoruDelta, VersionedValue, ViewDefinition};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use storage::{IndexedDbStorage, is_indexeddb_supported};
use wasm_bindgen::prelude::*;

/// WASM-friendly wrapper around KoruDelta
#[wasm_bindgen]
pub struct KoruDeltaWasm {
    db: KoruDelta,
    storage: Option<IndexedDbStorage>,
}

/// Helper struct for batch operations from JavaScript
#[derive(Deserialize)]
struct BatchItem {
    namespace: String,
    key: String,
    value: serde_json::Value,
}

/// Helper struct for namespace-scoped batch operations
#[derive(Deserialize)]
struct NsBatchItem {
    key: String,
    value: serde_json::Value,
}

/// Search result from semantic similarity search
#[derive(Clone)]
pub struct SearchResult {
    pub namespace: String,
    pub key: String,
    pub score: f32,
}

#[wasm_bindgen]
impl KoruDeltaWasm {
    /// Create a new in-memory KoruDelta database instance
    ///
    /// Data will be lost when the page is refreshed. For persistent storage,
    /// use `newPersistent()` instead.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const db = await KoruDeltaWasm.new();
    /// ```
    pub async fn new() -> Result<KoruDeltaWasm, JsValue> {
        let db = KoruDelta::start()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to start database: {}", e)))?;

        Ok(KoruDeltaWasm { db, storage: None })
    }

    /// Create a new persistent KoruDelta database instance with IndexedDB
    ///
    /// Data will be automatically saved to IndexedDB and loaded on startup.
    /// Falls back to memory-only if IndexedDB is unavailable.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const db = await KoruDeltaWasm.newPersistent();
    ///
    /// // Check if persistence is working
    /// if (db.isPersistent()) {
    ///   console.log("Data will survive page refreshes!");
    /// }
    /// ```
    #[wasm_bindgen(js_name = newPersistent)]
    pub async fn new_persistent() -> Result<KoruDeltaWasm, JsValue> {
        let db = KoruDelta::start()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to start database: {}", e)))?;

        // Initialize IndexedDB storage
        let storage = IndexedDbStorage::new().await?;

        let mut wasm_db = KoruDeltaWasm {
            db,
            storage: Some(storage),
        };

        // Load existing data from IndexedDB
        wasm_db.load_from_storage().await?;

        Ok(wasm_db)
    }

    /// Check if the database is using IndexedDB persistence
    #[wasm_bindgen(js_name = isPersistent)]
    pub fn is_persistent(&self) -> bool {
        self.storage
            .as_ref()
            .map(|s| s.is_persistent())
            .unwrap_or(false)
    }

    /// Check if IndexedDB is supported in the current environment
    #[wasm_bindgen(js_name = isIndexedDbSupported)]
    pub fn is_indexeddb_supported_js() -> bool {
        is_indexeddb_supported()
    }

    /// Clear all persisted data from IndexedDB
    ///
    /// This will delete all data stored in IndexedDB. Use with caution!
    #[wasm_bindgen(js_name = clearPersistence)]
    pub async fn clear_persistence(&self) -> Result<(), JsValue> {
        if let Some(storage) = &self.storage {
            storage.clear_all().await?;
            web_sys::console::log_1(&"IndexedDB: All data cleared".into());
        }
        Ok(())
    }

    /// Load data from IndexedDB storage
    async fn load_from_storage(&mut self) -> Result<(), JsValue> {
        let storage = match &self.storage {
            Some(s) => s,
            None => return Ok(()),
        };

        let records = storage.load_all_records().await?;
        let record_count = records.len();

        for (namespace, key, value, _timestamp, _version_id, _previous_version) in records {
            // Restore the data to the database
            // We use put without triggering persistence to avoid recursion
            let _ = self.db.put(&namespace, &key, value).await;
        }

        if storage.is_persistent() {
            web_sys::console::log_1(
                &format!("Loaded {} records from IndexedDB", record_count).into(),
            );
        }

        Ok(())
    }

    /// Save a record to IndexedDB (called after successful put)
    async fn save_to_storage(
        &self,
        namespace: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), JsValue> {
        let storage = match &self.storage {
            Some(s) => s,
            None => return Ok(()),
        };

        // Get the latest version info from the database
        if let Ok(versioned) = self.db.get(namespace, key).await {
            storage
                .save_record(
                    namespace,
                    key,
                    value,
                    &versioned.timestamp(),
                    versioned.version_id(),
                    versioned.previous_version(),
                )
                .await?;
        }

        Ok(())
    }

    /// Store a value in the database
    ///
    /// # Arguments
    /// * `namespace` - The namespace (e.g., "users")
    /// * `key` - The key within the namespace (e.g., "alice")
    /// * `value` - JSON value to store (as JavaScript object/value)
    ///
    /// # Returns
    /// A JavaScript object with version information
    #[wasm_bindgen(js_name = put)]
    pub async fn put_js(
        &self,
        namespace: &str,
        key: &str,
        value: JsValue,
    ) -> Result<JsValue, JsValue> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON value: {}", e)))?;

        let versioned = self
            .db
            .put(namespace, key, json_value.clone())
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store value: {}", e)))?;

        // Auto-save to IndexedDB if persistence is enabled
        if let Err(e) = self.save_to_storage(namespace, key, &json_value).await {
            web_sys::console::warn_1(&format!("Failed to save to IndexedDB: {:?}", e).into());
            // Don't fail the put if IndexedDB save fails - data is still in memory
        }

        versioned_to_js(&versioned)
    }

    /// Store content with automatic distinction-based embedding
    ///
    /// This is the simplified API for semantic storage. The embedding is
    /// synthesized from the content structure using distinction calculus.
    ///
    /// # Arguments
    /// * `namespace` - The namespace
    /// * `key` - The key
    /// * `content` - Content to store (will be embedded)
    /// * `metadata` - Optional metadata (JavaScript object)
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// await db.putSimilar('docs', 'article1', 'Hello world', { author: 'Alice' });
    /// ```
    #[wasm_bindgen(js_name = putSimilar)]
    pub async fn put_similar_js(
        &self,
        namespace: &str,
        key: &str,
        content: JsValue,
        metadata: Option<JsValue>,
    ) -> Result<(), JsValue> {
        let json_content: JsonValue = serde_wasm_bindgen::from_value(content)
            .map_err(|e| JsValue::from_str(&format!("Invalid content: {}", e)))?;

        let meta = metadata.and_then(|m| serde_wasm_bindgen::from_value(m).ok());

        self.db
            .put_similar(namespace, key, json_content, meta)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store similar content: {}", e)))?;

        Ok(())
    }

    /// Find similar content using semantic search
    ///
    /// Searches for content similar to the provided query using
    /// distinction-based embeddings and cosine similarity.
    ///
    /// # Arguments
    /// * `namespace` - Namespace to search (optional, pass null for all namespaces)
    /// * `query` - Query content to search for
    /// * `top_k` - Maximum number of results (default: 10)
    ///
    /// # Returns
    /// Array of search results with namespace, key, and score
    #[wasm_bindgen(js_name = findSimilar)]
    pub async fn find_similar_js(
        &self,
        namespace: Option<String>,
        query: JsValue,
        top_k: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        let json_query: JsonValue = serde_wasm_bindgen::from_value(query)
            .map_err(|e| JsValue::from_str(&format!("Invalid query: {}", e)))?;

        let results = self
            .db
            .find_similar(namespace.as_deref(), json_query, top_k.unwrap_or(10))
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to search: {}", e)))?;

        let js_array = js_sys::Array::new();
        for result in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &"namespace".into(),
                &JsValue::from_str(&result.namespace),
            )?;
            js_sys::Reflect::set(&obj, &"key".into(), &JsValue::from_str(&result.key))?;
            js_sys::Reflect::set(
                &obj,
                &"score".into(),
                &JsValue::from_f64(result.score as f64),
            )?;
            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Store multiple values as a batch operation
    ///
    /// This is significantly faster than calling put() multiple times, especially
    /// when persistence is enabled, as it performs a single fsync for all items.
    ///
    /// # Arguments
    /// * `items` - A JavaScript array of objects with `namespace`, `key`, and `value` properties
    ///
    /// # Returns
    /// A JavaScript array of versioned metadata objects for each stored item
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const items = [
    ///     { namespace: 'users', key: 'alice', value: { name: 'Alice', age: 30 } },
    ///     { namespace: 'users', key: 'bob', value: { name: 'Bob', age: 25 } }
    /// ];
    /// const results = await db.putBatch(items);
    /// ```
    #[wasm_bindgen(js_name = putBatch)]
    pub async fn put_batch_js(&self, items: JsValue) -> Result<JsValue, JsValue> {
        // Convert JavaScript array to Vec of batch items
        let batch_items: Vec<BatchItem> = serde_wasm_bindgen::from_value(items)
            .map_err(|e| JsValue::from_str(&format!("Invalid batch items: {}", e)))?;

        if batch_items.is_empty() {
            return Ok(JsValue::from_str("[]"));
        }

        // Convert BatchItems to tuples for put_batch
        let tuples: Vec<(String, String, serde_json::Value)> = batch_items
            .into_iter()
            .map(|item| (item.namespace, item.key, item.value))
            .collect();

        // Perform the batch write
        let versioned = self
            .db
            .put_batch(tuples)
            .await
            .map_err(|e| JsValue::from_str(&format!("Batch write failed: {}", e)))?;

        // Convert results back to JavaScript array
        let js_results: Vec<JsValue> = versioned
            .iter()
            .map(versioned_to_js)
            .collect::<Result<Vec<_>, _>>()?;

        // Create JavaScript array from results
        let array = js_sys::Array::new();
        for result in js_results {
            array.push(&result);
        }

        Ok(array.into())
    }

    /// Store multiple values in a single namespace (simplified API)
    ///
    /// # Arguments
    /// * `namespace` - The namespace to store all items in
    /// * `items` - A JavaScript array of [key, value] pairs
    ///
    /// # Returns
    /// Count of items stored
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const items = [
    ///     ['key1', { value: 1 }],
    ///     ['key2', { value: 2 }]
    /// ];
    /// const count = await db.putBatchInNs('my_namespace', items);
    /// ```
    #[wasm_bindgen(js_name = putBatchInNs)]
    pub async fn put_batch_in_ns_js(
        &self,
        namespace: &str,
        items: JsValue,
    ) -> Result<usize, JsValue> {
        // Convert JavaScript array to Vec of namespace-scoped batch items
        let batch_items: Vec<NsBatchItem> = serde_wasm_bindgen::from_value(items)
            .map_err(|e| JsValue::from_str(&format!("Invalid batch items: {}", e)))?;

        if batch_items.is_empty() {
            return Ok(0);
        }

        // Convert to tuples for put_batch_in_ns
        let tuples: Vec<(String, serde_json::Value)> = batch_items
            .into_iter()
            .map(|item| (item.key, item.value))
            .collect();

        let count = tuples.len();

        // Perform the batch write
        self.db
            .put_batch_in_ns(namespace, tuples)
            .await
            .map_err(|e| JsValue::from_str(&format!("Batch write failed: {}", e)))?;

        Ok(count)
    }

    /// Retrieve the current value for a key
    ///
    /// # Arguments
    /// * `namespace` - The namespace
    /// * `key` - The key within the namespace
    ///
    /// # Returns
    /// A JavaScript object with the value and metadata
    #[wasm_bindgen(js_name = get)]
    pub async fn get_js(&self, namespace: &str, key: &str) -> Result<JsValue, JsValue> {
        let versioned = self.db.get(namespace, key).await.map_err(|e| match e {
            DeltaError::KeyNotFound { .. } => {
                JsValue::from_str(&format!("Key not found: {}/{}", namespace, key))
            }
            _ => JsValue::from_str(&format!("Failed to retrieve value: {}", e)),
        })?;

        versioned_to_js(&versioned)
    }

    /// Get the complete history for a key
    ///
    /// # Arguments
    /// * `namespace` - The namespace
    /// * `key` - The key within the namespace
    ///
    /// # Returns
    /// Array of version history entries
    #[wasm_bindgen(js_name = history)]
    pub async fn history_js(&self, namespace: &str, key: &str) -> Result<JsValue, JsValue> {
        let history = self.db.history(namespace, key).await.map_err(|e| match e {
            DeltaError::KeyNotFound { .. } => {
                JsValue::from_str(&format!("Key not found: {}/{}", namespace, key))
            }
            _ => JsValue::from_str(&format!("Failed to retrieve history: {}", e)),
        })?;

        let js_array = js_sys::Array::new();
        for entry in history {
            js_array.push(&history_entry_to_js(&entry)?);
        }

        Ok(js_array.into())
    }

    /// Get value at a specific timestamp
    ///
    /// # Arguments
    /// * `namespace` - The namespace
    /// * `key` - The key
    /// * `timestamp_iso` - ISO 8601 timestamp string
    ///
    /// # Returns
    /// The value at that point in time
    #[wasm_bindgen(js_name = getAt)]
    pub async fn get_at_js(
        &self,
        namespace: &str,
        key: &str,
        timestamp_iso: &str,
    ) -> Result<JsValue, JsValue> {
        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_iso)
            .map_err(|e| JsValue::from_str(&format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let value = self
            .db
            .get_at(namespace, key, timestamp)
            .await
            .map_err(|e| match e {
                DeltaError::KeyNotFound { .. } => {
                    JsValue::from_str(&format!("Key not found: {}/{}", namespace, key))
                }
                _ => JsValue::from_str(&format!("Failed to retrieve value: {}", e)),
            })?;

        serde_wasm_bindgen::to_value(&value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert value: {}", e)))
    }

    /// List all namespaces in the database
    #[wasm_bindgen(js_name = listNamespaces)]
    pub async fn list_namespaces_js(&self) -> JsValue {
        let namespaces = self.db.list_namespaces().await;

        let js_array = js_sys::Array::new();
        for ns in namespaces {
            js_array.push(&JsValue::from_str(&ns));
        }

        js_array.into()
    }

    /// List all keys in a namespace
    #[wasm_bindgen(js_name = listKeys)]
    pub async fn list_keys_js(&self, namespace: &str) -> JsValue {
        let keys = self.db.list_keys(namespace).await;

        let js_array = js_sys::Array::new();
        for key in keys {
            js_array.push(&JsValue::from_str(&key));
        }

        js_array.into()
    }

    /// Delete a key
    ///
    /// Also removes the key from IndexedDB if persistence is enabled.
    #[wasm_bindgen(js_name = delete)]
    pub async fn delete_js(&self, namespace: &str, key: &str) -> Result<(), JsValue> {
        self.db
            .delete(namespace, key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {}", e)))?;

        // Also delete from IndexedDB if persistence is enabled
        if let Some(storage) = &self.storage {
            if let Err(e) = storage.delete_record(namespace, key).await {
                web_sys::console::warn_1(
                    &format!("Failed to delete from IndexedDB: {:?}", e).into(),
                );
            }
        }

        Ok(())
    }

    /// Check if a key exists
    #[wasm_bindgen(js_name = contains)]
    pub async fn contains_js(&self, namespace: &str, key: &str) -> bool {
        self.db.contains(namespace, key).await
    }

    /// Get database statistics
    #[wasm_bindgen(js_name = stats)]
    pub async fn stats_js(&self) -> Result<JsValue, JsValue> {
        let stats = self.db.stats().await;

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"keyCount".into(), &stats.key_count.into())?;
        js_sys::Reflect::set(&obj, &"totalVersions".into(), &stats.total_versions.into())?;
        js_sys::Reflect::set(
            &obj,
            &"namespaceCount".into(),
            &stats.namespace_count.into(),
        )?;

        Ok(obj.into())
    }

    /// Store a vector embedding associated with a document
    ///
    /// # Arguments
    /// * `namespace` - Document namespace
    /// * `key` - Document key
    /// * `vector` - Array of f32 values (the embedding)
    /// * `model` - Optional model identifier
    #[wasm_bindgen(js_name = embed)]
    pub async fn embed_js(
        &self,
        namespace: &str,
        key: &str,
        vector: &js_sys::Array,
        model: Option<String>,
    ) -> Result<(), JsValue> {
        // Convert JS Array to Vec<f32>
        let vec_data: Vec<f32> = vector
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let vec = Vector::new(vec_data, model.as_deref().unwrap_or("default"));

        self.db
            .embed(namespace, key, vec, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store embedding: {}", e)))?;

        Ok(())
    }

    /// Search for similar documents by vector
    ///
    /// # Arguments
    /// * `namespace` - Namespace to search
    /// * `query` - Array of f32 values (the query embedding)
    /// * `limit` - Maximum number of results (default: 10)
    ///
    /// # Returns
    /// Array of search results with namespace, key, and similarity score
    #[wasm_bindgen(js_name = embedSearch)]
    pub async fn embed_search_js(
        &self,
        namespace: &str,
        query: &js_sys::Array,
        limit: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        // Convert JS Array to Vec<f32>
        let query_data: Vec<f32> = query
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let query_vec = Vector::new(query_data, "query");

        let options = VectorSearchOptions {
            top_k: limit.unwrap_or(10),
            ..Default::default()
        };

        let results = self
            .db
            .embed_search(Some(namespace), &query_vec, options)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to search: {}", e)))?;

        let js_array = js_sys::Array::new();
        for result in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &"namespace".into(),
                &JsValue::from_str(&result.namespace),
            )?;
            js_sys::Reflect::set(&obj, &"key".into(), &JsValue::from_str(&result.key))?;
            js_sys::Reflect::set(
                &obj,
                &"score".into(),
                &JsValue::from_f64(result.score as f64),
            )?;

            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Delete an embedding
    #[wasm_bindgen(js_name = deleteEmbed)]
    pub async fn delete_embed_js(&self, namespace: &str, key: &str) -> Result<(), JsValue> {
        self.db
            .delete_embed(namespace, key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to delete embedding: {}", e)))?;
        Ok(())
    }

    /// Create a materialized view
    ///
    /// # Arguments
    /// * `name` - View name
    /// * `source_namespace` - Source collection/namespace
    #[wasm_bindgen(js_name = createView)]
    pub async fn create_view_js(&self, name: &str, source_namespace: &str) -> Result<(), JsValue> {
        let view_def = ViewDefinition::new(name, source_namespace);

        self.db
            .create_view(view_def)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create view: {}", e)))?;

        Ok(())
    }

    /// List all views
    #[wasm_bindgen(js_name = listViews)]
    pub async fn list_views_js(&self) -> Result<JsValue, JsValue> {
        let views = self.db.list_views().await;

        let js_array = js_sys::Array::new();
        for view in views {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"name".into(), &JsValue::from_str(&view.name))?;
            js_sys::Reflect::set(
                &obj,
                &"source".into(),
                &JsValue::from_str(&view.source_collection),
            )?;
            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Query a view
    #[wasm_bindgen(js_name = queryView)]
    pub async fn query_view_js(&self, name: &str) -> Result<JsValue, JsValue> {
        let result = self
            .db
            .query_view(name)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to query view: {}", e)))?;

        let obj = js_sys::Object::new();

        // Convert records
        let records_array = js_sys::Array::new();
        for record in result.records {
            let record_obj = js_sys::Object::new();
            js_sys::Reflect::set(&record_obj, &"key".into(), &JsValue::from_str(&record.key))?;

            let value_js = serde_wasm_bindgen::to_value(&record.value)
                .map_err(|e| JsValue::from_str(&format!("Failed to convert value: {}", e)))?;
            js_sys::Reflect::set(&record_obj, &"value".into(), &value_js)?;

            js_sys::Reflect::set(
                &record_obj,
                &"timestamp".into(),
                &JsValue::from_str(&record.timestamp.to_rfc3339()),
            )?;

            records_array.push(&record_obj);
        }
        js_sys::Reflect::set(&obj, &"records".into(), &records_array)?;

        js_sys::Reflect::set(
            &obj,
            &"totalCount".into(),
            &JsValue::from_f64(result.total_count as f64),
        )?;

        if let Some(agg) = result.aggregation {
            let agg_js = serde_wasm_bindgen::to_value(&agg)
                .map_err(|e| JsValue::from_str(&format!("Failed to convert aggregation: {}", e)))?;
            js_sys::Reflect::set(&obj, &"aggregation".into(), &agg_js)?;
        }

        Ok(obj.into())
    }

    /// Refresh a view
    #[wasm_bindgen(js_name = refreshView)]
    pub async fn refresh_view_js(&self, name: &str) -> Result<(), JsValue> {
        self.db
            .refresh_view(name)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to refresh view: {}", e)))?;
        Ok(())
    }

    /// Delete a view
    #[wasm_bindgen(js_name = deleteView)]
    pub async fn delete_view_js(&self, name: &str) -> Result<(), JsValue> {
        self.db
            .delete_view(name)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to delete view: {}", e)))?;
        Ok(())
    }

    /// Query the database with filters
    ///
    /// # Arguments
    /// * `namespace` - Namespace to query
    /// * `filter` - Filter object (e.g., {status: "active", age: 30})
    /// * `limit` - Maximum results
    #[wasm_bindgen(js_name = query)]
    pub async fn query_js(
        &self,
        namespace: &str,
        filter: JsValue,
        limit: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        use crate::query::{Filter, Query};

        let mut query = Query::new();

        // Add filter if provided
        if !filter.is_null() && !filter.is_undefined() {
            let filter_json: JsonValue = serde_wasm_bindgen::from_value(filter)
                .map_err(|e| JsValue::from_str(&format!("Invalid filter: {}", e)))?;

            if let Some(obj) = filter_json.as_object() {
                for (key, value) in obj {
                    query.filters.push(Filter::Eq {
                        field: key.clone(),
                        value: value.clone(),
                    });
                }
            }
        }

        // Add limit
        if let Some(lim) = limit {
            query.limit = Some(lim);
        }

        let result = self
            .db
            .query(namespace, query)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to query: {}", e)))?;

        let js_array = js_sys::Array::new();
        for record in result.records {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"key".into(), &JsValue::from_str(&record.key))?;

            let value_js = serde_wasm_bindgen::to_value(&record.value)
                .map_err(|e| JsValue::from_str(&format!("Failed to convert value: {}", e)))?;
            js_sys::Reflect::set(&obj, &"value".into(), &value_js)?;

            js_sys::Reflect::set(
                &obj,
                &"timestamp".into(),
                &JsValue::from_str(&record.timestamp.to_rfc3339()),
            )?;

            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Create a new identity
    ///
    /// # Arguments
    /// * `display_name` - Optional display name
    /// * `bio` - Optional bio/description
    ///
    /// # Returns
    /// Object with id, secretKey, and createdAt
    #[wasm_bindgen(js_name = createIdentity)]
    pub async fn create_identity_js(
        &self,
        display_name: Option<String>,
        bio: Option<String>,
    ) -> Result<JsValue, JsValue> {
        use crate::auth::mine_identity;

        let user_data = IdentityUserData {
            display_name,
            bio,
            avatar_hash: None,
            metadata: std::collections::HashMap::new(),
        };

        // Use async mine_identity for WASM
        let mined = mine_identity(user_data, crate::auth::DEFAULT_DIFFICULTY).await;
        let identity = mined.identity;
        let secret_key = mined.secret_key;

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"id".into(), &JsValue::from_str(&identity.public_key))?;
        js_sys::Reflect::set(
            &obj,
            &"secretKey".into(),
            &JsValue::from_str(&hex::encode(&secret_key)),
        )?;
        js_sys::Reflect::set(
            &obj,
            &"createdAt".into(),
            &JsValue::from_str(&identity.created_at.to_rfc3339()),
        )?;

        Ok(obj.into())
    }

    /// Verify an identity exists and is valid
    ///
    /// # Arguments
    /// * `identity_id` - The identity public key
    ///
    /// # Returns
    /// Boolean indicating if identity is valid
    #[wasm_bindgen(js_name = verifyIdentity)]
    pub async fn verify_identity_js(&self, identity_id: &str) -> Result<bool, JsValue> {
        let valid = self
            .db
            .auth()
            .verify_identity(identity_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to verify identity: {}", e)))?;

        Ok(valid)
    }

    /// Get identity information
    ///
    /// # Arguments
    /// * `identity_id` - The identity public key
    ///
    /// # Returns
    /// Object with identity info, or null if not found
    #[wasm_bindgen(js_name = getIdentity)]
    pub async fn get_identity_js(&self, identity_id: &str) -> Result<JsValue, JsValue> {
        match self.db.auth().get_identity(identity_id) {
            Ok(Some(identity)) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"id".into(), &JsValue::from_str(&identity.public_key))?;
                js_sys::Reflect::set(
                    &obj,
                    &"createdAt".into(),
                    &JsValue::from_str(&identity.created_at.to_rfc3339()),
                )?;
                js_sys::Reflect::set(
                    &obj,
                    &"difficulty".into(),
                    &JsValue::from_f64(identity.difficulty as f64),
                )?;
                Ok(obj.into())
            }
            Ok(None) => Ok(JsValue::NULL),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get identity: {}", e))),
        }
    }

    /// Get a workspace handle
    ///
    /// # Arguments
    /// * `name` - Workspace name
    ///
    /// # Returns
    /// Workspace object for operations within that namespace
    #[wasm_bindgen(js_name = workspace)]
    pub fn workspace_js(&self, name: &str) -> WorkspaceHandle {
        WorkspaceHandle {
            db: self.db.clone(),
            namespace: name.to_string(),
        }
    }

    /// Store a value with causal parent links in the graph.
    ///
    /// This establishes causal relationships in the graph while storing the value.
    /// Use this when a distinction is caused by prior distinctions.
    ///
    /// # Arguments
    /// * `namespace` - The namespace
    /// * `key` - The key
    /// * `value` - JSON value to store
    /// * `parentKeys` - Array of parent keys that caused this distinction
    #[wasm_bindgen(js_name = putWithCausalLinks)]
    pub async fn put_with_causal_links_js(
        &self,
        namespace: &str,
        key: &str,
        value: JsValue,
        parent_keys: Vec<String>,
    ) -> Result<JsValue, JsValue> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON value: {}", e)))?;

        let versioned = self
            .db
            .put_with_causal_links(namespace, key, json_value, parent_keys)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store with causal links: {}", e)))?;

        versioned_to_js(&versioned)
    }

    /// Store a value with TTL (time-to-live) in seconds
    ///
    /// The value will be automatically deleted after the specified number of seconds.
    /// This is useful for temporary data, sessions, cache entries, etc.
    ///
    /// # Arguments
    /// * `namespace` - The namespace
    /// * `key` - The key
    /// * `value` - JSON value to store
    /// * `ttl_seconds` - Time-to-live in seconds
    #[wasm_bindgen(js_name = putWithTtl)]
    pub async fn put_with_ttl_js(
        &self,
        namespace: &str,
        key: &str,
        value: JsValue,
        ttl_seconds: u64,
    ) -> Result<JsValue, JsValue> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON value: {}", e)))?;

        let versioned = self
            .db
            .put_with_ttl(namespace, key, json_value.clone(), ttl_seconds)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store value with TTL: {}", e)))?;

        // Auto-save to IndexedDB if persistence is enabled
        if let Err(e) = self.save_to_storage(namespace, key, &json_value).await {
            web_sys::console::warn_1(&format!("Failed to save to IndexedDB: {:?}", e).into());
        }

        versioned_to_js(&versioned)
    }

    /// Store content with TTL and automatic distinction-based embedding
    ///
    /// Combines semantic storage with automatic expiration.
    #[wasm_bindgen(js_name = putSimilarWithTtl)]
    pub async fn put_similar_with_ttl_js(
        &self,
        namespace: &str,
        key: &str,
        content: JsValue,
        ttl_seconds: u64,
        metadata: Option<JsValue>,
    ) -> Result<(), JsValue> {
        let json_content: JsonValue = serde_wasm_bindgen::from_value(content)
            .map_err(|e| JsValue::from_str(&format!("Invalid content: {}", e)))?;

        let meta = metadata.and_then(|m| serde_wasm_bindgen::from_value(m).ok());

        // First store with semantic embedding
        self.db
            .put_similar(namespace, key, json_content.clone(), meta.clone())
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store similar content: {}", e)))?;

        // Then set TTL by re-putting with TTL
        self.db
            .put_with_ttl(namespace, key, json_content, ttl_seconds)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to set TTL: {}", e)))?;

        Ok(())
    }

    /// Clean up all expired TTL values
    ///
    /// Returns the number of items that were removed.
    #[wasm_bindgen(js_name = cleanupExpired)]
    pub async fn cleanup_expired_js(&self) -> Result<usize, JsValue> {
        let count = self
            .db
            .cleanup_expired()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to cleanup expired: {}", e)))?;
        Ok(count)
    }

    /// Get remaining TTL for a key in seconds
    ///
    /// Returns null if the key doesn't exist or has no TTL.
    #[wasm_bindgen(js_name = getTtlRemaining)]
    pub async fn get_ttl_remaining_js(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<JsValue, JsValue> {
        let ttl = self
            .db
            .get_ttl_remaining(namespace, key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get TTL: {}", e)))?;

        match ttl {
            Some(seconds) => Ok(JsValue::from_f64(seconds as f64)),
            None => Ok(JsValue::NULL),
        }
    }

    /// List keys expiring soon (within the given seconds)
    ///
    /// Returns an array of objects with namespace, key, and secondsRemaining.
    #[wasm_bindgen(js_name = listExpiringSoon)]
    pub async fn list_expiring_soon_js(&self, within_seconds: u64) -> Result<JsValue, JsValue> {
        let expiring = self.db.list_expiring_soon(within_seconds).await;

        let js_array = js_sys::Array::new();
        for item in expiring {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"namespace".into(), &JsValue::from_str(&item.0))?;
            js_sys::Reflect::set(&obj, &"key".into(), &JsValue::from_str(&item.1))?;
            js_sys::Reflect::set(
                &obj,
                &"secondsRemaining".into(),
                &JsValue::from_f64(item.2 as f64),
            )?;
            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Check if two distinctions are causally connected
    ///
    /// Returns true if there is a causal path between the two distinctions.
    #[wasm_bindgen(js_name = areConnected)]
    pub async fn are_connected_js(
        &self,
        namespace: &str,
        key_a: &str,
        key_b: &str,
    ) -> Result<bool, JsValue> {
        let connected = self
            .db
            .are_connected(namespace, key_a, key_b)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to check connection: {}", e)))?;
        Ok(connected)
    }

    /// Get the causal connection path between two distinctions
    ///
    /// Returns an array of distinction IDs representing the path, or null if not connected.
    #[wasm_bindgen(js_name = getConnectionPath)]
    pub async fn get_connection_path_js(
        &self,
        namespace: &str,
        key_a: &str,
        key_b: &str,
    ) -> Result<JsValue, JsValue> {
        let path = self
            .db
            .get_connection_path(namespace, key_a, key_b)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get connection path: {}", e)))?;

        match path {
            Some(path_vec) => {
                let js_array = js_sys::Array::new();
                for id in path_vec {
                    js_array.push(&JsValue::from_str(&id));
                }
                Ok(js_array.into())
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// Get the most highly-connected distinctions
    ///
    /// Returns an array of objects with distinction info and connection scores.
    #[wasm_bindgen(js_name = getHighlyConnected)]
    pub async fn get_highly_connected_js(
        &self,
        namespace: Option<String>,
        k: usize,
    ) -> Result<JsValue, JsValue> {
        let results = self
            .db
            .get_highly_connected(namespace.as_deref(), k)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get highly connected: {}", e)))?;

        let js_array = js_sys::Array::new();
        for dist in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &"namespace".into(),
                &JsValue::from_str(&dist.namespace),
            )?;
            js_sys::Reflect::set(&obj, &"key".into(), &JsValue::from_str(&dist.key))?;
            js_sys::Reflect::set(
                &obj,
                &"connectionScore".into(),
                &JsValue::from_f64(dist.connection_score as f64),
            )?;

            // Convert parents to JS array
            let parents_array = js_sys::Array::new();
            for parent in &dist.parents {
                parents_array.push(&JsValue::from_str(parent));
            }
            js_sys::Reflect::set(&obj, &"parents".into(), &parents_array)?;

            // Convert children to JS array
            let children_array = js_sys::Array::new();
            for child in &dist.children {
                children_array.push(&JsValue::from_str(child));
            }
            js_sys::Reflect::set(&obj, &"children".into(), &children_array)?;

            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Find similar distinctions that are not causally connected
    ///
    /// These pairs are candidates for synthesis.
    #[wasm_bindgen(js_name = findSimilarUnconnectedPairs)]
    pub async fn find_similar_unconnected_pairs_js(
        &self,
        namespace: Option<String>,
        k: usize,
        threshold: f32,
    ) -> Result<JsValue, JsValue> {
        let pairs = self
            .db
            .find_similar_unconnected_pairs(namespace.as_deref(), k, threshold)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to find pairs: {}", e)))?;

        let js_array = js_sys::Array::new();
        for pair in pairs {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &"namespaceA".into(),
                &JsValue::from_str(&pair.namespace_a),
            )?;
            js_sys::Reflect::set(&obj, &"keyA".into(), &JsValue::from_str(&pair.key_a))?;
            js_sys::Reflect::set(
                &obj,
                &"namespaceB".into(),
                &JsValue::from_str(&pair.namespace_b),
            )?;
            js_sys::Reflect::set(&obj, &"keyB".into(), &JsValue::from_str(&pair.key_b))?;
            js_sys::Reflect::set(
                &obj,
                &"similarityScore".into(),
                &JsValue::from_f64(pair.similarity_score as f64),
            )?;
            js_array.push(&obj);
        }

        Ok(js_array.into())
    }

    /// Generate random walk combinations for dream-phase creative synthesis
    ///
    /// Performs random walks through the causal graph to discover novel combinations.
    #[wasm_bindgen(js_name = randomWalkCombinations)]
    pub async fn random_walk_combinations_js(
        &self,
        n: usize,
        steps: usize,
    ) -> Result<JsValue, JsValue> {
        let combinations = self
            .db
            .random_walk_combinations(n, steps)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to generate combinations: {}", e)))?;

        let js_array = js_sys::Array::new();
        for combo in combinations {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &"startNamespace".into(),
                &JsValue::from_str(&combo.start_namespace),
            )?;
            js_sys::Reflect::set(
                &obj,
                &"startKey".into(),
                &JsValue::from_str(&combo.start_key),
            )?;
            js_sys::Reflect::set(
                &obj,
                &"endNamespace".into(),
                &JsValue::from_str(&combo.end_namespace),
            )?;
            js_sys::Reflect::set(&obj, &"endKey".into(), &JsValue::from_str(&combo.end_key))?;

            // Convert path to JS array
            let path_array = js_sys::Array::new();
            for step in &combo.path {
                path_array.push(&JsValue::from_str(step));
            }
            js_sys::Reflect::set(&obj, &"path".into(), &path_array)?;

            js_sys::Reflect::set(
                &obj,
                &"noveltyScore".into(),
                &JsValue::from_f64(combo.novelty_score as f64),
            )?;
            js_array.push(&obj);
        }

        Ok(js_array.into())
    }
}

/// Handle for workspace-scoped operations
#[wasm_bindgen]
pub struct WorkspaceHandle {
    db: KoruDelta,
    namespace: String,
}

#[wasm_bindgen]
impl WorkspaceHandle {
    /// Get the workspace namespace
    #[wasm_bindgen(getter)]
    pub fn namespace(&self) -> String {
        self.namespace.clone()
    }

    /// Store a value in the workspace
    #[wasm_bindgen(js_name = put)]
    pub async fn put_js(&self, key: &str, value: JsValue) -> Result<JsValue, JsValue> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON value: {}", e)))?;

        let versioned = self
            .db
            .put(&self.namespace, key, json_value)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store value: {}", e)))?;

        versioned_to_js(&versioned)
    }

    /// Retrieve a value from the workspace
    #[wasm_bindgen(js_name = get)]
    pub async fn get_js(&self, key: &str) -> Result<JsValue, JsValue> {
        let versioned = self
            .db
            .get(&self.namespace, key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to retrieve value: {}", e)))?;

        versioned_to_js(&versioned)
    }

    /// Delete a key from the workspace
    #[wasm_bindgen(js_name = delete)]
    pub async fn delete_js(&self, key: &str) -> Result<(), JsValue> {
        self.db
            .delete(&self.namespace, key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {}", e)))?;
        Ok(())
    }

    /// List all keys in the workspace
    #[wasm_bindgen(js_name = listKeys)]
    pub async fn list_keys_js(&self) -> Result<Vec<String>, JsValue> {
        Ok(self.db.list_keys(&self.namespace).await)
    }

    /// String representation
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("<Workspace '{}'>", self.namespace)
    }
}

/// Convert VersionedValue to JavaScript object
fn versioned_to_js(versioned: &VersionedValue) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();

    let value_js = serde_wasm_bindgen::to_value(versioned.value())
        .map_err(|e| JsValue::from_str(&format!("Failed to convert value: {}", e)))?;
    js_sys::Reflect::set(&obj, &"value".into(), &value_js)?;

    js_sys::Reflect::set(
        &obj,
        &"timestamp".into(),
        &JsValue::from_str(&versioned.timestamp().to_rfc3339()),
    )?;
    js_sys::Reflect::set(
        &obj,
        &"versionId".into(),
        &JsValue::from_str(versioned.version_id()),
    )?;

    if let Some(prev) = versioned.previous_version() {
        js_sys::Reflect::set(&obj, &"previousVersion".into(), &JsValue::from_str(prev))?;
    }

    Ok(obj.into())
}

/// Convert HistoryEntry to JavaScript object
fn history_entry_to_js(entry: &HistoryEntry) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();

    let value_js = serde_wasm_bindgen::to_value(&entry.value)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert value: {}", e)))?;
    js_sys::Reflect::set(&obj, &"value".into(), &value_js)?;

    js_sys::Reflect::set(
        &obj,
        &"timestamp".into(),
        &JsValue::from_str(&entry.timestamp.to_rfc3339()),
    )?;
    js_sys::Reflect::set(
        &obj,
        &"versionId".into(),
        &JsValue::from_str(&entry.version_id),
    )?;

    Ok(obj.into())
}

/// Initialize panic hook for better error messages in the browser console
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
