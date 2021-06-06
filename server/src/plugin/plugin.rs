use rutie::{Exception, Module, Object, RString};

pub struct Plugin {
  name: String,
  m:    Module,
}

impl Plugin {
  pub fn new(name: String, m: Module) -> Self {
    Plugin { name, m }
  }

  pub fn call(&self, name: &str) {
    if self.m.respond_to(name) {
      if let Err(e) = self.m.protect_send(name, &[]) {
        error!("Error while calling {} on plugin {}: {}", name, self.name, e.inspect());
        for l in e.backtrace().unwrap() {
          error!("{}", l.try_convert_to::<RString>().unwrap().to_str());
        }
      }
    }
  }
}
