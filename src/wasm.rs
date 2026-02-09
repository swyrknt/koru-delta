//! WASM bindings for KoruDelta
//!
//! This module provides JavaScript-friendly bindings for using KoruDelta
//! in browser and edge compute environments.
//!
//! # Features
//! - Full database operations (put, get, history, time-travel)
//! - Namespace management
//! - Vector embedding storage and similarity search
//! - Query engine with filters and aggregation
//! - Views for materialized queries
//!
//! # Usage
//! ```javascript
//! import init, { KoruDeltaWasm } from 'koru-delta';
//!
//! await init();
//! const db = await KoruDeltaWasm.new();
//!
//! await db.put('users', 'alice', { name: 'Alice', age: 30 });
//! const user = await db.get('users', 'alice');
//! ```

use crate::vector::{Vector, VectorSearchOptions};
use crate::{DeltaError, HistoryEntry, KoruDelta, VersionedValue, ViewDefinition};
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

/// WASM-friendly wrapper around KoruDelta
#[wasm_bindgen]
pub struct KoruDeltaWasm {
    db: KoruDelta,
}

#[wasm_bindgen]
impl KoruDeltaWasm {
    /// Create a new KoruDelta database instance
    ///
    /// Returns a Promise that resolves to a new database instance.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const db = await KoruDeltaWasm.new();
    /// ```
    pub async fn new() -> Result<KoruDeltaWasm, JsValue> {
        let db = KoruDelta::start()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to start database: {}", e)))?;

        Ok(KoruDeltaWasm { db })
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
            .put(namespace, key, json_value)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store value: {}", e)))?;

        versioned_to_js(&versioned)
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
        let versioned = self
            .db
            .get(namespace, key)
            .await
            .map_err(|e| match e {
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
                DeltaError::NoValueAtTimestamp { .. } => {
                    JsValue::from_str("No value exists at that timestamp")
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
    #[wasm_bindgen(js_name = delete)]
    pub async fn delete_js(&self, namespace: &str, key: &str) -> Result<(), JsValue> {
        self.db
            .delete(namespace, key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {}", e)))?;
        Ok(())
    }

    /// Check if a key exists
    #[wasm_bindgen(js_name = contains)]
    pub async fn contains_js(&self, namespace: &str, key: &str) -> bool {
        self.db.contains_key(namespace, key).await
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
        vector: Vec<f32>,
        model: Option<String>,
    ) -> Result<(), JsValue> {
        let vec = Vector::new(vector, model.as_deref().unwrap_or("default"));

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
        query: Vec<f32>,
        limit: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        let query_vec = Vector::new(query, "query");
        
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
            js_sys::Reflect::set(&obj, &"namespace".into(), &JsValue::from_str(&result.namespace))?;
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
    pub async fn create_view_js(
        &self,
        name: &str,
        source_namespace: &str,
    ) -> Result<(), JsValue> {
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
            js_sys::Reflect::set(&obj, &"source".into(), &JsValue::from_str(&view.source_collection))?;
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
                        value: value.clone() 
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
