//! The `chat` crate handles any chat messages in Minecraft. This is most
//! commonly used in the chat box, but is also used in written books and window
//! titles.
//!
//! A chat message is a list of [`Section`]s. Each of these sections has a text
//! component, and a bunch of styling options. To add a section to a chat
//! message, use [`Chat::add`]. This will add a section with the given text, and
//! no styling options.
//!
//! # Example
//!
//! ```rust
//! use bb_common::util::{Chat, chat::Color};
//!
//! let mut msg = Chat::new("Hello! ".to_string());
//!
//! msg.add("I am a section. ".to_string()).bold();
//! msg.add("I am another section".to_string()).color(Color::BrightGreen).italic();
//!
//! let json = msg.to_json();
//! assert_eq!(json, r#"[{"text":"Hello! "},{"text":"I am a section. ","bold":true},{"text":"I am another section","italic":true,"color":"green"}]"#);
//! ```
//!
//! This will make a chat message with three sections. The first one contains
//! the text `Hello! `, and has no formatting options. A space has been added to
//! the end so that the next section won't be right up against this text. The
//! next section has the text `I am a section. `, and is formatted in bold.
//! Finally, the last section `I am another section`, will be show in bright
//! green italics on the client.
//!
//! The resulting json is not very nice to look at, but it is what the Minecraft
//! client parses.

use serde::ser::{Serialize, SerializeMap, SerializeSeq, SerializeStruct, Serializer};
use serde_derive::{Deserialize, Serialize};

/// This is a chat message. It has a list of sections, and can be serialized to
/// json.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Chat {
  sections: Vec<Section>,
}

impl Chat {
  /// Creates a new Chat message. This will contain a single section, with the
  /// given text set. No formatting will be applied.
  pub fn new<M: Into<String>>(msg: M) -> Self {
    Chat { sections: vec![Section { text: msg.into(), ..Default::default() }] }
  }
  /// Creates a new Chat message, with no sections.
  pub fn empty() -> Self { Chat { sections: vec![] } }

  /// Adds a new chat section, with the given string. The returned reference is
  /// a reference into self, so it must be dropped before adding another
  /// section.
  pub fn add<M: Into<String>>(&mut self, msg: M) -> &mut Section {
    let s = Section { text: msg.into(), ..Default::default() };
    let idx = self.sections.len();
    self.sections.push(s);
    self.sections.get_mut(idx).unwrap()
  }

  /// Generates a json message that represents this chat message. This is used
  /// when serializing chat packets, and when dealing with things like books.
  pub fn to_json(&self) -> String { serde_json::to_string(self).unwrap() }

  /// Parses the given json as a chat message.
  pub fn from_json(src: String) -> Result<Self, serde_json::Error> {
    if src.starts_with('{') {
      let s: Section = serde_json::from_str(&src)?;
      Ok(Chat { sections: vec![s] })
    } else {
      let sections: Vec<Section> = serde_json::from_str(&src)?;
      Ok(Chat { sections })
    }
  }

  /// Generates a string for this chat message in plain text (no formatting).
  pub fn to_plain(&self) -> String {
    let mut out = String::new();
    for s in &self.sections {
      s.to_plain(&mut out);
    }
    out
  }

  /// Generates a color-coded string for this message. Depending on where the
  /// text is being rendered, this may be the only option that works. However,
  /// this is much less flexible than the json format, and there may be missing
  /// features.
  pub fn to_codes(&self) -> String {
    let mut out = String::new();
    for s in &self.sections {
      s.to_codes(&mut out);
    }
    out
  }

  pub fn sections_len(&self) -> usize { self.sections.len() }
  pub fn get_section(&mut self, idx: usize) -> Option<&mut Section> { self.sections.get_mut(idx) }
}

impl Serialize for Chat {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    if self.sections.is_empty() {
      let s = serializer.serialize_map(Some(0))?;
      s.end()
    } else if self.sections.len() == 1 {
      self.sections[0].serialize(serializer)
    } else {
      let mut s = serializer.serialize_seq(Some(self.sections.len()))?;
      for sec in &self.sections {
        s.serialize_element(sec)?;
      }
      s.end()
    }
  }
}

impl From<&str> for Chat {
  fn from(msg: &str) -> Chat { Chat::new(msg) }
}
impl From<String> for Chat {
  fn from(msg: String) -> Chat { Chat::new(msg) }
}

