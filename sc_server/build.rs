fn main() {
  println!("cargo:rerun-if-changed=../sc_data/src");

  sc_data::generate_blocks();
  sc_data::generate_items();
  sc_data::generate_entities();
}
