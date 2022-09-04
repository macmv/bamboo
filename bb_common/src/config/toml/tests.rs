use super::{Map, ParseErrorKind, Token, Tokenizer, Value};

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

#[test]
fn parsing() {
  let val: Value = "a = 3".parse().unwrap();

  let mut map = Map::new();
  map.insert("a".into(), Value::new(1, 3.into()));
  assert_eq!(val, Value::new_table(0, map));
}
