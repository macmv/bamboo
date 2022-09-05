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
impl From<Array> for ValueInner {
  fn from(v: Array) -> Self { ValueInner::Array(v) }
}
impl From<Map> for ValueInner {
  fn from(v: Map) -> Self { ValueInner::Table(v) }
}

impl Value {
  pub fn new(line: usize, value: impl Into<ValueInner>) -> Self {
    Value { comments: vec![], line, value: value.into() }
  }
  pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
    self.comments.push(comment.into());
    self
  }

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

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  pub line: usize,
  pub kind: ParseErrorKind,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
  MissingValue,
  MissingKey,
  Expected(Vec<TokenKind>),
  UnexpectedEOF,
  UnexpectedEOL,
  UnexpectedToken(char),
  InvalidInteger(std::num::ParseIntError),
  InvalidFloat(std::num::ParseFloatError),
  DuplicateKey(String),
  Other(String),
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
      Self::MissingKey => write!(f, "missing map key"),
      Self::Expected(tok) => {
        if tok.is_empty() {
          panic!("cannot have empty missing list")
        } else if tok.len() == 1 {
          write!(f, "expected {}", tok[0])
        } else if tok.len() == 2 {
          write!(f, "expected {} or {}", tok[0], tok[1])
        } else {
          write!(f, "expected ")?;
          for (i, t) in tok.iter().enumerate() {
            write!(f, " {t}")?;
            if i != tok.len() - 1 {
              write!(f, ",")?;
            }
            if i == tok.len() - 2 {
              write!(f, " or")?;
            }
          }
          Ok(())
        }
      }
      Self::UnexpectedEOF => write!(f, "unexpected end of file"),
      Self::UnexpectedEOL => write!(f, "unexpected end of line"),
      Self::UnexpectedToken(c) => write!(f, "unexpected token `{c}`"),
      Self::InvalidInteger(e) => write!(f, "invalid integer: {e}"),
      Self::InvalidFloat(e) => write!(f, "invalid float: {e}"),
      Self::DuplicateKey(key) => write!(f, "duplicate key '{key}'"),
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

  /// Changed based on if we are within an array literal, or within a table.
  allow_newlines: bool,
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
  Dot,
  Comma,
  OpenArr,
  CloseArr,
  OpenBrace,
  CloseBrace,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
  Comment,
  Word,
  String,
  Integer,
  Float,
  Boolean,

  Eq,
  Dot,
  Comma,
  OpenArr,
  CloseArr,
  OpenBrace,
  CloseBrace,
}
impl fmt::Display for TokenKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Comment => write!(f, "comment"),
      Self::Word => write!(f, "word"),
      Self::String => write!(f, "string"),
      Self::Integer => write!(f, "integer"),
      Self::Float => write!(f, "float"),
      Self::Boolean => write!(f, "boolean"),

      Self::Eq => write!(f, "`=`"),
      Self::Dot => write!(f, "`.`"),
      Self::Comma => write!(f, "`,`"),
      Self::OpenArr => write!(f, "`[`"),
      Self::CloseArr => write!(f, "`]`"),
      Self::OpenBrace => write!(f, "`{{`"),
      Self::CloseBrace => write!(f, "`}}`"),
    }
  }
}
impl Token<'_> {
  pub fn kind(&self) -> TokenKind {
    match self {
      Self::Comment(_) => TokenKind::Comment,
      Self::Word(_) => TokenKind::Word,
      Self::String(_) => TokenKind::String,
      Self::Integer(_) => TokenKind::Integer,
      Self::Float(_) => TokenKind::Float,
      Self::Boolean(_) => TokenKind::Boolean,

      Self::Eq => TokenKind::Eq,
      Self::Dot => TokenKind::Dot,
      Self::Comma => TokenKind::Comma,
      Self::OpenArr => TokenKind::OpenArr,
      Self::CloseArr => TokenKind::CloseArr,
      Self::OpenBrace => TokenKind::OpenBrace,
      Self::CloseBrace => TokenKind::CloseBrace,
    }
  }
}
impl<'a> Tokenizer<'a> {
  pub fn new(s: &'a str) -> Self {
    Tokenizer { s, peeked: None, index: 0, line: 1, allow_newlines: false }
  }
  pub fn expect(&mut self, tok: TokenKind) -> Result<Token<'a>, ParseError> {
    let actual = self.next()?;
    if actual.kind() == tok {
      Ok(actual)
    } else {
      Err(self.err(ParseErrorKind::Expected(vec![tok])))
    }
  }
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
    let mut found_comment = false;
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
      if c == Some('\n') {
        self.line += 1;
      }
      match c {
        Some('#') if !found_comment => found_comment = true,
        Some('\n') if found_comment => {
          // First, remove the newline with self.index - 1. Then, trim whitespace from
          // before the #. Then trim the # with [1..]. Finally, trim whitespace on the
          // actual comment.
          return Ok(Token::Comment(self.s[start..self.index - 1].trim()[1..].trim()));
        }
        Some(_) if found_comment => continue,

        Some('"') if !found_string => found_string = true,
        Some('"') if found_string => {
          return Ok(Token::String(self.s[start + 1..self.index - 1].trim().into()))
        }
        Some(_) if found_string => continue,

        c if c.map(|c| !c.is_ascii_digit() && c != '.').unwrap_or(true) && found_number => {
          if let Some(c) = c {
            self.index -= 1;
            if c == '\n' {
              self.line -= 1;
            }
          }
          let text = self.s[start..self.index].trim();
          if found_float {
            return Ok(Token::Float(text.parse::<f64>().map_err(|e| self.err(e.into()))?));
          } else {
            return Ok(Token::Integer(text.parse::<i64>().map_err(|e| self.err(e.into()))?));
          }
        }
        c if c.map(|c| !c.is_ascii_alphabetic()).unwrap_or(true) && found_word => {
          if let Some(c) = c {
            self.index -= 1;
            if c == '\n' {
              self.line -= 1;
            }
          }
          let word = self.s[start..self.index].trim();
          match word {
            "true" => return Ok(Token::Boolean(true)),
            "false" => return Ok(Token::Boolean(false)),
            _ => return Ok(Token::Word(word)),
          }
        }

        Some(c) if c.is_ascii_digit() => found_number = true,
        Some('.') if found_number => found_float = true,

        Some(c) if c.is_ascii_alphabetic() => found_word = true,

        Some('=') => return Ok(Token::Eq),
        Some('.') => return Ok(Token::Dot),
        Some(',') => return Ok(Token::Comma),
        Some('[') => return Ok(Token::OpenArr),
        Some(']') => return Ok(Token::CloseArr),
        Some('{') => return Ok(Token::OpenBrace),
        Some('}') => return Ok(Token::CloseBrace),

        Some('\n') if !self.allow_newlines => {
          let mut err = self.err(ParseErrorKind::UnexpectedEOL);
          err.line -= 1;
          return Err(err);
        }
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
  // Parses a value after a `=`, and returns None if it finds an invalid token.
  fn parse_value_opt(&mut self) -> Result<Option<ValueInner>, ParseError> {
    self.allow_newlines = false;
    Ok(Some(match self.next()? {
      Token::String(s) => ValueInner::String(s),
      Token::Integer(v) => ValueInner::Integer(v),
      Token::Float(v) => ValueInner::Float(v),
      Token::Boolean(v) => ValueInner::Boolean(v),
      Token::OpenArr => {
        let mut values = vec![];
        loop {
          match self.next()? {
            Token::CloseArr => break,
            t => self.peeked = Some(t),
          }
          let value = self.parse_value()?;
          values.push(Value::new(self.line, value));
          match self.next()? {
            Token::Comma => {}
            Token::CloseArr => break,
            _ => {
              return Err(
                self.err(ParseErrorKind::Expected(vec![TokenKind::Comma, TokenKind::CloseArr])),
              )
            }
          }
        }
        ValueInner::Array(values)
      }
      Token::OpenBrace => {
        let mut values = Map::new();
        loop {
          match self.next()? {
            Token::CloseBrace => break,
            t => self.peeked = Some(t),
          }
          let key = match self.next()? {
            Token::Word(w) => w.to_string(),
            _ => return Err(self.err(ParseErrorKind::MissingKey)),
          };
          self.expect(TokenKind::Eq)?;
          let value = self.parse_value()?;
          values.insert(key, Value::new(self.line, value));
          match self.next()? {
            Token::Comma => {}
            Token::CloseBrace => break,
            _ => {
              return Err(
                self.err(ParseErrorKind::Expected(vec![TokenKind::Comma, TokenKind::CloseBrace])),
              )
            }
          }
        }
        ValueInner::Table(values)
      }
      t => {
        self.peeked = Some(t);
        return Ok(None);
      }
    }))
  }
  // Parses a value after an `=`
  fn parse_value(&mut self) -> Result<ValueInner, ParseError> {
    match self.parse_value_opt()? {
      Some(v) => Ok(v),
      None => Err(self.err(ParseErrorKind::MissingValue)),
    }
  }
  // Parses a path between braces, like `foo.bar.baz`.
  fn parse_path(&mut self) -> Result<Vec<String>, ParseError> {
    let first = match self.expect(TokenKind::Word)? {
      Token::Word(w) => w,
      _ => unreachable!(),
    };
    let mut segments = vec![first.into()];
    loop {
      match self.next()? {
        Token::Dot => {
          let segment = match self.expect(TokenKind::Word)? {
            Token::Word(w) => w,
            _ => unreachable!(),
          };
          segments.push(segment.into());
        }
        Token::CloseArr => return Ok(segments),
        _ => {
          return Err(self.err(ParseErrorKind::Expected(vec![TokenKind::Dot, TokenKind::CloseArr])))
        }
      }
    }
  }
  // Parses a list of key-value pairs, seperated by newlines
  fn parse_map(&mut self) -> Result<Map, ParseError> {
    self.allow_newlines = true;
    let mut comments = self.parse_comments()?;
    let mut path: Vec<String> = vec![];
    let mut map = Map::new();
    loop {
      match self.next_opt()? {
        Some(Token::Word(key)) => {
          self.expect(TokenKind::Eq)?;
          let value = dbg!(self.parse_value())?;
          let line = self.line;
          self.allow_newlines = true;
          let mut map = &mut map;
          for key in path.iter().map(|s| s.as_str()).chain([key].into_iter()) {
            if map.contains_key(key) {
              map = match &mut map.get_mut(key).unwrap().value {
                ValueInner::Table(t) => t,
                _ => return Err(self.err(ParseErrorKind::DuplicateKey(key.into()))),
              }
            } else {
              map.insert(key.to_string(), Value::new(line, Map::new()));
            }
          }
          map.insert(
            key.into(),
            Value {
              comments: std::mem::replace(&mut comments, self.parse_comments()?),
              line,
              value,
            },
          );
        }
        Some(Token::OpenArr) => {
          path = self.parse_path()?;
          let mut map = &mut map;
          for key in path.iter() {
            if map.contains_key(key) {
              map = match &mut map.get_mut(key).unwrap().value {
                ValueInner::Table(t) => t,
                _ => return Err(self.err(ParseErrorKind::DuplicateKey(key.into()))),
              }
            } else {
              map.insert(key.to_string(), Value::new(self.line, Map::new()));
            }
          }
        }
        Some(_) => {
          return Err(self.err(ParseErrorKind::Expected(vec![TokenKind::Word, TokenKind::OpenArr])))
        }
        None => return Ok(map),
      }
    }
  }
}

impl FromStr for Value {
  type Err = ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut tok = Tokenizer::new(s);

    Ok(Value::new(0, tok.parse_map()?))
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
