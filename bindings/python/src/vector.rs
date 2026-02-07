//! Vector/embedding utilities for Python

use pyo3::prelude::*;
use pyo3::exceptions::{PyTypeError, PyValueError};

/// Convert Python list or numpy array to Vec<f32>
pub fn extract_f32_vector(obj: &PyAny) -> PyResult<Vec<f32>> {
    // Try list first
    if let Ok(list) = obj.extract::<Vec<f32>>() {
        return Ok(list);
    }
    
    // Try numpy array
    if let Ok(array) = obj.downcast::<numpy::PyArray1<f32>>() {
        let readonly = array.readonly();
        let slice = readonly.as_slice()
            .map_err(|e| PyValueError::new_err(format!("Cannot read array: {}", e)))?;
        return Ok(slice.to_vec());
    }
    
    // Try converting from other numeric types
    if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
        let mut result = Vec::with_capacity(list.len());
        for item in list.iter() {
            let val: f32 = item.extract()
                .map_err(|_| PyTypeError::new_err(
                    "Embedding must contain numeric values"
                ))?;
            result.push(val);
        }
        return Ok(result);
    }
    
    Err(PyTypeError::new_err(
        "Embedding must be a list or numpy array of floats"
    ))
}

/// Get dimensionality of a vector
pub fn get_vector_dim(obj: &PyAny) -> PyResult<usize> {
    if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
        Ok(list.len())
    } else if let Ok(array) = obj.downcast::<numpy::PyArray1<f32>>() {
        Ok(array.len())
    } else if let Ok(vec) = obj.extract::<Vec<f32>>() {
        Ok(vec.len())
    } else {
        Err(PyTypeError::new_err("Cannot determine vector dimensions"))
    }
}
