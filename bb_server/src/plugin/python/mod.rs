use pyo3::prelude::*;

pub struct PyCallback {
  callback: PyObject,
}
