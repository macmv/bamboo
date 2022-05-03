use crate::player::Player;
use bb_common::{math::Pos, util::Chat};
use std::sync::Arc;

/// Anyone who can send commands. This could be the server console, a player, a
/// command block, a panda plugin, etc.
pub trait CommandSender {
  /// If this command sender has a position in the world, it should be returned.
  /// If it does not have a position, None should be returned, and relative
  /// coordinates will not be avalible to this sender.
  fn block_pos(&self) -> Option<Pos>;

  /// If this is a player, returns the player.
  fn as_player(&self) -> Option<&Arc<Player>> { None }

  /// Sends a message to this command sender. Used for invalid commands.
  fn send_message(&mut self, msg: Chat);
}
