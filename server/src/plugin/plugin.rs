use super::wrapper::SugarcaneRb;
use common::math::Pos;
use rutie::{AnyObject, Exception, Module, Object, RString};

/// A wrapper struct for a Ruby plugin. This is used to execute Ruby code
/// whenever an event happens.
pub struct Plugin {
  name: String,
  m:    Module,
}

impl Plugin {
  /// Creates a new plugin. The name should be the name of the module (for
  /// debugging) and the Module should be the ruby module for this plugin.
  pub fn new(name: String, m: Module) -> Self {
    Plugin { name, m }
  }

  /// Calls init on the plugin. This is called right after all plugins are
  /// loaded. The world will have been initialized, and it is possible for
  /// clients to be joining when this function is called.
  pub fn init(&self, sc: SugarcaneRb) {
    self.call("init", &[sc.try_convert_to().unwrap()]);
  }

  /// Calls on_block_place on the plugin. This can be called whenever, but will
  /// always be called after init.
  pub fn on_block_place(&self, pos: Pos) {
    self.call("init", &[pos.try_convert_to().unwrap()]);
  }

  /// Calls the given function with the given args. This will verify that the
  /// function exists, and will handle errors in the log.
  fn call(&self, name: &str, args: &[AnyObject]) {
    if self.m.respond_to(name) {
      if let Err(e) = self.m.protect_send(name, args) {
        error!("Error while calling {} on plugin {}: {}", name, self.name, e.inspect());
        for l in e.backtrace().unwrap() {
          error!("{}", l.try_convert_to::<RString>().unwrap().to_str());
        }
      }
    }
  }
}
