use bb_common::math::Pos;

/// Anyone who can send commands. This could be the server console, a player, a
/// command block, a sugarlang plugin, etc.
pub trait CommandSender {
  /// If this command sender has a position in the world, it should be returned.
  /// If it does not have a position, None should be returned, and relative
  /// coordinates will not be avalible to this sender.
  fn block_pos(&self) -> Option<Pos>;
}
