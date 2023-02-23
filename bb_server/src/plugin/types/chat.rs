use bb_common::util::{chat::Color, Chat};
use bb_server_macros::define_ty;
use panda::{
  parse::token::Span,
  runtime::{Result, RuntimeError, Var},
};
use parking_lot::Mutex;
use std::{str::FromStr, sync::Arc};

impl From<Chat> for PChat {
  fn from(c: Chat) -> PChat { PChat { inner: Arc::new(Mutex::new(c)) } }
}

impl PChat {
  pub fn from_var(var: Var) -> Chat {
    match var {
      Var::Builtin(_, ref data) => {
        let borrow = data.borrow();
        let chat = borrow.as_any().downcast_ref::<PChat>();
        chat.map(|c| c.inner.lock().clone()).unwrap_or_else(|| Chat::new(var.to_string()))
      }
      _ => Chat::new(var.to_string()),
    }
  }
}

/// A chat message. This is how you can send formatted chat message to players.
#[define_ty]
impl PChat {
  info! {
    wrap: Arc<Mutex<Chat>>,

    panda: {
      path: "bamboo::chat::Chat",
    },
    python: {
      class: "Chat",
    },
  }

  /// Creates a new chat message with the given text.
  pub fn new(text: &str) -> PChat { PChat { inner: Arc::new(Mutex::new(Chat::new(text))) } }
  /// Sets the color of the chat message. This won't do anything if the chat
  /// message has multiple sections.
  ///
  /// This is intended to be used with `new`, like so:
  /// ```
  /// chat = Chat::new("hello").color("green")
  /// //     ^^^^^^^^^^^^^^^^^^ -------------- This function
  /// //     |
  /// //      \ Creates the chat messaage "hello"
  /// ```
  pub fn color(&self, color: &str) -> Result<Self> {
    let mut lock = self.inner.lock();
    if lock.sections_len() == 1 {
      lock.get_section(0).unwrap().color(
        Color::from_str(color)
          .map_err(|err| RuntimeError::custom(err.to_string(), Span::call_site()))?,
      );
    }
    Ok(self.clone())
  }
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
    let mut lock = self.inner.lock();
    lock.add(msg);
    PChatSection { inner: self.inner.clone(), idx: lock.sections_len() - 1 }
  }
}

/// A chat message section. This section knows which chat message it came from.
/// All of the functions on this section will modify the chat message this came
/// from.
#[define_ty]
impl PChatSection {
  info! {
    fields: {
      inner: Arc<Mutex<Chat>>,
      idx: usize,
    },

    panda: {
      path: "bamboo::chat::ChatSection",
    },
    python: {
      class: "ChatSection",
    },
  }
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
  pub fn color(&self, color: &str) -> Result<Self> {
    self.inner.lock().get_section(self.idx).unwrap().color(
      Color::from_str(color)
        .map_err(|err| RuntimeError::custom(err.to_string(), Span::call_site()))?,
    );
    Ok(self.clone())
  }
}
