use super::{
  add_from,
  chat::PChat,
  item::{PInventory, PStack},
  util::{PFPos, PUUID},
  world::PWorld,
  wrap,
};
use crate::{
  item::Stack,
  player::{Player, Team},
};
use bb_common::util::{chat::Color, Chat, UUID};
use bb_plugin_macros::define_ty;
use panda::{
  parse::token::Span,
  runtime::{Result, RuntimeError, Var},
};
use parking_lot::Mutex;
use std::{
  net::{SocketAddr, ToSocketAddrs},
  str::FromStr,
  sync::{Arc, Weak},
};

#[derive(Clone, Debug)]
pub struct PPlayer {
  pub(super) inner: Weak<Player>,
  pub username:     String,
  pub uuid:         UUID,
}
impl From<Arc<Player>> for PPlayer {
  fn from(p: Arc<Player>) -> Self {
    PPlayer { username: p.username().clone(), uuid: p.id(), inner: Arc::downgrade(&p) }
  }
}
wrap!(Arc<Mutex<Team>>, PTeam);

impl PPlayer {
  pub fn inner(&self) -> Result<Arc<Player>> {
    self.inner.upgrade().ok_or_else(|| {
      RuntimeError::custom(format!("`{}` is offline", self.username), Span::call_site())
    })
  }
}

/// A Player. This struct is for online players. There is currently no way to
/// lookup an offline player.
///
/// Offline players are difficult to handle. This struct cannot be consutructed
/// for a player who is offline, but it can stay alive after a player has
/// disconnected. For this reason, some functions simply do nothing if a player
/// has logged off, while others will cause an error.
#[define_ty(panda_path = "bamboo::player::Player")]
impl PPlayer {
  /// Returns the username of the player. This will never change, as long as the
  /// user stays online.
  ///
  /// This will return an error if the player is offline. This is intentional,
  /// as their username can change after they log off. If you need a way to
  /// identify players, use `Player::uuid` instead.
  pub fn username(&self) -> Result<String> { Ok(self.inner()?.username().into()) }

  /// Returns the unique identifier of this player. This will never change, even
  /// if the player leaves.
  ///
  /// Note that this may change if the proxy is in offline mode instead of
  /// online mode.
  pub fn uuid(&self) -> PUUID { PUUID { inner: self.uuid } }

  /// Teleports the player to the given position, with a yaw and pitch.
  ///
  /// This will do nothing if the player is offline.
  pub fn teleport(&self, pos: &PFPos, yaw: f32, pitch: f32) {
    if let Ok(i) = self.inner() {
      i.teleport(pos.inner, yaw, pitch);
    }
  }

