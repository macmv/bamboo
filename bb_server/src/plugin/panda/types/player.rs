use super::{
  add_from,
  chat::SlChat,
  item::{SlInventory, SlStack},
  world::SlWorld,
  wrap,
};
use crate::player::Player;
use bb_common::util::Chat;
use std::{
  net::{SocketAddr, ToSocketAddrs},
  sync::Arc,
};
use sugarlang::{
  define_ty,
  parse::token::Span,
  runtime::{RuntimeError, Var},
};

wrap!(Arc<Player>, SlPlayer);

/// A Player. This struct is for online players. If anyone has disconnected,
/// this struct will still exist, but the functions will return outdated
/// information. There is currently no way to lookup an offline player.
#[define_ty(path = "sugarcane::player::Player")]
impl SlPlayer {
  /// Returns the username of the player. This will never change, as long as the
  /// user stays online.
  pub fn username(&self) -> String { self.inner.username().into() }

  /// Sends the given chat message to a player. This accepts exactly one
  /// argument, which can be any type. If it is a `SlChat`, then it will be
  /// formatted correctly. Anything else will show up with debug formatting.
  ///
  /// # Example
  ///
  /// ```
  /// // The text `Hello!` will show up the the user's chat box.
  /// p.send_message("Hello!")
  ///
  /// chat = Chat::new()
  /// chat.add("I").color("red")
  /// chat.add(" am").color("gold")
  /// chat.add(" colors!").color("yellow")
  /// // The text `I am colors!` will show up in the user's chat box, colored
  /// // in red, then gold, then yellow.
  /// p.send_message(chat)
  /// ```
  pub fn send_message(&self, msg: Var) {
    let out = match &msg {
      Var::Builtin(_, data) => {
        let borrow = data.borrow();
        let chat = borrow.as_any().downcast_ref::<SlChat>();
        if let Some(chat) = chat {
          chat.inner.lock().unwrap().clone()
        } else {
          Chat::new(msg.to_string())
        }
      }
      _ => Chat::new(msg.to_string()),
    };
    self.inner.send_message(&out);
  }

  /// Returns the world this player is in. This can be used to get/set
  /// blocks, access other players, and modify entities.
  pub fn world(&self) -> SlWorld { self.inner.world().clone().into() }

  /// Switches the player to a new server. If the server is found, the player
  /// will be disconnected after this call. If the server is not found, an error
  /// will be returned.
  pub fn switch_to(&self, ip: &str) -> Result<(), RuntimeError> {
    // TODO: Span::call_site()
    let ips: Vec<SocketAddr> = ip
      .to_socket_addrs()
      .map_err(|e| RuntimeError::custom(format!("invalid ip '{ip}': {e}"), Span::call_site()))?
      .collect();
    self.inner.switch_to(ips);
    Ok(())
  }

  /// Shows an inventory to the player.
  pub fn show_inventory(&self, inv: &SlInventory, title: &SlChat) {
    self.inner.show_inventory(inv.inner.clone(), &title.inner.lock().unwrap())
  }

  /// Shows a scoreboard to the player. Call `set_scoreboard_line` to display
  /// anything in the scoreboard.
  pub fn show_scoreboard(&self) { self.inner.lock_scoreboard().show(); }
  /// Hides the scoreboard for the player.
  pub fn hide_scoreboard(&self) { self.inner.lock_scoreboard().hide(); }
  /// Sets a line in the scoreboard. If it is hidden, this will still work, and
  /// the updated lines will show when the scoreboard is shown again.
  pub fn set_scoreboard_line(&self, line: u8, message: &SlChat) {
    self.inner.lock_scoreboard().set_line(line, &message.inner.lock().unwrap());
  }

  /// Sets a display name for this player. This will be shown instead of their
  /// username in the tab list and above their head.
  pub fn set_display_name(&self, name: &SlChat) {
    self.inner.set_display_name(Some(name.inner.lock().unwrap().clone()));
  }
  /// Removes a display name on this player (if any is present). This does
  /// nothing if there is no display name on this player.
  pub fn clear_display_name(&self) { self.inner.set_display_name(None); }

  /// Gives the player the passed item.
  pub fn give(&self, stack: &SlStack) { self.inner.lock_inventory().give(stack.inner.clone()); }
}
