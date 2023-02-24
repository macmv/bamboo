use super::{
  panda::PandaPlugin,
  types::{Callback, Callback as BCallback},
  Bamboo, CallError, GlobalEvent, PlayerEvent, PlayerRequest, PluginImpl, PluginManager,
  PluginReply,
};
use crate::world::WorldManager;
use crossbeam_channel::{Receiver, Sender};
use panda::runtime::RuntimeError;
use pyo3::{exceptions, intern, prelude::*, types::PyString};
use std::{env, fs, path::PathBuf, sync::Arc, thread};

pub struct PyCallback {
  callback: PyObject,
}

pub fn conv_err(err: RuntimeError) -> PyErr { exceptions::PyValueError::new_err(err.to_string()) }

impl Callback for PyObject {
  fn box_clone(&self) -> Box<dyn Callback> { Box::new(self.clone()) }
}

pub struct Plugin {
  tx: Sender<Event>,
}

struct Event {
  ev:    GlobalEvent,
  reply: bool,
}

use parking_lot::Mutex;

static BAMBOO: Mutex<Option<Bamboo>> = Mutex::new(None);

#[pyfunction]
fn instance() -> Bamboo { BAMBOO.lock().as_ref().unwrap().clone() }

#[pymodule]
fn bamboo(py: Python<'_>, m: &PyModule) -> PyResult<()> {
  m.add_class::<Bamboo>()?;
  m.add_function(wrap_pyfunction!(instance, m)?)?;
  Ok(())
}

impl Plugin {
  pub fn new(idx: usize, name: String, path: PathBuf, wm: Arc<WorldManager>) -> Self {
    let (tx, rx) = crossbeam_channel::bounded::<Event>(1024);
    thread::spawn(move || {
      // TODO: Handle multiple plugins going brrrrr
      *BAMBOO.lock() = Some(Bamboo::new(idx, wm));
      pyo3::append_to_inittab!(bamboo);
      pyo3::prepare_freethreaded_python();
      let code = fs::read_to_string(path).unwrap();
      match Python::with_gil::<_, PyResult<_>>(|py| {
        let module = PyModule::from_code(py, &code, "main.py", "main")?;
        while let Ok(e) = rx.recv() {
          let name = format!("on_{}", e.ev.name());
          let s = PyString::intern(py, &name);
          if !module.hasattr(s)? {
            continue;
          }
          let func = module.getattr(s)?;
          e.ev.with_python(py, |arg| func.call1(arg))?;
        }
        Ok(())
      }) {
        Ok(f) => f,
        Err(e) => {
          error!("python plugin encountered error: {e}");
          return;
        }
      };
    });
    Plugin { tx }
  }
}

impl PluginImpl for Plugin {
  fn call_global(&self, ev: GlobalEvent) -> Result<(), CallError> {
    self.tx.send(Event { ev, reply: false }).map_err(CallError::no_keep)?;
    Ok(())
  }
  fn call(&self, ev: PlayerEvent) -> Result<(), CallError> {
    // self.tx.send(()).map_err(CallError::no_keep)?;
    Ok(())
  }
  fn req(&self, req: PlayerRequest) -> Result<PluginReply, CallError> {
    // Ok(PluginReply::Cancel { allow: self.req(req.name(),
    // vec![req.into_panda()]) })
    Ok(PluginReply::Cancel { allow: true })
  }
  fn panda(&mut self) -> Option<&mut PandaPlugin> { None }
}
