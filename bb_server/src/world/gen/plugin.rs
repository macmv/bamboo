use super::BiomeGen;

pub struct PBiomeGen {
  id: usize,
}

impl BiomeGen for PBiomeGen {
  fn new(id: usize) -> Self { PBiomeGen { id } }
  fn id(&self) -> usize { self.id }
}
