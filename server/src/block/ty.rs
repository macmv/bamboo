use std::sync::Arc;

use common::version::BlockVersion;

use super::Kind;

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
#[derive(Debug)]
pub struct Type {
  kind:  Arc<Kind>,
  state: u32,
}

impl Type {
  /// Returns the block kind that this state comes from.
  pub fn kind(&self) -> &Kind {
    &self.kind
  }
  /// Gets the block id for the given version. This is simply a lookup in a
  /// hashtable. It will panic if the version is invalid.
  pub fn id(&self, _v: BlockVersion) -> u32 {
    self.state
  }
}
