use super::BiomeGen;

pub struct PdBiomeGen {
  id: usize,
}

impl BiomeGen for PdBiomeGen {
  fn new(id: usize) -> Self { PdBiomeGen { id } }
  fn id(&self) -> usize { self.id }
}
