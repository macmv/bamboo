use crate::version::BlockVersion;

pub mod cb;
pub mod sb;
mod ser;

pub trait VersionConverter {
  /// The `id` argument is a block id in the latest version. This function
  /// should return the equivalent block id for the given version. It should
  /// also work when passed the latest version (it should return the same id).
  fn block_to_old(&self, id: u32, ver: BlockVersion) -> u32;
  /// The `id` argument is a block id in the given version. The returned block
  /// id should be the equivalent id in the latest version this server supports.
  /// This should also support passing in the latest version (it should return
  /// the same id).
  fn block_to_new(&self, id: u32, ver: BlockVersion) -> u32;

  /// Converts an item id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  fn item_to_old(&self, id: u32, ver: BlockVersion) -> u32 { 0 }
  /// Converts an item id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  fn item_to_new(&self, id: u32, ver: BlockVersion) -> u32 { 0 }

  /// Converts an entity id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  fn entity_to_old(&self, id: u32, ver: BlockVersion) -> u32 { 0 }
  /// Converts an entity id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  fn entity_to_new(&self, id: u32, ver: BlockVersion) -> u32 { 0 }
}
