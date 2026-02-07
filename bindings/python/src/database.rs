//! Python wrapper for KoruDelta database

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3_asyncio::tokio::future_into_py;

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
        value: &PyAny,
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

    /// Store a vector embedding
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
                dict.set_item("namespace_count", stats.namespace_count).ok();
                Ok(dict.to_object(py))
            })
        })
    }

    /// String representation
    fn __repr__(&self) -> String {
        "<Database instance>".to_string()
    }
}

use pyo3::types::PyList;
use pyo3::types::PyDict;
