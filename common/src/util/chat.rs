pub struct Chat {
  sections: Vec<ChatSection>,
}

impl Chat {
  /// Creates a new Chat message.
  pub fn new(msg: String) -> Self {
    Chat { sections: vec![ChatSection { text: msg, ..Default::default() }] }
  }

  /// Generates a json message that represents this chat message. This is used
  /// when serializing chat packets, and when dealing with things like books.
  pub fn to_json(&self) -> String {
    if self.sections.len() == 1 {
      return serde_json::serialize(self.sections[0]);
    }
  }
}

#[derive(Debug, Default)]
struct ChatSection {
  text:          String,
  bol:           Option<bool>,
  italic:        Option<bool>,
  underlined:    Option<bool>,
  strikethrough: Option<bool>,
  obfuscated:    Option<bool>,
  color:         Option<Color>,
  // Holding shift and clicking on this section will insert this text into the chat box.
  insertion:     Option<String>,
  // Clicking on this section will do something
  click_event:   Option<ClickEvent>,
  // Hovering over this section will do something
  hover_event:   Option<HoverEvent>,
  // Any child elements. If any of their options are None, then these options should be used.
  extra:         Vec<ChatSection>,
}

#[derive(Debug)]
pub enum ClickEvent {
  OpenURL(String),
  RunCommand(String),
  SuggestCommand(String),
  ChangePage(String),
  CopyToClipboard(String),
}

#[derive(Debug)]
pub enum HoverEvent {
  ShowText(String),
  ShowItem(String),
  ShowEntity(String),
}

#[derive(Debug)]
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
  Bright,
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
      Self::Bright => "green",
      Self::Cyan => "aqua",
      Self::Red => "red",
      Self::Pink => "pink",
      Self::Yellow => "yellow",
      Self::White => "white",
      Self::Custom(v) => &v,
    }
  }
}
