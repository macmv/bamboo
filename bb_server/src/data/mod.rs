use std::path::Path;

mod crafting;

pub use crafting::CraftingData;

pub struct Data {
  crafting: CraftingData,
}

impl Data {
  pub fn load(path: &str) -> Self {
    Data { crafting: CraftingData::load(&Path::new(path).join("minecraft/recipes")) }
  }
}
