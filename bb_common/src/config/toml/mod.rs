use indexmap::IndexMap;
use std::{fmt, str::FromStr};

#[cfg(test)]
mod tests;

// This uses very similar techniques to the `toml` crate, but adds support for
// serializing comments from values.

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
  pub comments: Vec<String>,
  pub line:     usize,
  pub value:    ValueInner,
}

pub type Array = Vec<Value>;
pub type Map = IndexMap<String, Value>;

#[derive(Clone, Debug, PartialEq)]
pub enum ValueInner {
  String(String),
  Integer(i64),
  Float(f64),
  Boolean(bool),
  // Datetime(Datetime),
  Array(Array),
  Table(Map),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  pub line: usize,
  pub kind: ParseErrorKind,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
  MissingValue,
  MissingEq,
  UnexpectedEOF,
  UnexpectedToken(char),
  InvalidInteger(std::num::ParseIntError),
  InvalidFloat(std::num::ParseFloatError),
  Other(String),
}

impl From<&str> for ValueInner {
  fn from(s: &str) -> Self { ValueInner::String(s.into()) }
}
impl From<bool> for ValueInner {
  fn from(v: bool) -> Self { ValueInner::Boolean(v) }
}
impl From<f64> for ValueInner {
  fn from(v: f64) -> Self { ValueInner::Float(v) }
}
impl From<i64> for ValueInner {
  fn from(v: i64) -> Self { ValueInner::Integer(v) }
}

impl Value {
  pub fn new(line: usize, value: ValueInner) -> Self { Value { comments: vec![], line, value } }
  pub fn new_array(line: usize, arr: Array) -> Self { Self::new(line, ValueInner::Array(arr)) }
  pub fn new_table(line: usize, map: Map) -> Self { Self::new(line, ValueInner::Table(map)) }

  pub fn is_array(&self) -> bool { matches!(self.value, ValueInner::Array(_)) }
  pub fn is_table(&self) -> bool { matches!(self.value, ValueInner::Table(_)) }

  pub fn as_str(&self) -> Option<&String> {
    match &self.value {
      ValueInner::String(a) => Some(a),
      _ => None,
    }
  }
  pub fn as_integer(&self) -> Option<i64> {
    match &self.value {
      ValueInner::Integer(v) => Some(*v),
      _ => None,
    }
  }
  pub fn as_float(&self) -> Option<f64> {
    match &self.value {
      ValueInner::Float(v) => Some(*v),
      _ => None,
    }
  }
  pub fn as_bool(&self) -> Option<bool> {
    match &self.value {
      ValueInner::Boolean(v) => Some(*v),
      _ => None,
    }
  }
  pub fn as_array(&self) -> Option<&Array> {
    match &self.value {
      ValueInner::Array(a) => Some(a),
      _ => None,
    }
  }
  pub fn as_table(&self) -> Option<&Map> {
    match &self.value {
      ValueInner::Table(t) => Some(t),
      _ => None,
    }
  }
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
      Self::MissingEq => write!(f, "missing `=`"),
      Self::UnexpectedEOF => write!(f, "unexpected end of file"),
      Self::UnexpectedToken(c) => write!(f, "unexpected token `{c}`"),
      Self::InvalidInteger(e) => write!(f, "invalid integer: {e}"),
      Self::InvalidFloat(e) => write!(f, "invalid float: {e}"),
      Self::Other(s) => write!(f, "{s}"),
    }
  }
}
impl From<std::num::ParseIntError> for ParseErrorKind {
  fn from(e: std::num::ParseIntError) -> Self { ParseErrorKind::InvalidInteger(e) }
}
impl From<std::num::ParseFloatError> for ParseErrorKind {
  fn from(e: std::num::ParseFloatError) -> Self { ParseErrorKind::InvalidFloat(e) }
}