  /// Sends the given chat message to a player. This accepts exactly one
  /// argument, which can be any type. If it is a `PChat`, then it will be
  /// formatted correctly. Anything else will show up with debug formatting.
  ///
  /// This will do nothing if the player is offline.
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
        let chat = borrow.as_any().downcast_ref::<PChat>();
        if let Some(chat) = chat {
          chat.inner.lock().unwrap().clone()
        } else {
          Chat::new(msg.to_string())
        }
      }
      _ => Chat::new(msg.to_string()),
    };
    if let Ok(i) = self.inner() {
      i.send_message(out);
    }
  }

  /// Sets the title for this player. To show the title and subtitle, call
  /// [`show_title`].
  ///
  /// This will do nothing if the player is offline.
  pub fn set_title(&self, title: &PChat) {
    if let Ok(i) = self.inner() {
      i.set_title(title.inner.lock().unwrap().clone());
    }
  }
  /// Sets the subtitle for this player. To show the title and subtitle, call
  /// [`show_title`].
  ///
  /// This will do nothing if the player is offline.
  pub fn set_subtitle(&self, subtitle: &PChat) {
    if let Ok(i) = self.inner() {
      i.set_subtitle(subtitle.inner.lock().unwrap().clone());
    }
  }
  /// Shows the current title to the player. The `fade_in`, `stay`, and
  /// `fade_out` arguments are all in ticks.
  ///
  /// This will do nothing if the player is offline.
  pub fn show_title(&self, fade_in: u32, stay: u32, fade_out: u32) {
    if let Ok(i) = self.inner() {
      i.show_title(fade_in, stay, fade_out);
    }
  }

  /// Returns the world this player is in. This can be used to get/set
  /// blocks, access other players, and modify entities.
  ///
  /// This will return an error if the player is offline.
  pub fn world(&self) -> Result<PWorld> { Ok(self.inner()?.world().clone().into()) }

  /// Switches the player to a new server. If the server is found, the player
  /// will be disconnected after this call. If the server is not found, an error
  /// will be returned.
  ///
  /// This will do nothing if the player is offline.
  pub fn switch_to(&self, ip: &str) -> Result<()> {
    let ips: Vec<SocketAddr> = ip
      .to_socket_addrs()
      .map_err(|e| RuntimeError::custom(format!("invalid ip '{ip}': {e}"), Span::call_site()))?
      .collect();
    if let Ok(i) = self.inner() {
      i.switch_to(ips);
    }
    Ok(())
  }

  /// Shows an inventory to the player.
  ///
  /// This will do nothing if the player is offline.
  pub fn show_inventory(&self, inv: &PInventory, title: &PChat) {
    if let Ok(i) = self.inner() {
      i.show_inventory(inv.inner.clone(), &title.inner.lock().unwrap());
    }
  }
  /// Returns true if the player is in an inventory. This does not include their
  /// own inventory! The server cannot know if the player is in their survival
  /// inventory.
  pub fn in_window(&self) -> bool {
    if let Ok(i) = self.inner() {
      i.lock_inventory().win().is_some()
    } else {
      false
    }
  }
  pub fn get_item(&self, slot: i32) -> PStack {
    if let Ok(i) = self.inner() {
      i.lock_inventory().get(slot).clone().into()
    } else {
      Stack::empty().into()
    }
  }
  /// If the player has the items in the given stack, they will be removed, and
  /// `true` will be returned. If not, this will do nothing, and return `false`.
  ///
  /// If the stack is empty, this will always return `true`.
  pub fn try_remove_item(&self, stack: &PStack) -> bool {
    if stack.inner.amount() == 0 {
      return true;
    }
    if let Ok(i) = self.inner() {
      let mut inv = i.lock_inventory();
      let main = inv.main();
      let mut needed_amount = stack.inner.amount();
      for slot in 9..46 {
        let it = main.get(slot);
        if it.item() == stack.inner.item() {
          needed_amount = needed_amount.checked_sub(it.amount()).unwrap_or(0);
        }
        if needed_amount == 0 {
          break;
        }
      }
      if needed_amount == 0 {
        let mut amount_to_remove = stack.inner.amount();
        let main = inv.main_mut();
        for slot in 9..46 {
          let it = main.get_mut(slot);
          let mut sync = false;
          if it.item() == stack.inner.item() {
            if it.amount() <= amount_to_remove {
              it.set_amount(0);
              sync = true;
            } else {
              let new_amount = it.amount() - amount_to_remove;
              amount_to_remove = 0;
              it.set_amount(new_amount);
              sync = true;
            }
            needed_amount = needed_amount.checked_sub(it.amount()).unwrap_or(0);
          }
          if sync {
            main.sync(slot);
          }
          if amount_to_remove == 0 {
            break;
          }
        }
        true
      } else {
        false
      }
    } else {
      true
    }
  }

  /// Shows a scoreboard to the player. Call `set_scoreboard_line` to display
  /// anything in the scoreboard.
  ///
  /// This will do nothing if the player is offline.
  pub fn show_scoreboard(&self) {
    if let Ok(i) = self.inner() {
      i.lock_scoreboard().show();
    }
  }
  /// Hides the scoreboard for the player.
  ///
  /// This will do nothing if the player is offline.
  pub fn hide_scoreboard(&self) {
    if let Ok(i) = self.inner() {
      i.lock_scoreboard().hide();
    }
  }
  /// Sets a line in the scoreboard. If it is hidden, this will still work, and
  /// the updated lines will show when the scoreboard is shown again.
  ///
  /// This will do nothing if the player is offline.
  pub fn set_scoreboard_line(&self, line: u8, message: &PChat) {
    if let Ok(i) = self.inner() {
      i.lock_scoreboard().set_line(line, &message.inner.lock().unwrap());
    }
  }
  /// Clears a line in the scoreboard. If it is hidden, this will still work,
  /// and the updated lines will show when the scoreboard is shown again.
  ///
  /// This will do nothing if the player is offline.
  pub fn clear_scoreboard_line(&self, line: u8) {
    if let Ok(i) = self.inner() {
      i.lock_scoreboard().clear_line(line);
    }
  }

  /// Sets the player's tab list name.
  ///
  /// Note that this does not update the name above the player's head. The only
  /// way to do that is by adding this player to a team.
  ///
  /// This will produce inconsistent behavior if the player is on a team. Only
  /// use if needed. Using teams is going to be more reliable.
  ///
  /// This will do nothing if the player is offline.
  pub fn set_tab_name(&self, name: &PChat) {
    if let Ok(i) = self.inner() {
      i.set_tab_name(Some(name.inner.lock().unwrap().clone()));
    }
  }
  /// Removes the player's tab list name.
  ///
  /// This will do nothing if the player is offline.
  pub fn clear_tab_name(&self) {
    if let Ok(i) = self.inner() {
      i.set_tab_name(None);
    }
  }

  /// Gives the player the passed item.
  ///
  /// This will do nothing if the player is offline.
  pub fn give(&self, stack: &PStack) {
    if let Ok(i) = self.inner() {
      i.lock_inventory().give(&stack.inner);
    }
  }
}

/// A team. This is a group of players and entities, which all share a set
/// of properties. These properties include setting a username color, disabling
/// friendly fire, showing invisible teammates, and more.
///
/// This can be created through `Bamboo::create_team`.
#[define_ty(panda_path = "bamboo::player::Team")]
impl PTeam {
  /// Sets the color of this team. All players in this team will have their
  /// usernames displayed in this color.
  pub fn set_color(&self, name: &str) -> Result<()> {
    self.inner.lock().set_color(
      Color::from_str(name)
        .map_err(|err| RuntimeError::custom(err.to_string(), Span::call_site()))?,
    );
    Ok(())
  }

  /// Adds the player to this team.
  ///
  /// This will do nothing if the player is offline.
  pub fn add_player(&self, player: &PPlayer) {
    if let Ok(i) = player.inner() {
      self.inner.lock().add(i.as_ref());
    }
  }
}
