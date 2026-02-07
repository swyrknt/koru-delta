//! Utility functions for Python bindings

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

/// Helper to create rich error messages
pub fn format_key_not_found(namespace: &str, key: &str, similar: Vec<String>) -> String {
    let mut msg = format!("Key not found: {}:{}", namespace, key);
    
    if !similar.is_empty() {
        msg.push_str("\n\nDid you mean:\n");
        for s in similar.iter().take(3) {
            msg.push_str(&format!("  - {}\n", s));
        }
    }
    
    msg
}

/// Helper to validate namespace/key format
pub fn validate_key(key: &str) -> PyResult<()> {
    if key.is_empty() {
        return Err(PyValueError::new_err("Key cannot be empty"));
    }
    
    if key.contains('/') {
        return Err(PyValueError::new_err(
            "Key cannot contain '/'. Use separate namespace argument."
        ));
    }
    
    Ok(())
}

/// Helper to validate namespace format  
pub fn validate_namespace(namespace: &str) -> PyResult<()> {
    if namespace.is_empty() {
        return Err(PyValueError::new_err("Namespace cannot be empty"));
    }
    
    Ok(())
}