struct Tokenizer<'a> {
  s:      &'a str,
  peeked: Option<Token<'a>>,
  index:  usize,
  line:   usize,
}
#[derive(Debug, Clone, PartialEq)]
enum Token<'a> {
  Comment(&'a str),
  Word(&'a str),
  String(String),
  Integer(i64),
  Float(f64),
  Boolean(bool),

  Eq,
}
impl<'a> Tokenizer<'a> {
  pub fn new(s: &'a str) -> Self { Tokenizer { s, peeked: None, index: 0, line: 1 } }
  pub fn next_opt(&mut self) -> Result<Option<Token<'a>>, ParseError> {
    match self.next() {
      Ok(t) => Ok(Some(t)),
      Err(e) if e.kind == ParseErrorKind::UnexpectedEOF => Ok(None),
      Err(e) => Err(e),
    }
  }
  pub fn next(&mut self) -> Result<Token<'a>, ParseError> {
    if let Some(t) = self.peeked.take() {
      return Ok(t);
    }
    let mut found_word = false;
    let mut found_number = false;
    let mut found_float = false;
    let mut found_string = false;
    let start = self.index;
    loop {
      let c = match self.s.get(self.index..) {
        Some(s) => s.chars().next(),
        None => {
          self.index += 1;
          continue;
        }
      };
      if let Some(c) = c {
        self.index += c.len_utf8();
      }
      match c {
        Some('"') if !found_string => found_string = true,
        Some('"') if found_string => {
          return Ok(Token::String(self.s[start + 1..self.index - 1].trim().into()))
        }
        Some(_) if found_string => continue,

        c if c.map(|c| !c.is_ascii_digit() && c != '.').unwrap_or(true) && found_number => {
          if found_float {
            return Ok(Token::Float(
              self.s[start..self.index].trim().parse::<f64>().map_err(|e| self.err(e.into()))?,
            ));
          } else {
            return Ok(Token::Integer(
              self.s[start..self.index].trim().parse::<i64>().map_err(|e| self.err(e.into()))?,
            ));
          }
        }
        c if c.map(|c| !c.is_ascii_alphabetic()).unwrap_or(true) && found_word => {
          let word = self.s[start..self.index].trim();
          match word {
            "true" => return Ok(Token::Boolean(true)),
            "false" => return Ok(Token::Boolean(true)),
            _ => return Ok(Token::Word(word)),
          }
        }

        Some(c) if c.is_ascii_digit() => found_number = true,
        Some('.') if found_number => found_float = true,

        Some(c) if c.is_ascii_alphabetic() => found_word = true,

        Some('=') => return Ok(Token::Eq),

        Some('\n') => self.line += 1,
        Some(c) if c.is_whitespace() => continue,
        Some(c) => return Err(self.err(ParseErrorKind::UnexpectedToken(c))),
        None => return Err(self.err(ParseErrorKind::UnexpectedEOF)),
      }
    }
  }
  pub fn err(&self, kind: ParseErrorKind) -> ParseError { ParseError { line: self.line, kind } }

  fn parse_comments(&mut self) -> Result<Vec<String>, ParseError> {
    let mut comments = vec![];
    loop {
      match self.next_opt()? {
        Some(Token::Comment(c)) => comments.push(c.into()),
        Some(v) => {
          self.peeked = Some(v);
          return Ok(comments);
        }
        None => return Ok(comments),
      }
    }
  }
  fn parse_map(&mut self) -> Result<Map, ParseError> {
    let mut comments = self.parse_comments()?;
    let mut map = Map::new();
    loop {
      match self.next_opt()? {
        Some(Token::Word(key)) => {
          match self.next()? {
            Token::Eq => {}
            _ => break Err(self.err(ParseErrorKind::MissingEq)),
          }
          let value = match self.next()? {
            Token::String(s) => ValueInner::String(s),
            Token::Integer(v) => ValueInner::Integer(v),
            Token::Float(v) => ValueInner::Float(v),
            Token::Boolean(v) => ValueInner::Boolean(v),
            _ => break Err(self.err(ParseErrorKind::MissingValue)),
          };
          map.insert(
            key.into(),
            Value {
              comments: std::mem::replace(&mut comments, self.parse_comments()?),
              line: self.line,
              value,
            },
          );
        }
        _ => return Ok(map),
      }
    }
  }
}

impl FromStr for Value {
  type Err = ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut tok = Tokenizer::new(s);

    Ok(Value::new_table(0, tok.parse_map()?))
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
  fn fmt_long(&self, path: &mut Path, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for line in &self.comments {
      write!(f, "# {line}")?;
    }
    self.value.fmt_long(path, f)
  }
}
impl ValueInner {
  fn fmt_long(&self, path: &mut Path, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
