extern crate serde;
extern crate serde_derive;
extern crate serde_json;

pub mod block;
pub mod protocol;

use std::{env, path::Path};

pub fn generate() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);

  block::generate(&dir).unwrap();
}

pub fn generate_protocols() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);

  protocol::generate(&dir).unwrap();
}