/// This is a chat message section. It has some text, and a lot of optional
/// fields:
/// - [`bold`]: If true, this section will be rendered in bold.
/// - [`italic`]: If true, this section will be rendered in italics.
/// - [`underlined`]: If true, this section will be rendered with an underline.
/// - [`strikethrough`]: If true, this section will be rendered with a line
///   through it.
/// - [`obfuscated`]: If true, this section will be rendered as random always
///   changing letters.
/// - [`color`]: This is the [`Color`] to render this section in.
/// - [`insertion`]: If the user shift-right-clicks on this section, the
///   insertion text will be added to the chat box.
/// - [`on_click`]: If a user clicks on this chat message, the given
///   [`ClickEvent`] will happen.
/// - [`on_hover`]: If a user hovers over this chat message, the given
///   [`HoverEvent`] will happen.
/// - [`add_child`]: Adds a child chat section. If any of the children's fields
///   are left blank, then it will copy then from this section.
///
/// [`bold`]: Self::bold
/// [`italic`]: Self::italic
/// [`underlined`]: Self::underlined
/// [`strikethrough`]: Self::strikethrough
/// [`obfuscated`]: Self::obfuscated
/// [`color`]: Self::color
/// [`insertion`]: Self::insertion
/// [`on_click`]: Self::on_click
/// [`on_hover`]: Self::on_hover
/// [`add_child`]: Self::add_child
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Section {
  text:          String,
  #[serde(skip_serializing_if = "Option::is_none", default)]
  bold:          Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", default)]
  italic:        Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", default)]
  underlined:    Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", default)]
  strikethrough: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", default)]
  obfuscated:    Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
  color:         Option<Color>,
  // Holding shift and clicking on this section will insert this text into the chat box.
  #[serde(skip_serializing_if = "Option::is_none", default)]
  insertion:     Option<String>,
  // Clicking on this section will do something
  #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
  #[serde(rename = "clickEvent")]
  click_event:   Option<ClickEvent>,
  // Hovering over this section will do something
  #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
  #[serde(rename = "hoverEvent")]
  hover_event:   Option<HoverEvent>,
  // Any child elements. If any of their options are None, then these options should be used.
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  extra:         Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClickEvent {
  OpenURL(String),
  RunCommand(String),
  SuggestCommand(String),
  ChangePage(String),
  CopyToClipboard(String),
}

impl Serialize for ClickEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut s = serializer.serialize_struct("clickEvent", 2)?;
    let (action, val) = match self {
      Self::OpenURL(v) => ("open_url", v),
      Self::RunCommand(v) => ("run_command", v),
      Self::SuggestCommand(v) => ("suggest_command", v),
      Self::ChangePage(v) => ("change_page", v),
      Self::CopyToClipboard(v) => ("copy_to_clipboard", v),
    };
    s.serialize_field("action", action)?;
    s.serialize_field("value", val)?;
    s.end()
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoverEvent {
  ShowText(String),
  ShowItem(String),
  ShowEntity(String),
}

impl Serialize for HoverEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut s = serializer.serialize_struct("hoverEvent", 2)?;
    let (action, val) = match self {
      Self::ShowText(v) => ("show_text", v),
      Self::ShowItem(v) => ("show_item", v),
      Self::ShowEntity(v) => ("show_entity", v),
    };
    s.serialize_field("action", action)?;
    s.serialize_field("value", val)?;
    s.end()
  }
}

