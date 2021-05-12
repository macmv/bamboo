extern crate gens;

fn main() {
  println!("cargo:rerun-if-changed=gens");
  gens::generate_protocols();
}
