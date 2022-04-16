use super::{types::Callback, PluginImpl, ServerMessage};
use panda::runtime::RuntimeError;
use pyo3::{exceptions, prelude::*};

pub struct PyCallback {
  callback: PyObject,
}

pub fn conv_err(err: RuntimeError) -> PyErr { exceptions::PyValueError::new_err(err.to_string()) }

impl Callback for PyObject {
  fn box_clone(&self) -> Box<dyn Callback> { Box::new(self.clone()) }
}

pub struct Plugin {}

impl Plugin {
  pub fn new(name: String) -> Self { Plugin {} }
}

impl PluginImpl for Plugin {
  fn call(&self, event: ServerMessage) -> Result<bool, ()> { Ok(true) }
}
