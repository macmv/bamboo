use bb_data::Target::Host;

fn main() {
  bb_data::generate_blocks();
  bb_data::generate_items();
  bb_data::generate_entities();
  bb_data::generate_particles(Host);
}
