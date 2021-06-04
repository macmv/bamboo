extern crate data;

fn main() {
  println!("cargo:rerun-if-changed=../data/src");
  data::generate_protocol();
}
