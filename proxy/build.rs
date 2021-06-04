extern crate data_gen;

fn main() {
  println!("cargo:rerun-if-changed=../data-gen/src");
  data_gen::generate_protocol();
}