macro_rules! add_bool {
  (
    $(#[$meta:meta])*
    $name: ident
  ) => (
    $(#[$meta])*
    pub fn $name(&mut self) -> &mut Self {
      self.$name = Some(true);
      self
    }
  )
}

impl Section {
  add_bool!(
    /// Makes this chat section bold.
    bold
  );
  add_bool!(
    /// Makes this chat section italic.
    italic
  );
  add_bool!(
    /// Makes this chat section underlined.
    underlined
  );
  add_bool!(
    /// Makes this chat section strikethrough (puts a line through the middle of
    /// it).
    strikethrough
  );
  add_bool!(
    /// Makes this chat section obfuscated. All the letters will be randomized
    /// constantly.
    obfuscated
  );
  /// Applies the given color to this section.
  pub fn color(&mut self, c: Color) -> &mut Self {
    self.color = Some(c);
    self
  }
  /// If a client shift-right-clicks on this section, the given text will be
  /// inserted into the chat box.
  pub fn insertion<M: Into<String>>(&mut self, text: M) -> &mut Self {
    self.insertion = Some(text.into());
    self
  }
  /// When the client clicks on this section, something will happen.
  pub fn on_click(&mut self, e: ClickEvent) -> &mut Self {
    self.click_event = Some(e);
    self
  }
  /// When the client hovers over this section, something will happen.
  pub fn on_hover(&mut self, e: HoverEvent) -> &mut Self {
    self.hover_event = Some(e);
    self
  }
  /// This adds a child section to this chat section. Any properities left blank
  /// on that child will be filled in from this section. If you want multiple
  /// chat sections in a row, you probably want to use [`Chat::add`] instead.
  /// This is instead useful for something like a hyperlink, where part of it
  /// should be a different color.
  pub fn add_child<M: Into<String>>(&mut self, msg: M) -> &mut Section {
    let s = Section { text: msg.into(), ..Default::default() };
    let idx = self.extra.len();
    self.extra.push(s);
    self.extra.get_mut(idx).unwrap()
  }

  fn to_plain(&self, out: &mut String) {
    out.push_str(&self.text);
    for e in &self.extra {
      e.to_plain(out);
    }
  }

  fn to_codes(&self, out: &mut String) {
    if self.bold == Some(true) {
      out.push_str("§l");
    }
    if self.italic == Some(true) {
      out.push_str("§o");
    }
    if self.underlined == Some(true) {
      out.push_str("§u");
    }
    if self.strikethrough == Some(true) {
      out.push_str("§m");
    }
    if self.obfuscated == Some(true) {
      out.push_str("§k");
    }
    if let Some(c) = &self.color {
      if c != &Color::White {
        out.push('§');
        out.push(c.code());
      }
    }
    out.push_str(&self.text);
    for e in &self.extra {
      e.to_codes(out);
    }
    // This is the lazy way, of just resetting after every section. This is
    // meant to be after subsections, so that they keep the formatting of their
    // parent. This only works for a single level deep of children, but this format
    // isn't meant to be complete anyways.
    //
    // TODO: Implement resetting/color codes the correct way, where we track the
    // color state between every section, and find the difference between each one.
    out.push_str("§r");
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Color {
  Black,
  DarkBlue,
  DarkGreen,
  DarkAqua,
  DarkRed,
  Purple,
  Gold,
  Gray,
  DarkGray,
  Blue,
  BrightGreen,
  Cyan,
  Red,
  Pink,
  Yellow,
  White,
  Custom(String),
}

impl Color {
  /// Creates a new rgb color. This is only valid for 1.16+ clients. For older
  /// clients, this will render as white.
  pub fn rgb(r: u8, g: u8, b: u8) -> Self { Color::Custom(format!("#{:02x}{:02x}{:02x}", r, g, b)) }

  /// Converts the color to a string. This string should be used in chat json.
  pub fn to_str(&self) -> &str {
    match self {
      Self::Black => "black",
      Self::DarkBlue => "dark_blue",
      Self::DarkGreen => "dark_green",
      Self::DarkAqua => "dark_aqua",
      Self::DarkRed => "dark_red",
      Self::Purple => "dark_purple",
      Self::Gold => "gold",
      Self::Gray => "gray",
      Self::DarkGray => "dark_gray",
      Self::Blue => "blue",
      Self::BrightGreen => "green",
      Self::Cyan => "aqua",
      Self::Red => "red",
      Self::Pink => "pink",
      Self::Yellow => "yellow",
      Self::White => "white",
      Self::Custom(v) => v,
    }
  }

  /// Returns the color code for this color.
  pub const fn code(&self) -> char {
    match self {
      Self::Black => '0',
      Self::DarkBlue => '1',
      Self::DarkGreen => '2',
      Self::DarkAqua => '3',
      Self::DarkRed => '4',
      Self::Purple => '5',
      Self::Gold => '6',
      Self::Gray => '7',
      Self::DarkGray => '8',
      Self::Blue => '9',
      Self::BrightGreen => 'a',
      Self::Cyan => 'b',
      Self::Red => 'c',
      Self::Pink => 'd',
      Self::Yellow => 'e',
      Self::White => 'f',
      Self::Custom(_) => 'f',
    }
  }
}

impl Serialize for Color {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.to_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serialize() {
    // Test the basics of serialization
    {
      let mut msg = Chat::new("Hello!");
      assert_eq!(msg.to_json(), r#"{"text":"Hello!"}"#);

      msg.add(" more text").bold().italic();
      assert_eq!(
        msg.to_json(),
        r#"[{"text":"Hello!"},{"text":" more text","bold":true,"italic":true}]"#
      );
    }

    // Test serializing colors
    {
      let msg = Chat::empty();
      assert_eq!(msg.to_json(), r#"{}"#);

      let mut m = msg.clone();
      m.add("colored").color(Color::BrightGreen);
      assert_eq!(m.to_json(), r#"{"text":"colored","color":"green"}"#);

      let mut m = msg.clone();
      m.add("another color").color(Color::Black);
      assert_eq!(m.to_json(), r#"{"text":"another color","color":"black"}"#);

      let mut m = msg.clone();
      m.add("custom color").color(Color::rgb(0, 127, 255));
      assert_eq!(m.to_json(), r##"{"text":"custom color","color":"#007fff"}"##);
    }

    // Test all the other nonsense
    {
      // Insertion text
      let mut msg = Chat::empty();
      msg.add("click me!").insertion("I am text");
      assert_eq!(msg.to_json(), r#"{"text":"click me!","insertion":"I am text"}"#);

      // Click event
      let mut msg = Chat::empty();
      msg.add("click me!").on_click(ClickEvent::OpenURL("https://google.com".into()));
      assert_eq!(
        msg.to_json(),
        r#"{"text":"click me!","clickEvent":{"action":"open_url","value":"https://google.com"}}"#
      );

      // Hover event
      let mut msg = Chat::empty();
      msg.add("hover time").on_hover(HoverEvent::ShowText("big gaming".into()));
      assert_eq!(
        msg.to_json(),
        r#"{"text":"hover time","hoverEvent":{"action":"show_text","value":"big gaming"}}"#
      );

      // Children testing
      let mut msg = Chat::empty();
      // This adds a child section to this first section. This child will be rendered
      // in bold and italics, even though `bold` is not set to true on the child.
      msg.add("main section ").bold().add_child("hello").italic();
      assert_eq!(
        msg.to_json(),
        r#"{"text":"main section ","bold":true,"extra":[{"text":"hello","italic":true}]}"#
      );
    }
  }
}