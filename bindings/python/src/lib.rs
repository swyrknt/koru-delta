//! KoruDelta Python Bindings
//!
//! This crate provides Python bindings for the KoruDelta database,
//! designed for AI agents and edge deployment.

// Suppress non-local impl warnings from PyO3 macros (known issue with newer Rust)
#![allow(non_local_definitions)]

use pyo3::prelude::*;

mod database;
mod types;
#[allow(dead_code)]
mod utils;
#[allow(dead_code)]
mod vector;

use database::PyDatabase;

/// Convert DeltaError to Python exception
fn to_python_error(e: koru_delta::DeltaError) -> PyErr {
    use koru_delta::DeltaError;
    
    match e {
        DeltaError::KeyNotFound { namespace, key } => {
            KeyNotFoundError::new_err(format!(
                "Key not found: {}:{}",
                namespace, key
            ))
        }
        DeltaError::SerializationError(_) => {
            SerializationError::new_err(e.to_string())
        }
        DeltaError::InvalidData { .. } => {
            ValidationError::new_err(e.to_string())
        }
        _ => KoruDeltaError::new_err(e.to_string()),
    }
}

// Define Python exceptions
pyo3::create_exception!(koru_delta, KoruDeltaError, pyo3::exceptions::PyException);
pyo3::create_exception!(koru_delta, KeyNotFoundError, KoruDeltaError);
pyo3::create_exception!(koru_delta, SerializationError, KoruDeltaError);
pyo3::create_exception!(koru_delta, ValidationError, KoruDeltaError);
pyo3::create_exception!(koru_delta, DatabaseClosedError, KoruDeltaError);

/// Initialize the module
#[pymodule]
fn _internal(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register classes
    m.add_class::<PyDatabase>()?;
    
    // Register exceptions
    m.add("KoruDeltaError", _py.get_type::<KoruDeltaError>())?;
    m.add("KeyNotFoundError", _py.get_type::<KeyNotFoundError>())?;
    m.add("SerializationError", _py.get_type::<SerializationError>())?;
    m.add("ValidationError", _py.get_type::<ValidationError>())?;
    m.add("DatabaseClosedError", _py.get_type::<DatabaseClosedError>())?;
    
    Ok(())
}
