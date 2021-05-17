use super::Kind;

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
#[derive(Debug)]
pub struct Type {
  pub(super) kind:  Kind,
  pub(super) state: u32,
}

impl Type {
  /// Returns the block kind that this state comes from.
  pub fn kind(&self) -> &Kind {
    &self.kind
  }
  /// Gets the block id for the given version. This will always be the latest
  /// blockstate id.
  pub fn id(&self) -> u32 {
    self.state
  }
}
