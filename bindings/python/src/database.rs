//! Python wrapper for KoruDelta database with LCA Architecture
//!
//! This module provides Python bindings that expose the full LCA (Local Causal Agent)
//! architecture, including:
//! - Synthesis-based operations (ΔNew = ΔLocal_Root ⊕ ΔAction)
//! - Distinction-based vector embeddings
//! - Query and view support
//! - Identity management

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3_asyncio::tokio::future_into_py;
use pyo3::types::{PyDict, PyList, PyTuple};

use crate::to_python_error;
use crate::types::{json_to_pyobject, pyobject_to_json};
use koru_delta::vector::{Vector, VectorSearchOptions};
use koru_delta::KoruDelta;

/// Python wrapper for KoruDelta database
#[pyclass(name = "Database")]
pub struct PyDatabase {
    db: Arc<KoruDelta>,
}

#[pymethods]
impl PyDatabase {
    /// Create a new in-memory database
    #[new]
    fn new() -> PyResult<Self> {
        Err(PyRuntimeError::new_err(
            "Use Database.create() to initialize",
        ))
    }

    /// Create and initialize a database (async factory)
    #[staticmethod]
    fn create(py: Python<'_>) -> PyResult<&PyAny> {
        future_into_py(py, async move {
            let db = KoruDelta::start().await.map_err(to_python_error)?;
            Ok(PyDatabase { db: Arc::new(db) })
        })
    }

    /// Store a value
    fn put<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        value: &'py PyAny,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        let json_value = pyobject_to_json(value)?;

