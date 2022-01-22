use super::WorldGen;
use std::{error::Error, fmt};

mod desert;
mod forest;
mod mountain;
mod plains;

#[derive(Debug, Clone)]
pub struct InvalidBiome(String);

impl fmt::Display for InvalidBiome {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "invalid biome: {}", self.0) }
}

impl Error for InvalidBiome {}

impl WorldGen {
  pub fn add_named_biome(&mut self, name: &str) -> Result<(), InvalidBiome> {
    match name {
      "desert" => self.add_biome::<desert::Gen>(),
      "forest" => self.add_biome::<forest::Gen>(),
      "plains" => self.add_biome::<plains::Gen>(),
      "mountains" => self.add_biome::<mountain::Gen>(),
      _ => return Err(InvalidBiome(name.into())),
    }
    Ok(())
  }
}
