extern crate data;

fn main() {
  println!("cargo:rerun-if-changed=data");
  data::store_protocols();
}
