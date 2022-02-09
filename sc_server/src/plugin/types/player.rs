use super::{add_from, chat::SlChat, world::SlWorld, wrap};
use crate::player::Player;
use sc_common::util::Chat;
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
  pub fn send_message(&self, msg: &Var) {
    let out = match msg {
      Var::Builtin(_, data) => {
        let chat = data.as_any().downcast_ref::<SlChat>();
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
      .map_err(|e| RuntimeError::custom(format!("invalid ip '{ip}': {e}"), Span::default()))?
      .collect();
    self.inner.switch_to(ips);
    Ok(())
  }
}
