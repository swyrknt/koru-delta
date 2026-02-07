# Python Bindings Implementation Design

**Architecture:** PyO3 + maturin + pyo3-asyncio
**Goal:** Native Python feel with Rust performance

---

## Project Structure

```
bindings/python/
├── Cargo.toml              # PyO3 dependencies
├── pyproject.toml          # Python packaging (maturin)
├── src/
│   ├── lib.rs             # Module entry point
│   ├── database.rs        # Database wrapper
│   ├── agent_memory.rs    # AgentMemory wrapper
│   ├── vector.rs          # Vector/embedding support
│   ├── rag.rs             # RAG pipeline
│   └── types.rs           # Type conversions
├── koru_delta/
│   ├── __init__.py        # Public API
│   ├── _internal.pyi      # Type stubs for Rust module
│   └── utils.py           # Pure Python helpers
├── examples/
│   ├── 01_quickstart.py
│   ├── 02_agent_memory.py
│   ├── 03_rag_pipeline.py
│   └── 04_edge_deployment.py
└── tests/
    ├── test_database.py
    ├── test_agent_memory.py
    └── test_vector.py
```

---

## Core Design Decisions

### 1. Async Runtime Integration

**Challenge:** Rust uses Tokio, Python uses asyncio
**Solution:** `pyo3-asyncio` bridges them

```rust
// src/database.rs
use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;

#[pyclass]
struct PyDatabase {
    db: Arc<KoruDelta>,
    rt: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyDatabase {
    fn get<'py>(&self, py: Python<'py>, namespace: &str, key: &str) -> PyResult<&'py PyAny> {
        let db = self.db.clone();
        let ns = namespace.to_string();
        let k = key.to_string();
        
        future_into_py(py, async move {
            match db.get(&ns, &k).await {
                Ok(value) => Ok(Python::with_gil(|py| {
                    value_to_pyobject(py, value.value())
                })),
                Err(e) => Err(KoruDeltaError::new_err(e.to_string())),
            }
        })
    }
}
```

**Key insight:** Every async method returns `PyResult<&PyAny>` which is a Python awaitable

### 2. Memory Management

**Challenge:** Rust ownership vs Python GC
**Solution:** Reference counting + explicit cleanup

```rust
// Database keeps Arc to KoruDelta
// Python object drop = decrement Arc count
// When count = 0, Rust drops database

#[pyclass]
struct PyDatabase {
    db: Arc<KoruDelta>,
}

// Python context manager
impl PyDatabase {
    fn __enter__(&self) -> PyResult<Self> { Ok(self.clone()) }
    fn __exit__(&mut self, ...) -> PyResult<bool> { 
        // Explicit cleanup if needed
        Ok(false) 
    }
}
```

### 3. Type Conversion

**Challenge:** Rust types ↔ Python types
**Solution:** Centralized conversion layer

```rust
// src/types.rs
use pyo3::prelude::*;
use serde_json::Value;

/// Convert serde_json::Value to Python object
fn json_to_pyobject(py: Python, value: &Value) -> PyObject {
    match value {
        Value::Null => py.None(),
        Value::Bool(b) => b.to_object(py),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_object(py)
            } else if let Some(f) = n.as_f64() {
                f.to_object(py)
            } else {
                py.None()
            }
        }
        Value::String(s) => s.to_object(py),
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_pyobject(py, item)).unwrap();
            }
            list.to_object(py)
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_pyobject(py, v)).unwrap();
            }
            dict.to_object(py)
        }
    }
}

/// Convert Python object to serde_json::Value
fn pyobject_to_json(obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into())))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let arr: Result<Vec<_>, _> = list.iter().map(|item| pyobject_to_json(item)).collect();
        Ok(Value::Array(arr?))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            let value = pyobject_to_json(v)?;
            map.insert(key, value);
        }
        Ok(Value::Object(map))
    } else {
        Err(PyTypeError::new_err("Unsupported type"))
    }
}
```

### 4. NumPy Integration

**Challenge:** Zero-copy vector passing
**Solution:** `numpy` crate + buffer protocol

