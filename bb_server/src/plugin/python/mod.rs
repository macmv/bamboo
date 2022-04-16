use super::{types::Callback, PluginImpl, ServerMessage};
use crossbeam_channel::{Receiver, Sender};
use panda::runtime::RuntimeError;
use pyo3::{exceptions, prelude::*};
use std::{env, fs, path::Path, thread};

pub struct PyCallback {
  callback: PyObject,
}

pub fn conv_err(err: RuntimeError) -> PyErr { exceptions::PyValueError::new_err(err.to_string()) }

impl Callback for PyObject {
  fn box_clone(&self) -> Box<dyn Callback> { Box::new(self.clone()) }
}

pub struct Plugin {
  tx: Sender<ServerMessage>,
}

struct PyFuncs {
  init: Py<PyAny>,
}

impl Plugin {
  pub fn new(name: String) -> Self {
    let (tx, rx) = crossbeam_channel::bounded(1024);
    thread::spawn(move || {
      pyo3::prepare_freethreaded_python();
      let code = fs::read_to_string(Path::new("plugins/python-test/main.py")).unwrap();
      let funcs = match Python::with_gil::<_, PyResult<PyFuncs>>(|py| {
        let module = PyModule::from_code(py, &code, "main.py", "main")?;
        let funcs = PyFuncs { init: module.getattr("init")?.into_py(py) };
        Ok(funcs)
      }) {
        Ok(f) => f,
        Err(e) => {
          error!("python plugin encountered error: {e}");
          return;
        }
      };
      while let Ok(e) = rx.recv() {
        match match e {
          ServerMessage::Event { .. } => Python::with_gil::<_, PyResult<()>>(|py| {
            funcs.init.call0(py)?;
            Ok(())
          }),
          _ => Ok(()),
        } {
          Ok(_) => {}
          Err(e) => {
            error!("python plugin encountered error: {e}");
          }
        }
      }
    });
    Plugin { tx }
  }
}

impl PluginImpl for Plugin {
  fn call(&self, event: ServerMessage) -> Result<bool, ()> {
    self.tx.send(event);
    Ok(true)
  }
}
