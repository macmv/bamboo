use super::{Map, ParseErrorKind, Token, Tokenizer, Value, ValueInner};

#[test]
fn tokens() {
  use ParseErrorKind::*;
  use Token::*;

  let mut tok = Tokenizer::new("a = 3");

  assert_eq!(tok.next(), Ok(Word("a")));
  assert_eq!(tok.next(), Ok(Eq));
  assert_eq!(tok.next(), Ok(Integer(3)));
  assert_eq!(tok.next(), Err(tok.err(UnexpectedEOF)));

  let mut tok = Tokenizer::new("a = 3");

  let t = tok.next().unwrap();
  assert_eq!(t, Word("a"));
  tok.peeked = Some(t);
  assert_eq!(tok.next(), Ok(Word("a")));
  assert_eq!(tok.next(), Ok(Eq));
  assert_eq!(tok.next(), Ok(Integer(3)));
  assert_eq!(tok.next(), Err(tok.err(UnexpectedEOF)));
}

#[track_caller]
fn assert_value(toml: &str, value: impl Into<ValueInner>) {
  let val: Value = toml.parse().unwrap();

  let mut map = Map::new();
  map.insert("a".into(), Value::new(1, value.into()));
  assert_eq!(val, Value::new_table(0, map));
}

#[test]
fn parsing() {
  assert_value("a = 2", 2);
  assert_value("a = 1.2", 1.2);
  assert_value("a = true", true);
}
