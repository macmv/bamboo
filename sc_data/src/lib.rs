use std::{env, fmt, path::PathBuf};

mod block;
mod dl;
mod entity;
pub mod gen;
mod item;
mod protocol;

#[derive(Debug, Clone, Copy)]
pub struct Version {
  maj: u32,
  min: u32,
}

impl Version {
  pub const fn new(maj: u32, min: u32) -> Version {
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
  item::generate(&out_dir()).unwrap();
}

pub fn generate_entities() {
  entity::generate(&out_dir()).unwrap();
}

pub fn generate_protocol() {
  protocol::generate(&out_dir()).unwrap();
}

pub static VERSIONS: &'static [Version] = &[
  Version::new(8, 9),
  Version::new(9, 4),
  Version::new(10, 2),
  Version::new(11, 2),
  Version::new(12, 2),
  Version::new(14, 4),
  Version::new(15, 2),
  Version::new(16, 5),
  Version::new(17, 1),
];

impl Version {
  pub fn to_protocol(&self) -> String {
    format!("ProtocolVersion::V1_{}_{}", self.maj, self.min)
  }
}