```rust
// src/vector.rs
use numpy::{PyArray1, PyReadonlyArray1};

#[pymethods]
impl PyDatabase {
    fn embed<'py>(
        &self,
        py: Python<'py>,
        namespace: &str,
        key: &str,
        embedding: &PyAny,  // Accept list or numpy array
        metadata: Option<PyObject>,
    ) -> PyResult<&'py PyAny> {
        // Convert to Vec<f32> (handles both list and ndarray)
        let vector = if let Ok(arr) = embedding.downcast::<PyArray1<f32>>() {
            arr.readonly().as_slice()?.to_vec()
        } else if let Ok(list) = embedding.extract::<Vec<f32>>() {
            list
        } else {
            return Err(PyTypeError::new_err(
                "embedding must be list or numpy array of f32"
            ));
        };
        
        // ... rest of implementation
    }
}
```

### 5. Error Handling

**Challenge:** Rust errors → Python exceptions
**Solution:** Custom exception hierarchy

```rust
// src/lib.rs
use pyo3::create_exception;

// Base exception
create_exception!(koru_delta, KoruDeltaError, pyo3::exceptions::PyException);

// Specific exceptions
create_exception!(koru_delta, KeyNotFoundError, KoruDeltaError);
create_exception!(koru_delta, SerializationError, KoruDeltaError);
create_exception!(koru_delta, ValidationError, KoruDeltaError);
create_exception!(koru_delta, DatabaseClosedError, KoruDeltaError);

// Convert Rust errors
fn to_python_error(e: DeltaError) -> PyErr {
    match e {
        DeltaError::KeyNotFound { .. } => {
            KeyNotFoundError::new_err(e.to_string())
        }
        DeltaError::SerializationError(_) => {
            SerializationError::new_err(e.to_string())
        }
        _ => KoruDeltaError::new_err(e.to_string()),
    }
}
```

### 6. Context Managers

**Challenge:** Pythonic resource management
**Solution:** `__enter__`/`__exit__` + async context managers

```rust
// src/database.rs
#[pymethods]
impl PyDatabase {
    fn __enter__<'py>(slf: PyRef<'py, Self>) -> PyResult<PyRef<'py, Self>> {
        Ok(slf)
    }
    
    fn __exit__<'py>(
        &mut self,
        _py: Python<'py>,
        _exc_type: Option<&'py PyAny>,
        _exc_value: Option<&'py PyAny>,
        _traceback: Option<&'py PyAny>,
    ) -> PyResult<bool> {
        // Database cleanup happens automatically via Drop
        Ok(false)
    }
}

// For async context managers, we use Python wrapper
```

```python
# koru_delta/__init__.py
class Database:
    """Async context manager for database."""
    
    def __init__(self, config=None):
        self._config = config
        self._db = None
    
    async def __aenter__(self):
        self._db = await _internal.Database.create(
            self._config._to_rust() if self._config else None
        )
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        # Cleanup happens automatically
        pass
    
    async def get(self, namespace: str, key: str):
        return await self._db.get(namespace, key)
    
    # ... other methods
```

---

## Python Layer Design

### Pure Python Wrapper Benefits:
1. **Better error messages** - Add context in Python
2. **Type hints** - Full IDE autocomplete
3. **Docstrings** - Python help() works
4. **Convenience methods** - Combine multiple Rust calls
5. **Validation** - Check arguments before calling Rust

### Structure:

```python
# koru_delta/__init__.py
"""
KoruDelta - The database for AI agents.

Example:
    >>> async with Database() as db:
    ...     await db.put("users", "alice", {"name": "Alice"})
    ...     user = await db.get("users", "alice")
"""

from ._internal import (
    Database as _Database,
    KoruDeltaError,
    KeyNotFoundError,
    # ... other exports
)
from .config import Config
from .agent_memory import AgentMemory
from .rag import RAG, Document

__version__ = "2.0.0"

class Database:
    """
    KoruDelta database instance.
    
    Use as async context manager:
        async with Database() as db:
            ...
    """
    
    def __init__(self, config: Config | None = None):
        self._config = config or Config()
        self._db: _Database | None = None
    
    async def __aenter__(self) -> "Database":
        self._db = await _Database.create(self._config._to_rust())
        return self
    
    async def __aexit__(self, *args) -> None:
        self._db = None
    
    async def put(self, namespace: str, key: str, value: Any) -> None:
        """Store a value."""
        if self._db is None:
            raise DatabaseClosedError("Database not initialized")
        await self._db.put(namespace, key, value)
    
    async def get(self, namespace: str, key: str) -> Any:
        """Retrieve a value."""
        if self._db is None:
            raise DatabaseClosedError("Database not initialized")
        try:
            return await self._db.get(namespace, key)
        except KeyNotFoundError as e:
            # Add helpful suggestion
            similar = await self._suggest_similar_keys(namespace, key)
            if similar:
                e.add_note(f"Did you mean: {', '.join(similar)}?")
            raise
    
    # ... other methods
```

