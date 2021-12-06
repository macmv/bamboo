use super::{add_from, wrap};
use sc_common::util::{chat::Color, Chat};
use std::sync::{Arc, Mutex};
use sugarlang::{
  define_ty,
  parse::token::Span,
  runtime::{RuntimeError, VarData},
};

wrap!(Arc<Mutex<Chat>>, SlChat);
wrap!(Arc<Mutex<Chat>>, SlChatSection, idx: usize);

/// A chat message. This is how you can send formatted chat message to players.
#[define_ty(path = "sugarcane::chat::Chat")]
impl SlChat {
  /// Creates an empty chat message. This can have sections added using `add`.
  pub fn empty() -> SlChat { SlChat { inner: Arc::new(Mutex::new(Chat::empty())) } }
  /// Adds a new chat section. This will return the section that was just added,
  /// so that it can be modified.
  ///
  /// # Example
  ///
  /// ```
  /// chat = Chat::empty()
  ///
  /// chat.add("hello").color("red")
  /// //   ^^^^^^^^^^^^ ------------ This is a function on `ChatSection`, which changes it's color.
  /// //   |
  /// //    \ Adds the section "hello"
  /// ```
  pub fn add(&self, msg: &str) -> SlChatSection {
    let mut lock = self.inner.lock().unwrap();
    lock.add(msg);
    SlChatSection { inner: self.inner.clone(), idx: lock.sections_len() - 1 }
  }
}

/// A chat message section. This section knows which chat message it came from.
/// All of the functions on this section will modify the chat message this came
/// from.
#[define_ty(path = "sugarcane::chat::ChatSection")]
impl SlChatSection {
  /// Sets the color of this chat section. Since Sugarlang does not support
  /// enums, the color is simply a string. An invalid color will result in an
  /// error.
  ///
  /// # Example
  ///
  /// ```
  /// chat = Chat::empty()
  ///
  /// // Adds a new section, with the color set to red.
  /// chat.add("hello").color("red")
  /// ```
  pub fn color(&self, color: &str) -> Result<(), RuntimeError> {
    let col = match color {
      "red" => Color::Red,
      "yellow" => Color::Yellow,
      "green" => Color::BrightGreen,
      _ => return Err(RuntimeError::custom(format!("invalid color `{}`", color), Span::default())),
    };
    self.inner.lock().unwrap().get_section(self.idx).unwrap().color(col);
    Ok(())
  }
}
