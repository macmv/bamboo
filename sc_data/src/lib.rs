use std::{env, fmt, path::PathBuf};

mod block;
mod dl;
mod entity;
pub mod gen;
mod item;
mod protocol;

#[derive(Debug, Clone, Copy)]
pub struct Version {
  maj:      u32,
  min:      u32,
  protocol: u32,
}

impl Version {
  pub const fn new(maj: u32, min: u32, protocol: u32) -> Version {
    Version { maj, min, protocol }
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
  Version::new(8, 9, 47),
  Version::new(9, 4, 110),
  Version::new(10, 2, 210),
  Version::new(11, 2, 316),
  Version::new(12, 2, 340),
  Version::new(14, 4, 498),
  Version::new(15, 2, 578),
  Version::new(16, 5, 754),
  Version::new(17, 1, 756),
];

impl Version {
  pub fn to_protocol(&self) -> String {
    format!("ProtocolVersion::V1_{}_{}", self.maj, self.min)
  }
  pub fn to_index(&self) -> usize {
    if self.maj <= 12 {
      self.maj as usize - 8
    } else {
      // We are missing 1.13
      self.maj as usize - 9
    }
  }
}
