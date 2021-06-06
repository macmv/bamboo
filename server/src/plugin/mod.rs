mod plugin;

pub use plugin::Plugin;

use rutie::{Module, Object, RString, VM};

pub struct PluginManager {}

module!(Sugarcane);

methods!(
  Sugarcane,
  rtself,
  fn say_hello() -> RString {
    RString::new_utf8("Big rust callback energy")
  },
);

impl PluginManager {
  pub fn new() -> Self {
    VM::init();

    Module::new("Sugarcane").define(|c| {
      c.def_self("hello", say_hello);
    });

    VM::eval("puts Sugarcane::hello").unwrap();

    VM::exit(0);

    PluginManager {}
  }
  // /// Creates the `sugarcane` ruby module. Used whenever plugins are
  // /// re-loaded.
  // fn create_module(&self) -> PyResult<&PyModule> {
  //   let sugarcane = PyModule::new(self.gil.python(), "sugarcane")?;
  //   sugarcane.add_function(wrap_pyfunction!(get_world, sugarcane)?)?;
  //   Ok(sugarcane)
  // }
  // fn init(&self) -> Result<(), PyErr> {
  //   let sugarcane = PluginManager::create_module(self.py)?;
  //   let mut plugins = self.plugins.lock().unwrap();
  //   plugins.clear();
  //
  //   for f in fs::read_dir("plugins").unwrap() {
  //     let f = f.unwrap();
  //     let m = fs::metadata(f.path()).unwrap();
  //     if m.is_file() {
  //       let source = fs::read_to_string(f.path()).unwrap();
  //       let fname = f.file_name();
  //       let fname = fname.to_str().unwrap();
  //       // The file name without the extension
  //       let name = &fname[..fname.len() - f.path().extension().unwrap().len()];
  //       dbg!(&fname, &name);
  //
  //       let plug = PyModule::from_code(self.gil.python(), &source, fname,
  // name)?;       plug.add_submodule(&sugarcane)?;
  //       plug.call0("init")?;
  //       plugins.push(Plugin::new(plug));
  //     }
  //   }
  //
  //   Ok(())
  // }
}
