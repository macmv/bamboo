use super::BiomeGen;

pub struct SlBiomeGen {
  id: usize,
}

impl BiomeGen for SlBiomeGen {
  fn new(id: usize) -> Self { SlBiomeGen { id } }
  fn id(&self) -> usize { self.id }
}
