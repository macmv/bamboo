use crate::player::Player;
use bb_common::{math::Pos, util::Chat};
use std::sync::Arc;

pub enum ErrorFormat {
  /// Used for minecraft clients, where the text is not monospaced. The span in
  /// the command will have underline text formatting.
  Minecraft,
  /// Used for terminal clients, where the text is monospaced. This adds a line
  /// under the command, with a row of `^` for the underlined section.
  Monospace,
}

/// Anyone who can send commands. This could be the server console, a player, a
/// command block, a panda plugin, etc.
pub trait CommandSender {
  /// If this command sender has a position in the world, it should be returned.
  /// If it does not have a position, None should be returned, and relative
  /// coordinates will not be available to this sender.
  fn block_pos(&self) -> Option<Pos>;

  /// If this is a player, returns the player.
  fn as_player(&self) -> Option<&Arc<Player>> { None }

  /// Sends a message to this command sender. Used for invalid commands.
  fn send_message(&mut self, msg: Chat);

  /// Returns the format that the sender wants to receive errors in. This is so
  /// that rcon clients and players can receive errors in formats that work
  /// better for their clients.
  fn error_format(&self) -> ErrorFormat;
}
