extern crate data;

fn main() {
  println!("cargo:rerun-if-changed=data");
  data::generate_protocol();
}
