use pyo3::prelude::*;

pub struct PyCallback {
  callback: PyObject,
}

pub fn conv_err(err: RuntimeError) -> PyErr { PyErr::new(err.to_string()) }
