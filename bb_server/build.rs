use bb_data::Target::Host;

fn main() {
  println!("cargo:rerun-if-changed=../bb_data/src");

  bb_data::generate_blocks(bb_data::BlockOpts { versions: true, data: true, kinds: true });
  bb_data::generate_items();
  bb_data::generate_entities();
  bb_data::generate_particles(Host);
  bb_data::generate_tags();
}
