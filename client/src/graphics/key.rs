use num_derive::FromPrimitive;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive)]
pub enum KeyCode {
  None,

  Esc,
  Key1,
  Key2,
  Key3,
  Key4,
  Key5,
  Key6,
  Key7,
  Key8,
  Key9,
  Key0,
  Minus,
  Plus,
  Backspace,

  Tab,
  Q,
  W,
  E,
  R,
  T,
  Y,
  U,
  I,
  O,
  P,
  LeftBracket,
  RightBracket,

  Enter,
  LCtrl,

  A,
  S,
  D,
  F,
  G,
  H,
  J,
  K,
  L,
  Semicolon,
  Quote,

  Tilde,
  LShift,
  Backslash,

  Z,
  X,
  C,
  V,
  B,
  N,
  M,
  Comman,
  Period,
  Slash,

  RShift,
  NumpadStar,

  LAlt,
  Space,
  CapsLock,

  F1,
  F2,
  F3,
  F4,
  F5,
  F6,
  F7,
  F8,
  F9,
  F10,

  NumLock,
  ScrollLock,

  Numpad7,
  Numpad8,
  Numpad9,
  NumpadMinus,
  Numpad4,
  Numpad5,
  Numpad6,
  NumpadPlus,
  Numpad1,
  Numpad2,
  Numpad3,
  Numpad0,
  NumpadPeriod,
}
