extern crate data;

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  tonic_build::compile_protos("proto/connection.proto")?;

  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
  tonic_build::configure()
    .file_descriptor_set_path(out_dir.join("connection.bin"))
    .compile(&["proto/connection.proto"], &["proto"])
    .unwrap();

  data::generate_protocol();

  Ok(())
}
