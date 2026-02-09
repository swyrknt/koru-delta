//! IndexedDB persistence for KoruDelta WASM
//!
//! This module provides persistent storage using the browser's IndexedDB API.
//! It enables the database to survive page refreshes and browser restarts.
//!
//! # Features
//! - Auto-save on data changes
//! - Auto-load on startup
//! - Graceful fallback to memory-only if IndexedDB unavailable
//! - Efficient batch operations
//!
//! # Usage
//! The storage is automatically initialized when calling `KoruDeltaWasm::new_persistent()`.

use js_sys::{Array, Promise};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    IdbDatabase, IdbOpenDbRequest, IdbTransactionMode,
};

/// Convert an IdbRequest to a JsFuture by creating a Promise wrapper
fn idb_request_to_future(request: &web_sys::IdbRequest) -> Result<JsFuture, JsValue> {
    // Create a Promise that resolves/rejects based on the request
    let promise = Promise::new(&mut |resolve, reject| {
        let on_success = Closure::wrap(Box::new(move || {
            let _ = resolve.call0(&JsValue::NULL);
        }) as Box<dyn FnMut()>);
        
        let on_error = Closure::wrap(Box::new(move || {
            let _ = reject.call0(&JsValue::NULL);
        }) as Box<dyn FnMut()>);
        
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        
        // Forget the closures so they stay alive
        on_success.forget();
        on_error.forget();
    });
    
    Ok(JsFuture::from(promise))
}

const DB_NAME: &str = "koru-delta";
const DB_VERSION: u32 = 1;
const STORE_DATA: &str = "data";
const STORE_METADATA: &str = "metadata";

/// Persistent storage backend using IndexedDB
#[derive(Debug, Clone)]
pub struct IndexedDbStorage {
    db: Option<IdbDatabase>,
    memory_fallback: bool,
}

/// Serialized record for IndexedDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredRecord {
    namespace: String,
    key: String,
    value: serde_json::Value,
    timestamp: String, // ISO 8601
    version_id: String,
    previous_version: Option<String>,
}

/// Database metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseMetadata {
    created_at: String,
    last_modified: String,
    version: String,
    namespace_count: usize,
}

