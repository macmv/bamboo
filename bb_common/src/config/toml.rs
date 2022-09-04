use indexmap::IndexMap;
use std::{fmt, str::FromStr};

// This uses very similar techniques to the `toml` crate, but adds support for
// serializing comments from values.

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
  pub comments: Vec<String>,
  pub value:    ValueInner,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueInner {
  String(String),
  Integer(i64),
  Float(f64),
  Boolean(bool),
  // Datetime(Datetime),
  Array(Vec<Value>),
  Table(IndexMap<String, Value>),
}

pub struct ParseError {
  pub line: u32,
  pub kind: ParseErrorKind,
}
pub enum ParseErrorKind {
  MissingValue,
  UnexpectedEOF,
  Other(String),
}

impl Value {
  pub fn new(value: ValueInner) -> Self { Value { comments: vec![], value } }
  pub fn new_array() -> Self { Self::new(ValueInner::Array(vec![])) }
  pub fn new_table() -> Self { Self::new(ValueInner::Table(IndexMap::new())) }

  pub fn is_array(&self) -> bool { matches!(self.value, ValueInner::Array(_)) }
  pub fn is_table(&self) -> bool { matches!(self.value, ValueInner::Table(_)) }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "line {}: {}", self.line, self.kind)
  }
}
impl fmt::Display for ParseErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::MissingValue => write!(f, "missing value after `=`"),
      Self::UnexpectedEOF => write!(f, "unexpected end of file"),
      Self::Other(s) => write!(f, "{s}"),
    }
  }
}

struct Tokenizer<'a> {
  s:     &'a str,
  index: usize,
  line:  usize,
}
enum Token<'a> {
  Comment(&'a str),
  Word(&'a str),
  String(String),
  Integer(i64),
  Boolean(bool),
}
impl<'a> Tokenizer<'a> {
  pub fn new(s: &'a str) -> Self { Tokenizer { s, index: 0, line: 0 } }
  pub fn next(&mut self) -> Option<Token<'a>> {
    let mut found_word = false;
    let mut found_number = false;
    let mut found_string = false;
    let start = self.index;
    while self.index < self.s.len() {
      let c = match self.s.get(self.index..) {
        Some(s) => s.chars().next()?,
        None => {
          self.index += 1;
          continue;
        }
      };
      self.index += c.len_utf8();
      match c {
        s if s.is_ascii_alphabetic() => found_word = true,
        s if s.is_ascii_digit() => found_number = true,
        '"' if !found_string => found_string = true,
        '"' if found_string => {
          return Some(Token::String(self.s[start + 1..self.index - 1].into()))
        }
        _ => {
          if c == '\n' {
            self.line += 1;
          }
          if found_word {
            return Some(Token::Word(&self.s[start..self.index - 1]));
          } else if found_number {
            return Some(Token::Integer(self.s[start..self.index - 1].parse().ok()?));
          } else {
            continue;
          }
        }
      }
    }
    None
  }
}

impl FromStr for Value {
  type Err = ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut tok = Tokenizer::new(s);

    let mut comments = vec![];
    loop {
      match tok.next() {
        Some(Token::Comment(c)) => comments.push(c),
        Some(_) => todo!(),
        None => break,
      }
    }
  }
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.value.fmt(f) }
}
impl fmt::Display for ValueInner {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::String(v) => write!(f, "\"{v}\""),
      Self::Integer(v) => write!(f, "{v}"),
      Self::Float(v) => write!(f, "{v}"),
      Self::Boolean(v) => write!(f, "{v}"),
      Self::Array(v) => {
        write!(f, "[{}]", v.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", "))
      }
      Self::Table(v) => write!(
        f,
        "{{{}}}",
        v.iter().map(|(k, v)| format!("{k} = {v}")).collect::<Vec<String>>().join(", ")
      ),
    }
  }
}

pub struct Path {
  pub segments: Vec<String>,
}

impl fmt::Display for Path {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, elem) in self.segments.iter().enumerate() {
      if i != 0 {
        write!(f, ".")?;
      }
      write!(f, "{elem}")?;
    }
    Ok(())
  }
}

impl Value {
  fn fmt_long(&self, path: Path, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for line in &self.comments {
      write!(f, "# {line}")?;
    }
    self.value.fmt_long(path, f)
  }
}
impl ValueInner {
  fn fmt_long(&self, path: Path, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::String(v) => write!(f, "\"{v}\""),
      Self::Integer(v) => write!(f, "{v}"),
      Self::Float(v) => write!(f, "{v}"),
      Self::Boolean(v) => write!(f, "{v}"),
      Self::Array(arr) => {
        write!(f, "[{}]", arr.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", "))
      }
      Self::Table(table) => {
        if table.values().any(|v| !v.comments.is_empty() || v.is_array() || v.is_table()) {
          if !path.segments.is_empty() {
            write!(f, "[{path}]")?;
          }
          for (k, v) in table {
            path.segments.push(k.clone());
            v.fmt_long(path, f)?;
            path.segments.pop();
          }
          Ok(())
        } else {
          write!(
            f,
            "{{{}}}",
            table.iter().map(|(k, v)| format!("{k} = {v}")).collect::<Vec<String>>().join(", ")
          )
        }
      }
    }
  }
}
