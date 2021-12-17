use super::WorldGen;

mod desert;
mod forest;
mod mountain;
mod plains;

impl WorldGen {
  pub fn add_default_biomes(&mut self) {
    self.add_biome::<desert::Gen>();
    self.add_biome::<forest::Gen>();
    self.add_biome::<plains::Gen>();
    self.add_biome::<mountain::Gen>();
  }
}
