use super::{add_from, wrap};
use bb_common::util::{chat::Color, Chat};
use std::sync::{Arc, Mutex};
use panda::{define_ty, parse::token::Span, runtime::RuntimeError};

wrap!(Arc<Mutex<Chat>>, PdChat);
wrap!(Arc<Mutex<Chat>>, PdChatSection, idx: usize);

/// A chat message. This is how you can send formatted chat message to players.
#[define_ty(path = "bamboo::chat::Chat")]
impl PdChat {
  /// Creates a new chat message with the given text.
  pub fn new(text: &str) -> PdChat { PdChat { inner: Arc::new(Mutex::new(Chat::new(text))) } }
  /// Creates an empty chat message. This can have sections added using `add`.
  pub fn empty() -> PdChat { PdChat { inner: Arc::new(Mutex::new(Chat::empty())) } }
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
  pub fn add(&self, msg: &str) -> PdChatSection {
    let mut lock = self.inner.lock().unwrap();
    lock.add(msg);
    PdChatSection { inner: self.inner.clone(), idx: lock.sections_len() - 1 }
  }
}

/// A chat message section. This section knows which chat message it came from.
/// All of the functions on this section will modify the chat message this came
/// from.
#[define_ty(path = "bamboo::chat::ChatSection")]
impl PdChatSection {
  /// Sets the color of this chat section. Since Panda does not support
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
      "black" => Color::Black,
      "dark_blue" => Color::DarkBlue,
      "dark_green" => Color::DarkGreen,
      "dark_aqua" => Color::DarkAqua,
      "dark_red" => Color::DarkRed,
      "dark_purple" => Color::Purple,
      "gold" => Color::Gold,
      "gray" => Color::Gray,
      "dark_gray" => Color::DarkGray,
      "blue" => Color::Blue,
      "green" => Color::BrightGreen,
      "aqua" => Color::Cyan,
      "red" => Color::Red,
      "pink" => Color::Pink,
      "yellow" => Color::Yellow,
      "white" => Color::White,
      _ => {
        return Err(RuntimeError::custom(format!("invalid color `{}`", color), Span::call_site()))
      }
    };
    self.inner.lock().unwrap().get_section(self.idx).unwrap().color(col);
    Ok(())
  }
}
