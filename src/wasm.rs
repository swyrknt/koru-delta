//! WASM bindings for KoruDelta
//!
//! This module provides JavaScript-friendly bindings for using KoruDelta
//! in browser and edge compute environments.

use crate::{DeltaError, HistoryEntry, KoruDelta, VersionedValue};
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
        // Convert JsValue to serde_json::Value
        let json_value: JsonValue = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON value: {}", e)))?;

        // Store in database
        let versioned = self
            .db
            .put(namespace, key, json_value)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to store value: {}", e)))?;

        // Convert back to JsValue
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

        // Convert history entries to JS array
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
    /// The value at that point in time (without version metadata)
    #[wasm_bindgen(js_name = getAt)]
    pub async fn get_at_js(
        &self,
        namespace: &str,
        key: &str,
        timestamp_iso: &str,
    ) -> Result<JsValue, JsValue> {
        // Parse timestamp
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

        // Convert JSON value directly to JS
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
}

/// Convert VersionedValue to JavaScript object
fn versioned_to_js(versioned: &VersionedValue) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();

    // Convert value
    let value_js = serde_wasm_bindgen::to_value(versioned.value())
        .map_err(|e| JsValue::from_str(&format!("Failed to convert value: {}", e)))?;
    js_sys::Reflect::set(&obj, &"value".into(), &value_js)?;

    // Add metadata
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

    // Note: HistoryEntry doesn't have previous_version field

    Ok(obj.into())
}

/// Initialize panic hook for better error messages in the browser console
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
