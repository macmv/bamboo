extern crate serde;
extern crate serde_derive;
extern crate serde_json;

pub mod block;

use std::{env, path::Path};

pub fn generate() {
  let out = env::var_os("OUT_DIR").unwrap();
  let dir = Path::new(&out);

  block::generate(&dir).unwrap();
}
