extern crate gens;

use std::{env, error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
  println!("cargo:rerun-if-changed=gens");
  gens::generate();
  Ok(())
}
