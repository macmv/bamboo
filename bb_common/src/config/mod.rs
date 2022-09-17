use std::{fmt, fs, io};

pub use toml::*;

mod toml;
mod types;

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug)]
pub enum ConfigError {
  IO(std::io::Error),
  Parse(ParseError),
  Value(ValueError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValueError {
  pub path: Vec<String>,
  pub kind: ValueErrorKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueErrorKind {
  Missing,
  WrongType(String, Value),
  Other(String),
}

struct Path<'a>(&'a Vec<String>);

impl fmt::Display for Path<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "`")?;
    for (i, segment) in self.0.iter().enumerate() {
      if i != 0 {
        write!(f, "::")?;
      }
      write!(f, "{segment}")?;
    }
    write!(f, "`")
  }
}

impl fmt::Display for ValueError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.kind {
      ValueErrorKind::Missing => write!(f, "missing field {}", Path(&self.path)),
      ValueErrorKind::WrongType(expected, actual) => {
        write!(f, "expected {expected} at {}, got {actual}", Path(&self.path))
      }
      ValueErrorKind::Other(msg) => write!(f, "at {}, {msg}", Path(&self.path)),
    }
  }
}

impl fmt::Display for ConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self {
      Self::IO(e) => write!(f, "io error: {e}"),
      Self::Parse(e) => write!(f, "parse error: {e}"),
      Self::Value(e) => write!(f, "value error: {e}"),
    }
  }
}

impl From<io::Error> for ConfigError {
  fn from(v: io::Error) -> Self { ConfigError::IO(v) }
}
impl From<ParseError> for ConfigError {
  fn from(v: ParseError) -> Self { ConfigError::Parse(v) }
}
impl From<ValueError> for ConfigError {
  fn from(v: ValueError) -> Self { ConfigError::Value(v) }
}

impl ConfigError {
  pub fn new(kind: ValueErrorKind) -> Self { ValueError { path: vec![], kind }.into() }
  pub fn other(msg: impl std::fmt::Display) -> Self {
    ValueError { path: vec![], kind: ValueErrorKind::Other(msg.to_string()) }.into()
  }
  pub fn from_path<'a>(path: impl Iterator<Item = &'a str>, kind: ValueErrorKind) -> Self {
    ValueError { path: path.map(|s| s.to_string()).collect(), kind }.into()
  }
  pub fn from_value<T: TomlValue>(value: &Value) -> Self {
    Self::new(ValueErrorKind::WrongType(T::name(), value.clone()))
  }
  pub fn from_option<T: TomlValue>(value: &Value, opt: Option<T>) -> Result<T> {
    match opt {
      Some(v) => Ok(v),
      None => Err(Self::from_value::<T>(value)),
    }
  }
  pub fn prepend(mut self, element: impl Into<String>) -> Self {
    match &mut self {
      Self::Value(v) => v.path.insert(0, element.into()),
      _ => {}
    }
    self
  }
  pub fn prepend_list<'a>(mut self, path: impl Iterator<Item = &'a str>) -> Self {
    match &mut self {
      Self::Value(v) => {
        for (i, element) in path.enumerate() {
          v.path.insert(i, element.into());
        }
      }
      _ => {}
    }
    self
  }
}

pub trait TomlValue {
  /// If this current type matches the toml value, this returns Some(v).
  fn from_toml(v: &Value) -> Result<Self>
  where
    Self: Sized;

  /// Returns this struct as a toml value. This should include doc comments.
  fn to_toml(&self) -> Value;

  /// Returns the name of this toml value (string, integer, etc).
  fn name() -> String
  where
    Self: Sized;
}

/// A toml key. This is how a path to a toml value can be specified. This can be
/// represented as either an array or a string. If it is a string, it will be
/// split by dots into an array.
///
/// In order to index into maps, simply use a string name for a section. To
/// index into an array, use a number in the array. Example:
///
/// ```ignore
/// foo: bar
/// hello:
///   name: world
///   times: 1
/// items:
///   - 3
///   - 4
///   - a: 1
///     lot: 10
///     more: 100
///     things: 1000
/// ```
///
/// These are valid indices:
/// ```ignore
/// foo         // points to 'bar'
/// hello.name  // points to 'world'
/// items.0     // points to 3
/// items.2.lot // points to 10
/// ```
pub trait Key {
  /// Returns the sections of this key.
  fn sections(&self) -> Vec<&str>;
}

impl Key for str {
  fn sections(&self) -> Vec<&str> { self.split('.').collect() }
}
impl Key for [&str] {
  fn sections(&self) -> Vec<&str> { self.to_vec() }
}

/// Loads a config from the given path.
///
/// If the path does not exist, the default config will be returned. Use
/// [`new_write_default`] to write the default config to the path if it is not
/// present.
///
/// If the path does exist, and the config is valid, then the config will be
/// loaded and returned. Any missing fields will use the default value.
///
/// If the path does exist, and the config is invalid, then an error will be
/// logged, and the default config will be returned.
pub fn new_at_default<T: TomlValue + Default>(path: &str) -> T {
  let p = std::path::Path::new(path);
  if p.exists() {
    match new_at_err(path) {
      Ok(v) => return v,
      Err(e) => {
        error!("invalid config at `{path}`: {e}");
        std::process::exit(1);
      }
    }
  }
  T::default()
}

/// Loads the config from the given path, and writes the config if the file
/// doesn't exist.
pub fn new_at_write_default<T: TomlValue + Default>(path: &str) -> T {
  let p = std::path::Path::new(path);
  if p.exists() {
    match new_at_err(path) {
      Ok(v) => return v,
      Err(e) => {
        error!("invalid config at `{path}`: {e}");
        std::process::exit(1);
      }
    }
  }
  let config = T::default();
  write_to(&config, path);
  config
}

/// Loads the config from `path`, and writes the default to `default_path`.
pub fn new_at_write_default_to<T: TomlValue + Default>(path: &str, default_path: &str) -> T {
  write_to(&T::default(), default_path);

  new_at_default(path)
}

/// Writes the config to the given path.
pub fn write_to<T: TomlValue>(config: &T, path: &str) {
  match write_to_err(config, path) {
    Ok(()) => {}
    Err(e) => error!("could not write config to `{path}`: {e}"),
  }
}

pub fn write_to_err<T: TomlValue>(
  config: &T,
  path: &str,
) -> std::result::Result<(), std::io::Error> {
  let src = config.to_toml().to_toml();
  fs::write(path, src)
}

pub fn new_at_err<T: TomlValue>(path: &str) -> Result<T> {
  let src = fs::read_to_string(path).unwrap();
  new_err(&src)
}

pub fn new_err<T: TomlValue>(src: &str) -> Result<T> {
  let value = src.parse::<Value>()?;
  Ok(T::from_toml(&value)?)
}
