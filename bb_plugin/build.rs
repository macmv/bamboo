use bb_data::Target::Plugin;

fn main() {
  bb_data::generate_blocks(bb_data::BlockOpts { versions: false, data: false, kinds: true });
  bb_data::generate_items();
  bb_data::generate_entities();
  bb_data::generate_particles(Plugin);
}
