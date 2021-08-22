fn main() {
  data::clone_prismarine_data();

  data::generate_blocks();
  data::generate_items();
  data::generate_entities();
}
