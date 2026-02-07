//! Type conversion between Rust and Python

use pyo3::prelude::*;
use pyo3::exceptions::PyTypeError;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

/// Convert serde_json::Value to Python object
pub fn json_to_pyobject(py: Python, value: &Value) -> PyObject {
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
            let list = PyList::new(py, arr.iter().map(|v| json_to_pyobject(py, v)));
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
pub fn pyobject_to_json(obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(Value::Number(
            serde_json::Number::from_f64(f).unwrap_or(0.into()),
        ))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let arr: Result<Vec<_>, _> = list.iter().map(pyobject_to_json).collect();
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
        Err(PyTypeError::new_err(format!(
            "Unsupported type: cannot convert {} to JSON",
            obj.get_type().name()?
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_roundtrip() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let json = json!({"name": "Alice", "age": 30, "active": true});
            let py_obj = json_to_pyobject(py, &json);
            let back = pyobject_to_json(py_obj.as_ref(py)).unwrap();
            assert_eq!(json, back);
        });
    }
}
