fn main() {
  println!("cargo:rerun-if-changed=../data/src");

  sc_data::clone_prismarine_data();

  sc_data::generate_blocks();
  sc_data::generate_items();
  sc_data::generate_entities();
}
