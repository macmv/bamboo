use super::Type;

/// A block kind. This is the more general 'block' that you might think of. For
/// example, there is one block kind for each type of stairs. However, within
/// each stair, there is a different [`Type`] for each rotation of that stair.
/// If you need to quickly convert between the two, you can use
/// [`Kind::default_type()`] and [`Type::kind()`].
pub struct Kind {
  types: Vec<Type>,
}

impl Kind {
  pub fn air() -> Kind {
    Kind { types: Vec::new() }
  }
  pub fn default_type(&self) -> &Type {
    &self.types[0]
  }
}
