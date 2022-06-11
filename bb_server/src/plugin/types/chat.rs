use super::{add_from, wrap};
use bb_common::util::{chat::Color, Chat};
use bb_server_macros::define_ty;
use panda::{parse::token::Span, runtime::RuntimeError};
use std::{
  str::FromStr,
  sync::{Arc, Mutex},
};

wrap!(Arc<Mutex<Chat>>, PChat);
wrap!(Arc<Mutex<Chat>>, PChatSection, idx: usize);

/// A chat message. This is how you can send formatted chat message to players.
#[define_ty(panda_path = "bamboo::chat::Chat")]
impl PChat {
  /// Creates a new chat message with the given text.
  pub fn new(text: &str) -> PChat { PChat { inner: Arc::new(Mutex::new(Chat::new(text))) } }
  /// Creates an empty chat message. This can have sections added using `add`.
  pub fn empty() -> PChat { PChat { inner: Arc::new(Mutex::new(Chat::empty())) } }
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
  pub fn add(&self, msg: &str) -> PChatSection {
    let mut lock = self.inner.lock().unwrap();
    lock.add(msg);
    PChatSection { inner: self.inner.clone(), idx: lock.sections_len() - 1 }
  }
}

/// A chat message section. This section knows which chat message it came from.
/// All of the functions on this section will modify the chat message this came
/// from.
#[define_ty(panda_path = "bamboo::chat::ChatSection")]
impl PChatSection {
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
    self.inner.lock().unwrap().get_section(self.idx).unwrap().color(
      Color::from_str(color)
        .map_err(|err| RuntimeError::custom(err.to_string(), Span::call_site()))?,
    );
    Ok(())
  }
}
