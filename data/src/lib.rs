use proc_macro::TokenStream;
use std::{env, path::Path};

mod block;
mod entity;
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

#[proc_macro]
pub fn generate_blocks(input: TokenStream) -> TokenStream {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);
  prismarine::clone(&dir).unwrap();
  block::generate(&dir).unwrap().into()
}

#[proc_macro]
pub fn generate_items(input: TokenStream) -> TokenStream {
  "fn answer() -> u32 { 42 }".parse().unwrap()
}

#[proc_macro]
pub fn generate_cb_protocol(input: TokenStream) -> TokenStream {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);
  prismarine::clone(&dir).unwrap();
  protocol::generate(&dir, true).unwrap().into()
}

#[proc_macro]
pub fn generate_sb_protocol(input: TokenStream) -> TokenStream {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);
  prismarine::clone(&dir).unwrap();
  protocol::generate(&dir, false).unwrap().into()
}
