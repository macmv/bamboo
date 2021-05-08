use super::Kind;

use common::version::BlockVersion;

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
pub struct Type {
  kind: Kind,
}

impl Type {
  /// Creates a new block type. This should only be used when constructing the
  /// block tables. If you need to get a pre-existing block type, use
  /// [`WorldManager::get_block`].
  pub(crate) fn new(kind: Kind) -> Self {
    Type { kind }
  }
  pub fn kind(&self) -> &Kind {
    &self.kind
  }
  /// Gets the block id for the given version. This is simply a lookup in a
  /// hashtable. It will panic if the version is invalid.
  pub fn id(&self, _v: BlockVersion) -> u32 {
    0
  }
}
