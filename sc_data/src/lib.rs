use std::{env, fmt, path::PathBuf};

mod block;
mod dl;
pub mod gen;
mod protocol;

pub struct Version {
  maj: u32,
  min: u32,
}

impl Version {
  pub fn new(maj: u32, min: u32) -> Version {
    Version { maj, min }
  }
}

impl fmt::Display for Version {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.min == 0 {
      write!(f, "1.{}", self.maj)
    } else {
      write!(f, "1.{}.{}", self.maj, self.min)
    }
  }
}

fn out_dir() -> PathBuf {
  PathBuf::new().join(&env::var("OUT_DIR").expect("could not get out dir"))
}

pub fn generate_blocks() {
  block::generate(&out_dir()).unwrap();
}

pub fn generate_items() {
  /*
  item::generate(&out_dir(), block::generate_kinds(&out_dir()).unwrap()).unwrap();
  */
}

pub fn generate_entities() {
  /*
  entity::generate(&out_dir()).unwrap();
  */
}

pub fn generate_protocol() {
  protocol::generate(&out_dir()).unwrap();
}
