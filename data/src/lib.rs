use std::{
  env,
  path::{Path, PathBuf},
};

mod block;
mod entity;
mod gen;
mod item;
mod prismarine;
mod protocol;
mod util;

// /// Generates block, item, and entity data. Should only be called from the
// /// data crate.
// pub fn generate_server() {
//   let out = env::var_os("OUT_DIR").unwrap();
//   let dir = Path::new(&out);
//   prismarine::clone(&dir).unwrap();
//
//   let kinds = block::generate(&dir).unwrap();
//   item::generate(&dir, kinds).unwrap();
//   entity::generate(&dir).unwrap();
// }
//
// /// This should be run in build.rs. It reads all protocols from
// minecraft-data, /// and then stores that all in one json file. This file
// should then included /// with `include_str!`. The path is
// `$OUT_DIR/protcol/versions.rs` pub fn generate_protocol() {
//   let out = env::var_os("OUT_DIR").unwrap();
//   let dir = Path::new(&out);
//   prismarine::clone(&dir).unwrap();
//
//   protocol::store(&dir).unwrap();
// }

fn out_dir() -> PathBuf {
  PathBuf::new().join(&env::var("OUT_DIR").expect("could not get out dir"))
}

pub fn clone_prismarine_data() {
  prismarine::clone(&out_dir()).unwrap();
}

pub fn generate_blocks() {
  block::generate(&out_dir()).unwrap();
}

pub fn generate_items() {
  item::generate(&out_dir(), block::generate_kinds(&out_dir()).unwrap()).unwrap();
}

pub fn generate_entities() {
  entity::generate(&out_dir()).unwrap();
}

pub fn generate_protocol() {
  prismarine::clone(&out_dir()).unwrap();
  protocol::generate(&out_dir()).unwrap();
}
