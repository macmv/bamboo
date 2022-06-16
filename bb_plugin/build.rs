use bb_data::Target::Plugin;

fn main() {
  let c = bb_data::Collector::new();
  c.generate_blocks(bb_data::BlockOpts { versions: false, data: false, kinds: true });
  c.generate_items();
  c.generate_entities();
  c.generate_particles(Plugin);
}
