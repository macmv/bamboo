use bb_data::Target::Host;

fn main() {
  println!("cargo:rerun-if-changed=../bb_data/src");

  let c = bb_data::Collector::new();
  c.generate_protocol();
  c.generate_blocks(bb_data::BlockOpts { versions: true, data: false, kinds: false });
  c.generate_items();
  c.generate_entities();
  c.generate_particles(Host);
}
