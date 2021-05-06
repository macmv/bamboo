use super::Kind;

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
pub struct Type {
  kind: Kind,
}

impl Type {
  pub fn from_id(id: u32) -> Self {
    Type::air()
  }
  pub fn air() -> Type {
    Type { kind: Kind::air() }
  }
  pub fn kind(&self) -> &Kind {
    &self.kind
  }
  pub fn id(&self) -> u32 {
    0
  }
}
