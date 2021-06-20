/// Any data specific to an entity.
#[derive(Debug)]
pub struct Data {
  display_name: &'static str,
  width:        f32,
  height:       f32,
}

impl Data {
  pub fn display_name(&self) -> &str {
    &self.display_name
  }
}

/// Generates a table from all items to any metadata that type has. This
/// includes things like the display name, stack size, etc.
pub fn generate_items() -> Vec<Data> {
  let mut entities = vec![];
  include!(concat!(env!("OUT_DIR"), "/entity/data.rs"));
  entities
}
