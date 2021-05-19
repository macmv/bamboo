use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::Serialize;

#[derive(Clone)]
pub struct Chat {
  sections: Vec<Section>,
}

impl Chat {
  /// Creates a new Chat message. This will contain a single section, with the
  /// given text set. No formatting will be applied.
  pub fn new(msg: String) -> Self {
    Chat { sections: vec![Section { text: msg, ..Default::default() }] }
  }
  /// Creates a new Chat message, with no sections.
  pub fn empty() -> Self {
    Chat { sections: vec![] }
  }

  /// Adds a new chat section, with the given string. The returned reference is
  /// a reference into self, so it must be dropped before adding another
  /// section.
  pub fn add(&mut self, msg: String) -> &mut Section {
    let s = Section { text: msg, ..Default::default() };
    let idx = self.sections.len();
    self.sections.push(s);
    self.sections.get_mut(idx).unwrap()
  }

  /// Generates a json message that represents this chat message. This is used
  /// when serializing chat packets, and when dealing with things like books.
  pub fn to_json(&self) -> String {
    if self.sections.is_empty() {
      "{}".into()
    } else if self.sections.len() == 1 {
      serde_json::to_string(&self.sections[0]).unwrap()
    } else {
      serde_json::to_string(&self.sections).unwrap()
    }
  }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Section {
  text:          String,
  #[serde(skip_serializing_if = "Option::is_none")]
  bold:          Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  italic:        Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  underlined:    Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  strikethrough: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  obfuscated:    Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  color:         Option<Color>,
  // Holding shift and clicking on this section will insert this text into the chat box.
  #[serde(skip_serializing_if = "Option::is_none")]
  insertion:     Option<String>,
  // Clicking on this section will do something
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "clickEvent")]
  click_event:   Option<ClickEvent>,
  // Hovering over this section will do something
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "hoverEvent")]
  hover_event:   Option<HoverEvent>,
  // Any child elements. If any of their options are None, then these options should be used.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  extra:         Vec<Section>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
  ($name: ident, $sname: expr) => {
    #[doc = "Makes this chat section "]
    #[doc = $sname]
    pub fn $name(&mut self) -> &mut Self {
      self.$name = Some(true);
      self
    }
  };
}

impl Section {
  add_bool!(bold, stringify!(bold));
  add_bool!(italic, stringify!(italic));
  add_bool!(underlined, stringify!(underlined));
  add_bool!(strikethrough, stringify!(strikethrough));
  add_bool!(obfuscated, stringify!(obfuscated));
  /// Applies the given color to this section
  pub fn color(&mut self, c: Color) -> &mut Self {
    self.color = Some(c);
    self
  }
  /// If a client shift-right-clicks on this section, the given text will be
  /// inserted into the chat box.
  pub fn insertion(&mut self, text: String) -> &mut Self {
    self.insertion = Some(text);
    self
  }
  /// When the client clicks on this section, something will happen.
  pub fn on_click(&mut self, e: ClickEvent) -> &mut Self {
    self.click_event = Some(e);
    self
  }
  /// When the client hoveres over this section, something will happen.
  pub fn on_hover(&mut self, e: HoverEvent) -> &mut Self {
    self.hover_event = Some(e);
    self
  }
  /// This adds a child section to this chat section. Any properities left blank
  /// on that child will be filled in from this section. If you want multiple
  /// chat sections in a row, you probably want to use [`Chat::add`] instead.
  /// This is instead useful for something like a hyperlink, where part of it
  /// should be a different color.
  pub fn add_extra(&mut self, msg: String) -> &mut Section {
    let s = Section { text: msg, ..Default::default() };
    let idx = self.extra.len();
    self.extra.push(s);
    self.extra.get_mut(idx).unwrap()
  }
}

#[derive(Debug, Clone)]
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
  pub fn rgb(r: u8, g: u8, b: u8) -> Self {
    Color::Custom(format!("#{:02x}{:02x}{:02x}", r, g, b))
  }

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
      Self::Custom(v) => &v,
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
      let mut msg = Chat::new("Hello!".into());
      assert_eq!(msg.to_json(), r#"{"text":"Hello!"}"#);

      msg.add(" more text".into()).bold().italic();
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
      m.add("colored".into()).color(Color::BrightGreen);
      assert_eq!(m.to_json(), r#"{"text":"colored","color":"green"}"#);

      let mut m = msg.clone();
      m.add("another color".into()).color(Color::Black);
      assert_eq!(m.to_json(), r#"{"text":"another color","color":"black"}"#);

      let mut m = msg.clone();
      m.add("custom color".into()).color(Color::rgb(0, 127, 255));
      assert_eq!(m.to_json(), r##"{"text":"custom color","color":"#007fff"}"##);
    }

    // Test all the other nonsense
    {
      // Insertion text
      let mut msg = Chat::empty();
      msg.add("click me!".into()).insertion("I am text".into());
      assert_eq!(msg.to_json(), r#"{"text":"click me!","insertion":"I am text"}"#);

      // Click event
      let mut msg = Chat::empty();
      msg.add("click me!".into()).on_click(ClickEvent::OpenURL("https://google.com".into()));
      assert_eq!(
        msg.to_json(),
        r#"{"text":"click me!","clickEvent":{"action":"open_url","value":"https://google.com"}}"#
      );

      // Hover event
      let mut msg = Chat::empty();
      msg.add("hover time".into()).on_hover(HoverEvent::ShowText("big gaming".into()));
      assert_eq!(
        msg.to_json(),
        r#"{"text":"hover time","hoverEvent":{"action":"show_text","value":"big gaming"}}"#
      );
    }
  }
}
