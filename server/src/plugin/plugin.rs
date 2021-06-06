use rutie::{AnyObject, Exception, Module, Object, RString};

pub struct Plugin {
  name: String,
  m:    Module,
}

impl Plugin {
  pub fn new(name: String, m: Module) -> Self {
    Plugin { name, m }
  }

  pub fn init(&self) {
    self.call("init", &[]);
  }

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