---

## Build & Distribution

### maturin Configuration

```toml
# pyproject.toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "koru-delta"
version = "2.0.0"
description = "The database for AI agents"
readme = "README.md"
license = {text = "MIT OR Apache-2.0"}
requires-python = ">=3.9"
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.9",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Rust",
    "Topic :: Database",
    "Topic :: Scientific/Engineering :: Artificial Intelligence",
]
dependencies = [
    "typing-extensions>=4.0; python_version<'3.10'",
    "numpy>=1.20",
]

[project.optional-dependencies]
openai = ["openai>=1.0"]
rag = ["openai>=1.0", "tiktoken>=0.5"]
dev = ["pytest", "pytest-asyncio", "mypy", "black", "ruff"]

[tool.maturin]
python-source = "koru_delta"
module-name = "koru_delta._internal"
features = ["pyo3/extension-module"]
```

### Cargo Configuration

```toml
# Cargo.toml (in bindings/python/)
[package]
name = "koru-delta-python"
version = "2.0.0"
edition = "2021"

[lib]
name = "koru_delta"
crate-type = ["cdylib"]

[dependencies]
# KoruDelta core (path dependency)
koru-delta = { path = "../.." }

# PyO3
pyo3 = { version = "0.22", features = ["extension-module", "abi3-py39"] }
pyo3-asyncio = { version = "0.22", features = ["tokio-runtime"] }

# NumPy integration (optional but recommended)
numpy = "0.22"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async runtime
tokio = { version = "1.0", features = ["rt-multi-thread"] }
```

---

## Testing Strategy

### Rust Tests (in bindings/python/)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::Python;

    #[test]
    fn test_json_roundtrip() {
        Python::with_gil(|py| {
            let json = json!({"name": "Alice", "age": 30});
            let py_obj = json_to_pyobject(py, &json);
            let back = pyobject_to_json(py_obj.as_ref(py)).unwrap();
            assert_eq!(json, back);
        });
    }
}
```

### Python Tests
```python
# tests/test_database.py
import pytest
import asyncio
from koru_delta import Database, KeyNotFoundError

@pytest.fixture
async def db():
    async with Database() as database:
        yield database

@pytest.mark.asyncio
async def test_put_get(db):
    await db.put("users", "alice", {"name": "Alice"})
    result = await db.get("users", "alice")
    assert result["name"] == "Alice"

@pytest.mark.asyncio
async def test_key_not_found(db):
    with pytest.raises(KeyNotFoundError) as exc_info:
        await db.get("users", "nonexistent")
    
    assert "nonexistent" in str(exc_info.value)
```

---

## Performance Considerations

### 1. Minimize Python ↔ Rust Crossing
- Batch operations when possible
- Keep hot data in Rust
- Use callbacks for iteration, not lists

### 2. Zero-Copy for Vectors
- Use buffer protocol for NumPy
- Avoid copying large embeddings

### 3. Async Efficiency
- Single Tokio runtime shared across all DB instances
- Don't create new runtime per database

### 4. Memory Pooling
- Reuse buffers for serialization
- Avoid allocations in hot paths

---

## Development Workflow

```bash
# Setup
cd bindings/python
python -m venv .venv
source .venv/bin/activate
pip install maturin

# Build & test loop
maturin develop  # Build and install in venv
pytest tests/    # Run Python tests

# Release build
maturin build --release

# Publish to PyPI
maturin publish
```

---

## Success Metrics

1. **Install time:** `pip install koru-delta` < 30 seconds
2. **Import time:** `import koru_delta` < 100ms
3. **First query:** < 10ms (after import)
4. **Memory overhead:** < 20MB for basic usage
5. **Test coverage:** > 90%
6. **Type coverage:** 100% (full mypy strict)
