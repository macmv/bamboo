use super::WorldGen;

mod desert;
mod forest;
mod mountain;
mod plains;

impl WorldGen {
  pub fn add_named_biome(&mut self, name: &str) -> Result<(), ()> {
    match name {
      "desert" => self.add_biome::<desert::Gen>(),
      "forest" => self.add_biome::<forest::Gen>(),
      "plains" => self.add_biome::<plains::Gen>(),
      "mountains" => self.add_biome::<mountain::Gen>(),
      _ => return Err(()),
    }
    Ok(())
  }
}
