use super::{Map, ParseErrorKind, Token, Tokenizer, Value, ValueInner};
use indexmap::indexmap;

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

  let mut tok = Tokenizer::new("a = [1, 2]");

  assert_eq!(tok.next(), Ok(Word("a")));
  assert_eq!(tok.next(), Ok(Eq));
  assert_eq!(tok.next(), Ok(OpenArr));
  assert_eq!(tok.next(), Ok(Integer(1)));
  assert_eq!(tok.next(), Ok(Comma));
  assert_eq!(tok.next(), Ok(Integer(2)));
  assert_eq!(tok.next(), Ok(CloseArr));
  assert_eq!(tok.next(), Err(tok.err(UnexpectedEOF)));
}

#[track_caller]
fn assert_val(toml: &str, value: impl Into<ValueInner>) {
  let val: Value = toml.parse().unwrap();

  let mut map = Map::new();
  map.insert("a".into(), Value::new(1, value.into()));
  assert_eq!(val, Value::new(0, map));
}
#[track_caller]
fn assert_value(toml: &str, value: Value) {
  let val: Value = toml.parse().unwrap();

  assert_eq!(val, value);
}
#[track_caller]
fn assert_fail(toml: &str, error: &str) {
  assert_eq!(toml.parse::<Value>().unwrap_err().to_string(), error);
}

#[test]
fn parsing() {
  assert_val("a = 2", 2);
  assert_val("a = 1.2", 1.2);
  assert_val("a = true", true);
  assert_val("a = false", false);
  assert_val("a = []", vec![]);
  assert_val("a = {}", indexmap! {});
  assert_val("a = [1, 2, 3]", vec![Value::new(1, 1), Value::new(1, 2), Value::new(1, 3)]);
  assert_val(
    "a = { x = 2, y = 3, z = 4 }",
    indexmap! {
      "x".to_string() => Value::new(1, 2),
      "y".to_string() => Value::new(1, 3),
      "z".to_string() => Value::new(1, 4),
    },
  );
  assert_value(
    "# hello\na = 2",
    Value::new(
      0,
      indexmap! {
        "a".into() => Value::new(2, 2).with_comment("hello"),
      },
    ),
  );

  assert_fail("a = \n1", "line 1: unexpected end of line");
  assert_fail("a =\n", "line 1: unexpected end of line");
  assert_fail("a =$", "line 1: unexpected token `$`");
  assert_fail("a = [1,\n2, 3]", "line 1: unexpected end of line");
  assert_fail("\na =", "line 2: unexpected end of file");

  assert_fail("a = [1 2]", "line 1: expected `,` or `]`, got integer");
  assert_fail("a = ]", "line 1: missing value after `=`");

  assert_value("a_b = true", Value::new(0, indexmap! { "a_b".into() => Value::new(1, true) }));
  assert_value("a-b = true", Value::new(0, indexmap! { "a-b".into() => Value::new(1, true) }));
}

#[test]
fn parse_map() {
  assert_value(
    r#"
    a = 2
    b = 3

    [options]
    foo = 5
    "#,
    Value::new(
      0,
      indexmap! {
        "a".into() => Value::new(2, 2),
        "b".into() => Value::new(3, 3),
        "options".into() => Value::new(5, indexmap! {
          "foo".into() => Value::new(6, 5),
        }),
      },
    ),
  );
}

#[test]
fn display() {
  assert_eq!(ValueInner::from(3).to_string(), "3");
  assert_eq!(ValueInner::from("hello").to_string(), "\"hello\"");
}

#[test]
fn write_value() {
  let value = Value::new(
    0,
    indexmap! {
      "foo".into() => Value::new(0, 3),
      "bar".into() => Value::new(0, 4),
      "options".into() => Value::new(0, indexmap! {
        "baz".into() => Value::new(0, 2),
        "other".into() => Value::new(0, 100),
      }),
    },
  );

  assert_eq!(
    value.to_toml(),
    r#"
    foo = 3
    bar = 4

    [options]
    baz = 2
    other = 100
    "#
    .lines()
    .skip(1)
    .map(|line| line.trim())
    .collect::<Vec<&str>>()
    .join("\n"),
  );

  // writes nested objects
  let value = Value::new(
    0,
    indexmap! {
      "foo".into() => Value::new(0, 3),
      "bar".into() => Value::new(0, 4),
      "options".into() => Value::new(0, indexmap! {
        "baz".into() => Value::new(0, 2),
        "other".into() => Value::new(0, indexmap! {
          "a".into() => Value::new(0, 99),
          "b".into() => Value::new(0, 999),
        }),
      }),
      "blah".into() => Value::new(0, indexmap! {
        "c".into() => Value::new(0, "hello!"),
      }),
    },
  );

  assert_eq!(
    value.to_toml(),
    r#"
    foo = 3
    bar = 4

    [options]
    baz = 2

    [options.other]
    a = 99
    b = 999

    [blah]
    c = "hello!"
    "#
    .lines()
    .skip(1)
    .map(|line| line.trim())
    .collect::<Vec<&str>>()
    .join("\n"),
  );

  // writes nested objects at the top level
  let value = Value::new(
    0,
    indexmap! {
      "foo".into() => Value::new(0, 3),
      "options".into() => Value::new(0, indexmap! {
        "baz".into() => Value::new(0, 2),
        "other".into() => Value::new(0, 100),
      }),
      "bar".into() => Value::new(0, 4),
    },
  );

  assert_eq!(
    value.to_toml(),
    r#"
    foo = 3
    options = { baz = 2, other = 100 }
    bar = 4
    "#
    .lines()
    .skip(1)
    .map(|line| line.trim())
    .collect::<Vec<&str>>()
    .join("\n"),
  );
}

#[test]
fn read_comments() {
  assert_value(
    &r#"
    # hello
    # world
    foo = 3
    # foo
    # bar
    bar = 4

    # This is blah
    [blah]
    # I am comment
    # I am more comment
    baz = true
    "#
    .lines()
    .skip(1)
    .map(|line| line.trim())
    .collect::<Vec<&str>>()
    .join("\n"),
    Value::new(
      0,
      indexmap! {
        "foo".into() => Value::new(3, 3).with_comment("hello").with_comment("world"),
        "bar".into() => Value::new(6, 4).with_comment("foo").with_comment("bar"),
        "blah".into() => Value::new(9, indexmap! {
          "baz".into() => Value::new(12, true).with_comment("I am comment").with_comment("I am more comment"),
        }).with_comment("This is blah"),
      },
    ),
  );
}

#[test]
fn write_comments() {
  let value = Value::new(
    0,
    indexmap! {
      "foo".into() => Value::new(0, 3).with_comment("hello").with_comment("world"),
      "bar".into() => Value::new(0, 4).with_comment("foo").with_comment("bar"),
    },
  );

  assert_eq!(
    value.to_toml(),
    r#"
    # hello
    # world
    foo = 3
    # foo
    # bar
    bar = 4
    "#
    .lines()
    .skip(1)
    .map(|line| line.trim())
    .collect::<Vec<&str>>()
    .join("\n"),
  );
}
