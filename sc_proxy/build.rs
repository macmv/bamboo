fn main() {
  println!("cargo:rerun-if-changed=../sc_data/src");

  sc_data::generate_protocol();
  sc_data::generate_blocks();
}
