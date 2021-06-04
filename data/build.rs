extern crate data_gen;

fn main() {
  println!("cargo:rerun-if-changed=data");
  data_gen::generate_server();
}
