//! KoruDelta Python Bindings - LCA Architecture v3.0.0
//!
//! This module provides Python access to the full LCA (Local Causal Agent) architecture,
//! including unified storage, distinction-based embeddings, queries, views, and identity.

#![allow(non_local_definitions)]
use pyo3::prelude::*;
use pyo3::create_exception;

mod database;
mod types;

use database::{PyDatabase, PyIdentityManager, PyWorkspace};

/// Convert Rust DeltaError to appropriate Python exception
fn to_python_error(e: koru_delta::DeltaError) -> PyErr {
    match e {
        koru_delta::DeltaError::KeyNotFound { .. } => KeyNotFoundError::new_err(e.to_string()),
        koru_delta::DeltaError::NoValueAtTimestamp { .. } => KeyNotFoundError::new_err(e.to_string()),
        koru_delta::DeltaError::InvalidData { .. } => InvalidDataError::new_err(e.to_string()),
        koru_delta::DeltaError::EngineError(_) => EngineError::new_err(e.to_string()),
        koru_delta::DeltaError::StorageError(_) => StorageError::new_err(e.to_string()),
        koru_delta::DeltaError::TimeError(_) => TimeError::new_err(e.to_string()),
        koru_delta::DeltaError::SerializationError(_) => SerializationError::new_err(e.to_string()),
    }
}

// Exception hierarchy

// Base exception for all KoruDelta errors
create_exception!(koru_delta, KoruDeltaError, pyo3::exceptions::PyException);

// Storage exceptions

// Raised when a key is not found in the database
create_exception!(koru_delta, KeyNotFoundError, KoruDeltaError);

// Raised when data is invalid
create_exception!(koru_delta, InvalidDataError, KoruDeltaError);

// Raised when a storage operation fails
create_exception!(koru_delta, StorageError, KoruDeltaError);

// Raised for serialization errors
create_exception!(koru_delta, SerializationError, KoruDeltaError);

// Engine exceptions

// Raised when engine operation fails
create_exception!(koru_delta, EngineError, KoruDeltaError);

// Raised for time-related errors
create_exception!(koru_delta, TimeError, KoruDeltaError);

/// Module initialization
#[pymodule]
fn _internal(_py: Python, m: &PyModule) -> PyResult<()> {
    // Core classes
    m.add_class::<PyDatabase>()?;
    m.add_class::<PyIdentityManager>()?;
    m.add_class::<PyWorkspace>()?;
    
    // Exceptions
    m.add("KoruDeltaError", _py.get_type::<KoruDeltaError>())?;
    m.add("KeyNotFoundError", _py.get_type::<KeyNotFoundError>())?;
    m.add("InvalidDataError", _py.get_type::<InvalidDataError>())?;
    m.add("StorageError", _py.get_type::<StorageError>())?;
    m.add("SerializationError", _py.get_type::<SerializationError>())?;
    m.add("EngineError", _py.get_type::<EngineError>())?;
    m.add("TimeError", _py.get_type::<TimeError>())?;
    
    // Version
    m.add("__version__", "3.0.0")?;
    
    Ok(())
}
