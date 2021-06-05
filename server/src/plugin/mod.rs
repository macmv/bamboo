mod plugin;

pub use plugin::Plugin;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

pub struct PluginManager {
  gil: GILGuard,
}

#[pyfunction]
fn get_world(v: bool) -> i32 {
  if v {
    5
  } else {
    20
  }
}

impl PluginManager {
  pub fn new() -> Self {
    let m = PluginManager { gil: Python::acquire_gil() };
    m.init().unwrap();
    m
  }
  fn init(&self) -> Result<(), PyErr> {
    let py = self.gil.python();
    let code = PyModule::from_code(
      py,
      r#"
print("Hello world!")
def gaming():
  print("big")
  print(sugarcane.get_world(False))
  "#,
      "main.py",
      "main",
    )?;

    let sugarcane = PyModule::new(py, "sugarcane")?;
    sugarcane.add_function(wrap_pyfunction!(get_world, sugarcane)?)?;

    code.add_submodule(sugarcane)?;

    info!("done reading code");

    code.call0("gaming")?;

    Ok(())
  }
}