impl IndexedDbStorage {
    /// Create a new IndexedDB storage instance
    ///
    /// Attempts to open IndexedDB, falls back to memory-only if unavailable
    pub async fn new() -> Result<Self, JsValue> {
        match Self::open_database().await {
            Ok(db) => {
                web_sys::console::log_1(&"IndexedDB: Persistence enabled".into());
                Ok(Self {
                    db: Some(db),
                    memory_fallback: false,
                })
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("IndexedDB unavailable, using memory-only: {:?}", e).into());
                Ok(Self {
                    db: None,
                    memory_fallback: true,
                })
            }
        }
    }

    /// Check if persistence is available
    pub fn is_persistent(&self) -> bool {
        self.db.is_some() && !self.memory_fallback
    }

    /// Check if using memory fallback
    #[allow(dead_code)]
    pub fn is_memory_fallback(&self) -> bool {
        self.memory_fallback
    }

    /// Save a record to IndexedDB
    pub async fn save_record(
        &self,
        namespace: &str,
        key: &str,
        value: &serde_json::Value,
        timestamp: &chrono::DateTime<chrono::Utc>,
        version_id: &str,
        previous_version: Option<&str>,
    ) -> Result<(), JsValue> {
        if self.memory_fallback {
            return Ok(());
        }

        let db = self.db.as_ref().ok_or("Database not available")?;

        let record = StoredRecord {
            namespace: namespace.to_string(),
            key: key.to_string(),
            value: value.clone(),
            timestamp: timestamp.to_rfc3339(),
            version_id: version_id.to_string(),
            previous_version: previous_version.map(|s| s.to_string()),
        };

        let json = serde_json::to_string(&record)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        let transaction = db
            .transaction_with_str_and_mode(STORE_DATA, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Transaction error: {:?}", e)))?;

        let store = transaction
            .object_store(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Object store error: {:?}", e)))?;

        let full_key = format!("{}:{}", namespace, key);
        
        // Use put (upsert) to handle updates
        let request = store
            .put_with_key(&JsValue::from_str(&json), &JsValue::from_str(&full_key))
            .map_err(|e| JsValue::from_str(&format!("Put error: {:?}", e)))?;

        // Wait for the request to complete
        let _: JsValue = idb_request_to_future(&request)?.await?;

        // Update metadata
        self.update_metadata(db).await?;

        Ok(())
    }

    /// Load all records from IndexedDB
    pub async fn load_all_records(
        &self,
    ) -> Result<Vec<(String, String, serde_json::Value, chrono::DateTime<chrono::Utc>, String, Option<String>)>, JsValue> {
        if self.memory_fallback {
            return Ok(Vec::new());
        }

        let db = self.db.as_ref().ok_or("Database not available")?;

        let transaction = db
            .transaction_with_str(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Transaction error: {:?}", e)))?;

        let store = transaction
            .object_store(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Object store error: {:?}", e)))?;

        let request = store
            .get_all()
            .map_err(|e| JsValue::from_str(&format!("Get all error: {:?}", e)))?;

        let result = idb_request_to_future(&request)?.await?;
        let array: Array = result.dyn_into().map_err(|_| JsValue::from_str("Expected array"))?;

        let mut records = Vec::new();

        for i in 0..array.length() {
            let item = array.get(i);
            if let Some(json_str) = item.as_string() {
                if let Ok(record) = serde_json::from_str::<StoredRecord>(&json_str) {
                    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&record.timestamp) {
                        records.push((
                            record.namespace,
                            record.key,
                            record.value,
                            timestamp.with_timezone(&chrono::Utc),
                            record.version_id,
                            record.previous_version,
                        ));
                    }
                }
            }
        }

        web_sys::console::log_1(&format!("IndexedDB: Loaded {} records", records.len()).into());

        Ok(records)
    }

    /// Delete a record from IndexedDB
    pub async fn delete_record(&self, namespace: &str, key: &str) -> Result<(), JsValue> {
        if self.memory_fallback {
            return Ok(());
        }

        let db = self.db.as_ref().ok_or("Database not available")?;

        let transaction = db
            .transaction_with_str_and_mode(STORE_DATA, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Transaction error: {:?}", e)))?;

        let store = transaction
            .object_store(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Object store error: {:?}", e)))?;

        let full_key = format!("{}:{}", namespace, key);
        let request = store
            .delete(&JsValue::from_str(&full_key))
            .map_err(|e| JsValue::from_str(&format!("Delete error: {:?}", e)))?;

        let _: JsValue = idb_request_to_future(&request)?.await?;

        self.update_metadata(db).await?;

        Ok(())
    }

    /// Clear all data from IndexedDB
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        if self.memory_fallback {
            return Ok(());
        }

        let db = self.db.as_ref().ok_or("Database not available")?;

        let transaction = db
            .transaction_with_str_and_mode(STORE_DATA, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Transaction error: {:?}", e)))?;

        let store = transaction
            .object_store(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Object store error: {:?}", e)))?;

        let request = store
            .clear()
            .map_err(|e| JsValue::from_str(&format!("Clear error: {:?}", e)))?;

        let _: JsValue = idb_request_to_future(&request)?.await?;

        Ok(())
    }

    /// Get database statistics
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> Result<(usize, usize), JsValue> {
        if self.memory_fallback {
            return Ok((0, 0));
        }

        let db = self.db.as_ref().ok_or("Database not available")?;

        let transaction = db
            .transaction_with_str(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Transaction error: {:?}", e)))?;

        let store = transaction
            .object_store(STORE_DATA)
            .map_err(|e| JsValue::from_str(&format!("Object store error: {:?}", e)))?;

        let request = store
            .count()
            .map_err(|e| JsValue::from_str(&format!("Count error: {:?}", e)))?;

        let result = idb_request_to_future(&request)?.await?;
        let count = result.as_f64().unwrap_or(0.0) as usize;

        // Count unique namespaces
        let all_records = self.load_all_records().await?;
        let namespaces: std::collections::HashSet<_> = all_records
            .iter()
            .map(|(ns, _, _, _, _, _)| ns.clone())
            .collect();

        Ok((count, namespaces.len()))
    }

    /// Open the IndexedDB database
    async fn open_database() -> Result<IdbDatabase, JsValue> {
        let window = web_sys::window().ok_or("No window available")?;
        let indexed_db = window
            .indexed_db()
            .map_err(|_| JsValue::from_str("IndexedDB not supported"))?
            .ok_or("IndexedDB not available")?;

        let open_request: IdbOpenDbRequest = indexed_db
            .open_with_u32(DB_NAME, DB_VERSION)
            .map_err(|e| JsValue::from_str(&format!("Open error: {:?}", e)))?;

        // Set up upgrade needed handler
        let on_upgrade = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            let db: IdbDatabase = request.result().unwrap().dyn_into().unwrap();

            // Create object stores if they don't exist
            let store_names = db.object_store_names();
            let has_data_store = (0..store_names.length()).any(|i| {
                store_names.get(i).map_or(false, |name| name == STORE_DATA)
            });
            let has_meta_store = (0..store_names.length()).any(|i| {
                store_names.get(i).map_or(false, |name| name == STORE_METADATA)
            });

            if !has_data_store {
                db.create_object_store(STORE_DATA)
                    .expect("Failed to create data store");
            }

            if !has_meta_store {
                db.create_object_store(STORE_METADATA)
                    .expect("Failed to create metadata store");
            }

            web_sys::console::log_1(&"IndexedDB: Database upgraded".into());
        }) as Box<dyn FnMut(_)>);

        open_request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
        on_upgrade.forget();

        // Wait for the open request to complete using a simple poll approach
        // This is necessary because IndexedDB events don't work well with async/await
        let open_request_rc = Rc::new(std::cell::RefCell::new(Some(open_request)));
        
        loop {
            // Small delay between checks
            let _ = wasm_bindgen_futures::JsFuture::from(Promise::new(&mut |resolve, _reject| {
                let window = web_sys::window().unwrap();
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments(
                    &resolve,
                    10, // 10ms poll interval
                    &Array::new(),
                );
            })).await;

            let req = open_request_rc.borrow();
            if let Some(req) = req.as_ref() {
                // Check if the request is ready
                if let Ok(result) = req.result() {
                    if !result.is_null() && !result.is_undefined() {
                        let db: IdbDatabase = result.dyn_into().map_err(|_| JsValue::from_str("Expected database"))?;
                        web_sys::console::log_1(&"IndexedDB: Database opened successfully".into());
                        return Ok(db);
                    }
                }
                // Check for error
                if let Ok(error) = req.error() {
                    if let Some(err) = error {
                        return Err(JsValue::from_str(&format!("IndexedDB error: {:?}", err)));
                    }
                }
            }
        }
    }

    /// Update metadata after changes
    async fn update_metadata(&self, db: &IdbDatabase) -> Result<(), JsValue> {
        let metadata = DatabaseMetadata {
            created_at: chrono::Utc::now().to_rfc3339(),
            last_modified: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            namespace_count: 0, // Could count this if needed
        };

        let json = serde_json::to_string(&metadata)
            .map_err(|e| JsValue::from_str(&format!("Metadata serialization error: {}", e)))?;

        let transaction = db
            .transaction_with_str_and_mode(STORE_METADATA, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Metadata transaction error: {:?}", e)))?;

        let store = transaction
            .object_store(STORE_METADATA)
            .map_err(|e| JsValue::from_str(&format!("Metadata store error: {:?}", e)))?;

        let request = store
            .put_with_key(&JsValue::from_str(&json), &JsValue::from_str("meta"))
            .map_err(|e| JsValue::from_str(&format!("Metadata put error: {:?}", e)))?;

        let _: JsValue = idb_request_to_future(&request)?.await?;

        Ok(())
    }
}

/// Check if IndexedDB is supported in the current environment
pub fn is_indexeddb_supported() -> bool {
    if let Some(window) = web_sys::window() {
        if let Ok(idb) = window.indexed_db() {
            return idb.is_some();
        }
    }
    false
}
