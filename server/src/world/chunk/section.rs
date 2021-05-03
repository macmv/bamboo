/// A chunk section.
pub enum Section {
  // This is faster than using traits.
  Palletted(PalettedSection),
  Direct(DirectSection),
}

pub struct PalettedSection {}

pub struct DirectSection {}
