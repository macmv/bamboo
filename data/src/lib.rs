mod block;
mod entity;
mod item;
mod prismarine;
pub mod protocol;
mod util;

use std::{env, path::Path};

/// Generates block, item, and entity data. Should only be called from the
/// data crate.
pub fn generate_server() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);
  prismarine::clone(&dir).unwrap();

  let kinds = block::generate(&dir).unwrap();
  item::generate(&dir, kinds).unwrap();
  entity::generate(&dir).unwrap();
}

/// This should be run in build.rs. It reads all protocols from minecraft-data,
/// and then stores that all in one json file. This file should then included
/// with `include_str!`. The path is `$OUT_DIR/protcol/versions.rs`
pub fn generate_protocol() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);
  prismarine::clone(&dir).unwrap();

  protocol::store(&dir).unwrap();
}
