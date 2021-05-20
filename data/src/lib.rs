extern crate serde;
extern crate serde_derive;
extern crate serde_json;

pub mod block;
pub mod item;
pub mod protocol;

use std::{env, path::Path};

pub fn generate_blocks() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);

  block::generate(&dir).unwrap();
}

pub fn generate_items() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);

  item::generate(&dir).unwrap();
}

/// This should be run in build.rs. It reads all protocols from minecraft-data,
/// and then stores that all in one json file. This file should then included
/// with `include_str!`. The path is `$OUT_DIR/protcol/versions.rs`
pub fn store_protocols() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);

  protocol::store(&dir).unwrap();
}
