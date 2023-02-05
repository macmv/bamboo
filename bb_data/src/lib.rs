use dl::Downloader;
use std::{fmt, path::PathBuf};

mod block;
mod command;
mod dl;
mod enchantment;
mod entity;
pub mod gen;
mod item;
mod particle;
mod protocol;
mod tag;

pub use block::BlockOpts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
  maj:      u32,
  min:      u32,
  protocol: u32,
}

impl Version {
  pub const fn new(maj: u32, min: u32, protocol: u32) -> Version { Version { maj, min, protocol } }
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

#[derive(Debug, Clone, Copy)]
pub enum Target {
  Host,
  Plugin,
}

pub struct Collector {
  dl:  Downloader,
  out: PathBuf,
}

impl Collector {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { Collector::new_at("../data-config.toml", "../data-config-example.toml") }

  pub fn new_at(data_path: &str, data_example_path: &str) -> Self {
    #[cfg(not(test))]
    let out = PathBuf::new().join(&std::env::var("OUT_DIR").expect("could not get out dir"));
    #[cfg(test)]
    let out = PathBuf::new();
    Collector { dl: Downloader::new(data_path, data_example_path), out }
  }

  pub fn generate_blocks(&self, opts: BlockOpts) { block::generate(self, opts).unwrap(); }
  pub fn generate_commands(&self, target: Target) { command::generate(self, target).unwrap(); }
  pub fn generate_items(&self) { item::generate(self).unwrap(); }
  pub fn generate_entities(&self) { entity::generate(self).unwrap(); }
  pub fn generate_protocol(&self) { protocol::generate(self).unwrap(); }
  pub fn generate_particles(&self, target: Target) { particle::generate(self, target).unwrap(); }
  pub fn generate_enchantments(&self) { enchantment::generate(self).unwrap(); }
  pub fn generate_tags(&self) { tag::generate(self).unwrap(); }
}

pub static VERSIONS: &[Version] = &[
  Version::new(8, 9, 47),
  Version::new(9, 4, 110),
  Version::new(10, 2, 210),
  Version::new(11, 2, 316),
  Version::new(12, 2, 340),
  Version::new(14, 4, 498),
  Version::new(15, 2, 578),
  Version::new(16, 5, 754),
  Version::new(17, 1, 756),
  Version::new(18, 2, 758),
  Version::new(19, 3, 761),
];

impl Version {
  pub fn to_protocol(&self) -> String {
    if self.min == 0 {
      format!("ProtocolVersion::V1_{}", self.maj)
    } else {
      format!("ProtocolVersion::V1_{}_{}", self.maj, self.min)
    }
  }
  pub fn to_block(&self) -> String { format!("BlockVersion::V1_{}", self.maj) }
  pub fn to_index(&self) -> usize {
    if self.maj <= 12 {
      self.maj as usize - 8
    } else {
      // We are missing 1.13
      self.maj as usize - 9
    }
  }
}
