extern crate data_gen;

fn main() {
  println!("cargo:rerun-if-changed=data");
  println!("cargo:rerun-if-changed=data-gen");
  data_gen::generate_server();
}
