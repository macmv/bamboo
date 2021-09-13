fn main() {
  println!("cargo:rerun-if-changed=../data/src");

  data::clone_prismarine_data();

  data::generate_blocks();
  data::generate_items();
  data::generate_entities();
}
