use bb_data::Target::Plugin;

#[cfg(feature = "host")]
use std::env;

#[cfg(feature = "host")]
fn main() {
  let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

  cbindgen::Builder::new()
    .with_crate(crate_dir)
    .generate()
    .expect("Unable to generate bindings")
    .write_to_file("bamboo.h");

  build_data();
}
#[cfg(not(feature = "host"))]
fn main() { build_data(); }

fn build_data() { bb_data::generate_particles(Plugin); }