        future_into_py(py, async move {
            db.put(ns, k, json_value)
                .await
                .map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Store content with automatic distinction-based embedding
    ///
    /// This is the simplified API for semantic storage. The embedding is
    /// synthesized from the content structure using distinction calculus.
    #[pyo3(signature = (namespace, key, content, metadata = None))]
    fn put_similar<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        content: &'py PyAny,
        metadata: Option<PyObject>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        let json_content = pyobject_to_json(content)?;
        let meta = metadata.and_then(|m| {
            Python::with_gil(|py| {
                pyobject_to_json(m.as_ref(py)).ok()
            })
        });

        future_into_py(py, async move {
            db.put_similar(ns, k, json_content, meta)
                .await
                .map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Find similar content using semantic search
    ///
    /// Searches for content similar to the provided query using
    /// distinction-based embeddings and cosine similarity.
    #[pyo3(signature = (namespace, query, top_k = 10))]
    fn find_similar<'py>(
        &self,
        py: Python<'py>,
        namespace: Option<&str>,
        query: &'py PyAny,
        top_k: usize,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let json_query = pyobject_to_json(query)?;
        let ns = namespace.map(|s| s.to_string());

        future_into_py(py, async move {
            let results = db
                .find_similar(ns.as_deref(), json_query, top_k)
                .await
                .map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for result in results {
                    let dict = PyDict::new(py);
                    dict.set_item("namespace", &result.namespace).ok();
                    dict.set_item("key", &result.key).ok();
                    dict.set_item("score", result.score).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Store multiple values as a batch
    #[pyo3(signature = (items))]
    fn put_batch<'py>(
        &self,
        py: Python<'py>,
        items: &'py PyList,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        
        let mut batch_items: Vec<(String, String, serde_json::Value)> = Vec::new();
        for item in items.iter() {
            let tuple = item.downcast::<PyTuple>()
                .map_err(|_| PyValueError::new_err("Each item must be a tuple of (namespace, key, value)"))?;
            
            if tuple.len() != 3 {
                return Err(PyValueError::new_err("Each tuple must have exactly 3 elements: (namespace, key, value)"));
            }
            
            let namespace: String = tuple.get_item(0)?.extract()?;
            let key: String = tuple.get_item(1)?.extract()?;
            let value = pyobject_to_json(tuple.get_item(2)?)?;
            
            batch_items.push((namespace, key, value));
        }

        let item_count = batch_items.len();
        future_into_py(py, async move {
            db.put_batch(batch_items)
                .await
                .map_err(to_python_error)?;
            Ok(item_count)
        })
    }

    /// Store multiple values in a single namespace (simplified API)
    #[pyo3(signature = (namespace, items))]
    fn put_batch_in_ns<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        items: &'py PyList,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        
        let mut batch_items: Vec<(String, serde_json::Value)> = Vec::new();
        for item in items.iter() {
            let tuple = item.downcast::<PyTuple>()
                .map_err(|_| PyValueError::new_err("Each item must be a tuple of (key, value)"))?;
            
            if tuple.len() != 2 {
                return Err(PyValueError::new_err("Each tuple must have exactly 2 elements: (key, value)"));
            }
            
            let key: String = tuple.get_item(0)?.extract()?;
            let value = pyobject_to_json(tuple.get_item(1)?)?;
            
            batch_items.push((key, value));
        }

        let item_count = batch_items.len();
        future_into_py(py, async move {
            db.put_batch_in_ns(ns, batch_items)
                .await
                .map_err(to_python_error)?;
            Ok(item_count)
        })
    }

    /// Retrieve a value
    fn get<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();

        future_into_py(py, async move {
            let versioned = db.get(ns, k).await.map_err(to_python_error)?;
            Python::with_gil(|py| Ok(json_to_pyobject(py, versioned.value())))
        })
    }

    /// Get value at specific timestamp (time travel)
    fn get_at<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        timestamp: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        
        let ts = chrono::DateTime::parse_from_rfc3339(timestamp)
            .map_err(|e| PyValueError::new_err(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        future_into_py(py, async move {
            let versioned = db
                .get_at(&ns, &k, ts)
                .await
                .map_err(to_python_error)?;
            Python::with_gil(|py| Ok(json_to_pyobject(py, versioned.value())))
        })
    }

    /// Get history for a key
    fn history<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();

        future_into_py(py, async move {
            let entries = db.history(&ns, &k).await.map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for entry in entries {
                    let dict = PyDict::new(py);
                    dict.set_item("value", json_to_pyobject(py, &entry.value)).ok();
                    dict.set_item("timestamp", entry.timestamp.to_rfc3339()).ok();
                    dict.set_item("version_id", &entry.version_id).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Store a vector embedding with explicit vector data
    #[pyo3(signature = (namespace, key, embedding, model, metadata = None))]
    fn embed<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        embedding: Vec<f32>,
        model: &str,
        metadata: Option<PyObject>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        let vec = Vector::new(embedding, model);
        let meta = metadata.and_then(|m| {
            Python::with_gil(|py| {
                pyobject_to_json(m.as_ref(py)).ok()
            })
        });

        future_into_py(py, async move {
            db.embed(ns, k, vec, meta)
                .await
                .map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Search for similar vectors
    #[pyo3(signature = (namespace, query, top_k = 10, threshold = 0.0, model_filter = None))]
    fn similar<'py>(
        &self,
        py: Python<'py>,
        namespace: Option<&str>,
        query: Vec<f32>,
        top_k: usize,
        threshold: f32,
        model_filter: Option<String>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let query_vec = Vector::new(query, "query");
        let ns = namespace.map(|s| s.to_string());
        
        let opts = VectorSearchOptions::new()
            .top_k(top_k)
            .threshold(threshold);
        
        let opts = if let Some(filter) = model_filter {
            opts.model_filter(filter)
        } else {
            opts
        };

        future_into_py(py, async move {
            let results = db
                .embed_search(ns.as_deref(), &query_vec, opts)
                .await
                .map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for result in results {
                    let dict = PyDict::new(py);
                    dict.set_item("namespace", &result.namespace).ok();
                    dict.set_item("key", &result.key).ok();
                    dict.set_item("score", result.score).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Query data with filters
    #[pyo3(signature = (namespace, filters = None, sort = None, limit = None, offset = None))]
    fn query<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        filters: Option<PyObject>,
        sort: Option<PyObject>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();

        // Build query from Python arguments
        let mut query = koru_delta::query::Query::default();
        query.limit = limit;
        query.offset = offset;

        // Parse filters if provided
        if let Some(filters_obj) = filters {
            if let Ok(filters_list) = filters_obj.downcast::<PyList>(py) {
                for filter_obj in filters_list.iter() {
                    if let Ok(filter_dict) = filter_obj.downcast::<PyDict>() {
                        if let (Ok(Some(field_any)), Ok(Some(op_any)), Ok(Some(value))) = (
                            filter_dict.get_item("field"),
                            filter_dict.get_item("op"),
                            filter_dict.get_item("value"),
                        ) {
                            let field = field_any.extract::<String>().ok();
                            let op = op_any.extract::<String>().ok();
                            if field.is_none() || op.is_none() {
                                continue;
                            }
                            let field = field.unwrap();
                            let op = op.unwrap();
                            let json_value = pyobject_to_json(value).unwrap_or(serde_json::Value::Null);
                            let filter = match op.as_str() {
                                "eq" => koru_delta::query::Filter::eq(field, json_value),
                                "ne" => koru_delta::query::Filter::ne(field, json_value),
                                "gt" => koru_delta::query::Filter::gt(field, json_value),
                                "gte" => koru_delta::query::Filter::gte(field, json_value),
                                "lt" => koru_delta::query::Filter::lt(field, json_value),
                                "lte" => koru_delta::query::Filter::lte(field, json_value),
                                _ => koru_delta::query::Filter::eq(field, json_value),
                            };
                            query.filters.push(filter);
                        }
                    }
                }
            }
        }

        // Parse sort if provided
        if let Some(sort_obj) = sort {
            if let Ok(sort_list) = sort_obj.downcast::<PyList>(py) {
                for sort_item in sort_list.iter() {
                    if let Ok(sort_dict) = sort_item.downcast::<PyDict>() {
                        if let Ok(Some(field_any)) = sort_dict.get_item("field") {
                            if let Ok(field) = field_any.extract::<String>() {
                                let order = sort_dict.get_item("order")
                                    .ok()
                                    .flatten()
                                    .and_then(|o| o.extract::<String>().ok())
                                    .map(|o| match o.as_str() {
                                        "desc" | "Desc" => koru_delta::query::SortOrder::Desc,
                                        _ => koru_delta::query::SortOrder::Asc,
                                    })
                                    .unwrap_or(koru_delta::query::SortOrder::Asc);
                                
                                query.sort.push(koru_delta::query::SortBy { field, order });
                            }
                        }
                    }
                }
            }
        }

        future_into_py(py, async move {
            let results = db.query(&ns, query).await.map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("total_count", results.total_count).ok();
                
                let records = PyList::new(py, Vec::<PyObject>::new());
                for record in results.records {
                    let rec_dict = PyDict::new(py);
                    rec_dict.set_item("key", &record.key).ok();
                    rec_dict.set_item("value", json_to_pyobject(py, &record.value)).ok();
                    rec_dict.set_item("timestamp", record.timestamp.to_rfc3339()).ok();
                    rec_dict.set_item("version_id", &record.version_id).ok();
                    records.append(rec_dict).ok();
                }
                dict.set_item("records", records).ok();
                
                Ok(dict.to_object(py))
            })
        })
    }

    /// Create a materialized view
    #[pyo3(signature = (name, source_collection, filters = None, description = None, auto_refresh = false))]
    fn create_view<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        source_collection: &str,
        filters: Option<PyObject>,
        description: Option<String>,
        auto_refresh: bool,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let view_name = name.to_string();
        let source = source_collection.to_string();

        // Build query for view
        let mut query = koru_delta::query::Query::default();
        
        if let Some(filters_obj) = filters {
            if let Ok(filters_list) = filters_obj.downcast::<PyList>(py) {
                for filter_obj in filters_list.iter() {
                    if let Ok(filter_dict) = filter_obj.downcast::<PyDict>() {
                        if let (Ok(Some(field_any)), Ok(Some(value))) = (
                            filter_dict.get_item("field"),
                            filter_dict.get_item("value"),
                        ) {
                            let field = field_any.extract::<String>().ok();
                            if field.is_none() {
                                continue;
                            }
                            let field = field.unwrap();
                            let json_value = pyobject_to_json(value).unwrap_or(serde_json::Value::Null);
                            query.filters.push(koru_delta::query::Filter::eq(field, json_value));
                        }
                    }
                }
            }
        }

        let view_def = koru_delta::views::ViewDefinition {
            name: view_name,
            source_collection: source,
            query,
            created_at: chrono::Utc::now(),
            description,
            auto_refresh,
        };

        future_into_py(py, async move {
            db.create_view(view_def).await.map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Refresh a materialized view
    fn refresh_view<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let view_name = name.to_string();

        future_into_py(py, async move {
            db.refresh_view(&view_name).await.map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Query a materialized view
    fn query_view<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let view_name = name.to_string();

        future_into_py(py, async move {
            let results = db.query_view(&view_name).await.map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("total_count", results.total_count).ok();
                
                let records = PyList::new(py, Vec::<PyObject>::new());
                for record in results.records {
                    let rec_dict = PyDict::new(py);
                    rec_dict.set_item("key", &record.key).ok();
                    rec_dict.set_item("value", json_to_pyobject(py, &record.value)).ok();
                    rec_dict.set_item("timestamp", record.timestamp.to_rfc3339()).ok();
                    rec_dict.set_item("version_id", &record.version_id).ok();
                    records.append(rec_dict).ok();
                }
                dict.set_item("records", records).ok();
                
                Ok(dict.to_object(py))
            })
        })
    }

    /// List all views
    fn list_views<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();

        future_into_py(py, async move {
            let views = db.list_views().await;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for view in views {
                    let dict = PyDict::new(py);
                    dict.set_item("name", &view.name).ok();
                    dict.set_item("source_collection", &view.source_collection).ok();
                    dict.set_item("created_at", view.created_at.to_rfc3339()).ok();
                    dict.set_item("description", view.description.as_deref().unwrap_or("")).ok();
                    dict.set_item("auto_refresh", view.auto_refresh).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Delete a view
    fn delete_view<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let view_name = name.to_string();

        future_into_py(py, async move {
            db.delete_view(&view_name).await.map_err(to_python_error)?;
            Ok(())
        })
    }

    /// List all keys in a namespace
    fn list_keys<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();

        future_into_py(py, async move {
            let keys = db.list_keys(&ns).await;
            Python::with_gil(|py| Ok(keys.to_object(py)))
        })
    }

    /// List all namespaces
    fn list_namespaces<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();

        future_into_py(py, async move {
            let namespaces = db.list_namespaces().await;
            Python::with_gil(|py| Ok(namespaces.to_object(py)))
        })
    }

    /// Check if key exists
    fn contains<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();

        future_into_py(py, async move {
            let exists = db.contains(ns, k).await;
            Python::with_gil(|py| Ok(exists.to_object(py)))
        })
    }

    /// Delete a key (stores null as tombstone)
    fn delete<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();

        future_into_py(py, async move {
            db.delete(&ns, &k).await.map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Get database statistics
    fn stats<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let db = self.db.clone();

        future_into_py(py, async move {
            let stats = db.stats().await;
            
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("key_count", stats.key_count).ok();
                dict.set_item("total_versions", stats.total_versions).ok();
                dict.set_item("namespace_count", stats.namespace_count).ok();
                Ok(dict.to_object(py))
            })
        })
    }

    /// String representation
    fn __repr__(&self) -> String {
        "<Database instance>".to_string()
    }

    /// Get a workspace handle
    fn workspace(&self, name: &str) -> PyWorkspace {
        PyWorkspace {
            db: self.db.clone(),
            name: name.to_string(),
        }
    }

    /// Get identity manager
    fn identities(&self) -> PyIdentityManager {
        PyIdentityManager {
            db: self.db.clone(),
        }
    }

    /// Store a value with TTL (time-to-live) in seconds
    ///
    /// The value will be automatically deleted after the specified number of seconds.
    /// This is useful for temporary data, sessions, cache entries, etc.
    ///
    /// # Example
    /// ```python
    /// # Store a session that expires in 1 hour
    /// await db.put_with_ttl("sessions", "user_123", {"user": "alice"}, 3600)
    /// ```
    #[pyo3(signature = (namespace, key, value, ttl_seconds))]
    fn put_with_ttl<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        value: &'py PyAny,
        ttl_seconds: u64,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        let json_value = pyobject_to_json(value)?;

        future_into_py(py, async move {
            db.put_with_ttl(ns, k, json_value, ttl_seconds)
                .await
                .map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Store content with TTL and automatic distinction-based embedding
    ///
    /// Combines semantic storage with automatic expiration.
    #[pyo3(signature = (namespace, key, content, ttl_seconds, metadata = None))]
    fn put_similar_with_ttl<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        content: &'py PyAny,
        ttl_seconds: u64,
        metadata: Option<PyObject>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        let json_content = pyobject_to_json(content)?;
        let meta = metadata.and_then(|m| {
            Python::with_gil(|py| {
                pyobject_to_json(m.as_ref(py)).ok()
            })
        });

        future_into_py(py, async move {
            // First store with semantic embedding
            db.put_similar(ns.clone(), k.clone(), json_content.clone(), meta.clone())
                .await
                .map_err(to_python_error)?;
            
            // Then set TTL by re-putting with TTL (overwrites with same content but adds TTL)
            db.put_with_ttl(ns, k, json_content, ttl_seconds)
                .await
                .map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Clean up all expired TTL values
    ///
    /// Returns the number of items that were removed.
    fn cleanup_expired<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let db = self.db.clone();

        future_into_py(py, async move {
            let count = db.cleanup_expired().await.map_err(to_python_error)?;
            Ok(count)
        })
    }

    /// Get remaining TTL for a key in seconds
    ///
    /// Returns None if the key doesn't exist or has no TTL.
    fn get_ttl_remaining<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();

        future_into_py(py, async move {
            let ttl = db.get_ttl_remaining(&ns, &k).await.map_err(to_python_error)?;
            Python::with_gil(|py| Ok(ttl.to_object(py)))
        })
    }

    /// List keys expiring soon (within the given seconds)
    ///
    /// Returns a list of (namespace, key, seconds_remaining) tuples.
    #[pyo3(signature = (within_seconds))]
    fn list_expiring_soon<'py>(
        &self,
        py: Python<'py>,
        within_seconds: u64,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();

        future_into_py(py, async move {
            let expiring = db.list_expiring_soon(within_seconds).await.map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for (ns, key, remaining) in expiring {
                    let tuple = PyTuple::new(py, &[ns.to_object(py), key.to_object(py), remaining.to_object(py)]);
                    list.append(tuple).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Check if two distinctions are causally connected
    ///
    /// Returns True if there is a causal path between the two distinctions.
    fn are_connected<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key_a: &str,
        key_b: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let ka = key_a.to_string();
        let kb = key_b.to_string();

        future_into_py(py, async move {
            let connected = db.are_connected(&ns, &ka, &kb).await.map_err(to_python_error)?;
            Python::with_gil(|py| Ok(connected.to_object(py)))
        })
    }

    /// Get the causal connection path between two distinctions
    ///
    /// Returns a list of distinction IDs representing the path, or None if not connected.
    fn get_connection_path<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key_a: &str,
        key_b: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let ka = key_a.to_string();
        let kb = key_b.to_string();

        future_into_py(py, async move {
            let path = db.get_connection_path(&ns, &ka, &kb).await.map_err(to_python_error)?;
            Python::with_gil(|py| Ok(path.to_object(py)))
        })
    }

    /// Get the most highly-connected distinctions
    ///
    /// Returns a list of dictionaries with distinction info and connection scores.
    #[pyo3(signature = (namespace = None, k = 10))]
    fn get_highly_connected<'py>(
        &self,
        py: Python<'py>,
        namespace: Option<&str>,
        k: usize,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.map(|s| s.to_string());

        future_into_py(py, async move {
            let results = db.get_highly_connected(ns.as_deref(), k).await.map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for dist in results {
                    let dict = PyDict::new(py);
                    dict.set_item("namespace", &dist.namespace).ok();
                    dict.set_item("key", &dist.key).ok();
                    dict.set_item("connection_score", dist.connection_score).ok();
                    dict.set_item("parents", dist.parents).ok();
                    dict.set_item("children", dist.children).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Find similar distinctions that are not causally connected
    ///
    /// These pairs are candidates for synthesis. Returns a list of dictionaries
    /// with pair info and similarity scores.
    #[pyo3(signature = (namespace = None, k = 10, threshold = 0.7))]
    fn find_similar_unconnected_pairs<'py>(
        &self,
        py: Python<'py>,
        namespace: Option<&str>,
        k: usize,
        threshold: f32,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.map(|s| s.to_string());

        future_into_py(py, async move {
            let pairs = db.find_similar_unconnected_pairs(ns.as_deref(), k, threshold)
                .await
                .map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for pair in pairs {
                    let dict = PyDict::new(py);
                    dict.set_item("namespace_a", &pair.namespace_a).ok();
                    dict.set_item("key_a", &pair.key_a).ok();
                    dict.set_item("namespace_b", &pair.namespace_b).ok();
                    dict.set_item("key_b", &pair.key_b).ok();
                    dict.set_item("similarity_score", pair.similarity_score).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }

    /// Generate random walk combinations for dream-phase creative synthesis
    ///
    /// Performs random walks through the causal graph to discover novel combinations.
    /// Returns a list of dictionaries with start/end info and novelty scores.
    #[pyo3(signature = (n = 5, steps = 10))]
    fn random_walk_combinations<'py>(
        &self,
        py: Python<'py>,
        n: usize,
        steps: usize,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();

        future_into_py(py, async move {
            let combinations = db.random_walk_combinations(n, steps)
                .await
                .map_err(to_python_error)?;
            
            Python::with_gil(|py| {
                let list = PyList::new(py, Vec::<PyObject>::new());
                for combo in combinations {
                    let dict = PyDict::new(py);
                    dict.set_item("start_namespace", &combo.start_namespace).ok();
                    dict.set_item("start_key", &combo.start_key).ok();
                    dict.set_item("end_namespace", &combo.end_namespace).ok();
                    dict.set_item("end_key", &combo.end_key).ok();
                    dict.set_item("path", combo.path).ok();
                    dict.set_item("novelty_score", combo.novelty_score).ok();
                    list.append(dict).ok();
                }
                Ok(list.to_object(py))
            })
        })
    }
}

/// Identity management for Python
#[pyclass(name = "IdentityManager")]
pub struct PyIdentityManager {
    db: Arc<KoruDelta>,
}

#[pymethods]
impl PyIdentityManager {
    /// Create a new identity
    #[pyo3(signature = (display_name = None, bio = None))]
    fn create<'py>(
        &self,
        py: Python<'py>,
        display_name: Option<String>,
        bio: Option<String>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let dn = display_name;
        let b = bio;

        future_into_py(py, async move {
            let auth = db.auth();
            let user_data = koru_delta::auth::IdentityUserData {
                display_name: dn,
                bio: b,
                avatar_hash: None,
                metadata: std::collections::HashMap::new(),
            };
            
            let (identity, secret_key) = auth.create_identity(user_data)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("id", &identity.public_key).ok();
                dict.set_item("secret_key", secret_key).ok();
                dict.set_item("created_at", identity.created_at.to_rfc3339()).ok();
                Ok(dict.to_object(py))
            })
        })
    }

    /// Verify an identity exists and is valid
    fn verify<'py>(
        &self,
        py: Python<'py>,
        identity_id: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let id = identity_id.to_string();

        future_into_py(py, async move {
            let auth = db.auth();
            let valid = auth.verify_identity(&id).await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Python::with_gil(|py| Ok(valid.to_object(py)))
        })
    }

    /// Get identity info
    fn get<'py>(
        &self,
        py: Python<'py>,
        identity_id: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let id = identity_id.to_string();

        future_into_py(py, async move {
            let auth = db.auth();
            match auth.get_identity(&id)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))? {
                Some(identity) => {
                    Python::with_gil(|py| {
                        let dict = PyDict::new(py);
                        dict.set_item("id", &identity.public_key).ok();
                        dict.set_item("created_at", identity.created_at.to_rfc3339()).ok();
                        dict.set_item("difficulty", identity.difficulty).ok();
                        Ok(Some(dict.to_object(py)))
                    })
                }
                None => Ok(None),
            }
        })
    }
}

/// Workspace handle for Python
#[pyclass(name = "Workspace")]
pub struct PyWorkspace {
    db: Arc<KoruDelta>,
    name: String,
}

#[pymethods]
impl PyWorkspace {
    /// Store a value in the workspace
    fn put<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        value: &'py PyAny,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = self.name.clone();
        let k = key.to_string();
        let json_value = pyobject_to_json(value)?;

        future_into_py(py, async move {
            db.put(ns, k, json_value)
                .await
                .map_err(to_python_error)?;
            Ok(())
        })
    }

    /// Retrieve a value from the workspace
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = self.name.clone();
        let k = key.to_string();

        future_into_py(py, async move {
            let versioned = db.get(ns, k).await.map_err(to_python_error)?;
            Python::with_gil(|py| Ok(json_to_pyobject(py, versioned.value())))
        })
    }

    /// List all keys in the workspace
    fn list_keys<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = self.name.clone();

        future_into_py(py, async move {
            let keys = db.list_keys(&ns).await;
            Python::with_gil(|py| Ok(keys.to_object(py)))
        })
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("<Workspace '{}'>", self.name)
    }
}


