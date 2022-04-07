use super::{
  add_from,
  chat::PdChat,
  item::{PdInventory, PdStack},
  util::PdFPos,
  world::PdWorld,
  wrap,
};
use crate::player::{Player, Team};
use bb_common::util::{chat::Color, Chat};
use panda::{
  define_ty,
  parse::token::Span,
  runtime::{Result, RuntimeError, Var},
};
use parking_lot::Mutex;
use std::{
  net::{SocketAddr, ToSocketAddrs},
  str::FromStr,
  sync::{Arc, Weak},
};

// TODO: The `Weak` system is far better than using an `Arc`, but will still
// create race conditions if the player leaves while an event is being handled.
// This needs to be handled correctly, but I don't know how.

#[derive(Clone, Debug)]
pub struct PdPlayer {
  pub(super) inner: Weak<Player>,
  pub username:     String,
}
impl From<Arc<Player>> for PdPlayer {
  fn from(p: Arc<Player>) -> Self {
    PdPlayer { username: p.username().clone(), inner: Arc::downgrade(&p) }
  }
}
wrap!(Arc<Mutex<Team>>, PdTeam);

impl PdPlayer {
  pub fn inner(&self) -> Result<Arc<Player>> {
    self.inner.upgrade().ok_or_else(|| {
      RuntimeError::custom(format!("`{}` is offline", self.username), Span::call_site())
    })
  }
}

/// A Player. This struct is for online players. There is currently no way to
/// lookup an offline player.
///
/// Most of the functions on `Player` will return an error if the player has
/// disconnected. This means that any plugins that wish to keep track of online
/// players need to implement `on_player_leave`, and remove the player from
/// their internal list at that time.
#[define_ty(path = "bamboo::player::Player")]
impl PdPlayer {
  /// Returns the username of the player. This will never change, as long as the
  /// user stays online.
  pub fn username(&self) -> Result<String> { Ok(self.inner()?.username().into()) }

  /// Teleports the player to the given position, with a yaw and pitch.
  pub fn teleport(&self, pos: &PdFPos, yaw: f32, pitch: f32) -> Result<()> {
    self.inner()?.teleport(pos.inner, yaw, pitch);
    Ok(())
  }

  /// Sends the given chat message to a player. This accepts exactly one
  /// argument, which can be any type. If it is a `PdChat`, then it will be
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
  pub fn send_message(&self, msg: Var) -> Result<()> {
    let out = match &msg {
      Var::Builtin(_, data) => {
        let borrow = data.borrow();
        let chat = borrow.as_any().downcast_ref::<PdChat>();
        if let Some(chat) = chat {
          chat.inner.lock().unwrap().clone()
        } else {
          Chat::new(msg.to_string())
        }
      }
      _ => Chat::new(msg.to_string()),
    };
    self.inner()?.send_message(out);
    Ok(())
  }

  /// Sets the title for this player. To show the title and subtitle, call
  /// [`show_title`].
  pub fn set_title(&self, title: &PdChat) -> Result<()> {
    self.inner()?.set_title(title.inner.lock().unwrap().clone());
    Ok(())
  }
  /// Sets the subtitle for this player. To show the title and subtitle, call
  /// [`show_title`].
  pub fn set_subtitle(&self, subtitle: &PdChat) -> Result<()> {
    self.inner()?.set_subtitle(subtitle.inner.lock().unwrap().clone());
    Ok(())
  }
  /// Shows the current title to the player. The `fade_in`, `stay`, and
  /// `fade_out` arguments are all in ticks.
  pub fn show_title(&self, fade_in: u32, stay: u32, fade_out: u32) -> Result<()> {
    self.inner()?.show_title(fade_in, stay, fade_out);
    Ok(())
  }

  /// Returns the world this player is in. This can be used to get/set
  /// blocks, access other players, and modify entities.
  pub fn world(&self) -> Result<PdWorld> { Ok(self.inner()?.world().clone().into()) }

  /// Switches the player to a new server. If the server is found, the player
  /// will be disconnected after this call. If the server is not found, an error
  /// will be returned.
  pub fn switch_to(&self, ip: &str) -> Result<()> {
    // TODO: Span::call_site()
    let ips: Vec<SocketAddr> = ip
      .to_socket_addrs()
      .map_err(|e| RuntimeError::custom(format!("invalid ip '{ip}': {e}"), Span::call_site()))?
      .collect();
    self.inner()?.switch_to(ips);
    Ok(())
  }

  /// Shows an inventory to the player.
  pub fn show_inventory(&self, inv: &PdInventory, title: &PdChat) -> Result<()> {
    self.inner()?.show_inventory(inv.inner.clone(), &title.inner.lock().unwrap());
    Ok(())
  }

  /// Shows a scoreboard to the player. Call `set_scoreboard_line` to display
  /// anything in the scoreboard.
  pub fn show_scoreboard(&self) -> Result<()> {
    self.inner()?.lock_scoreboard().show();
    Ok(())
  }
  /// Hides the scoreboard for the player.
  pub fn hide_scoreboard(&self) -> Result<()> {
    self.inner()?.lock_scoreboard().hide();
    Ok(())
  }
  /// Sets a line in the scoreboard. If it is hidden, this will still work, and
  /// the updated lines will show when the scoreboard is shown again.
  pub fn set_scoreboard_line(&self, line: u8, message: &PdChat) -> Result<()> {
    self.inner()?.lock_scoreboard().set_line(line, &message.inner.lock().unwrap());
    Ok(())
  }
  /// Clears a line in the scoreboard. If it is hidden, this will still work,
  /// and the updated lines will show when the scoreboard is shown again.
  pub fn clear_scoreboard_line(&self, line: u8) -> Result<()> {
    self.inner()?.lock_scoreboard().clear_line(line);
    Ok(())
  }

  /// Sets the player's tab list name.
  ///
  /// Note that this does not update the name above the player's head. The only
  /// way to do that is by adding this player to a team.
  ///
  /// This will produce inconsistent behavior if the player is on a team. Only
  /// use if needed. Using teams is going to be more reliable.
  pub fn set_tab_name(&self, name: &PdChat) -> Result<()> {
    self.inner()?.set_tab_name(Some(name.inner.lock().unwrap().clone()));
    Ok(())
  }
  /// Removes the player's tab list name.
  pub fn clear_tab_name(&self) -> Result<()> {
    self.inner()?.set_tab_name(None);
    Ok(())
  }

  /// Gives the player the passed item.
  pub fn give(&self, stack: &PdStack) -> Result<()> {
    self.inner()?.lock_inventory().give(&stack.inner);
    Ok(())
  }
}

/// A team. This is a group of players and entities, which all share a set
/// of properties. These properties include setting a username color, disabling
/// friendly fire, showing invisible teammates, and more.
///
/// This can be created through `Bamboo::create_team`.
#[define_ty(path = "bamboo::player::Team")]
impl PdTeam {
  pub fn set_color(&self, name: &str) -> Result<()> {
    self.inner.lock().set_color(
      Color::from_str(name)
        .map_err(|err| RuntimeError::custom(err.to_string(), Span::call_site()))?,
    );
    Ok(())
  }

  /// Adds the player to this team.
  pub fn add_player(&self, player: &PdPlayer) -> Result<()> {
    self.inner.lock().add(player.inner()?.as_ref());
    Ok(())
  }
}
