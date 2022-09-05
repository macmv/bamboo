use std::{fmt, fs, sync::Arc};

pub use toml::*;

#[cfg(test)]
mod tests;
mod toml;
mod types;

pub struct Config {
  primary: Value,
  default: Value,
}

pub struct ConfigSection {
  config: Arc<Config>,
  path:   Vec<String>,
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigError {
  pub path: Vec<String>,
  pub kind: ConfigErrorKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigErrorKind {
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

impl fmt::Display for ConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.kind {
      ConfigErrorKind::Missing => write!(f, "missing field {}", Path(&self.path)),
      ConfigErrorKind::WrongType(expected, actual) => {
        dbg!(&actual);
        write!(f, "expected {expected} at {}, got {actual}", Path(&self.path))
      }
      ConfigErrorKind::Other(msg) => write!(f, "at {}, {msg}", Path(&self.path)),
    }
  }
}

impl ConfigError {
  pub fn new(kind: ConfigErrorKind) -> Self { ConfigError { path: vec![], kind } }
  pub fn other(msg: String) -> Self {
    ConfigError { path: vec![], kind: ConfigErrorKind::Other(msg) }
  }
  pub fn from_path<'a>(path: impl Iterator<Item = &'a str>, kind: ConfigErrorKind) -> Self {
    ConfigError { path: path.map(|s| s.to_string()).collect(), kind }
  }
  pub fn from_value<T: TomlValue>(value: &Value) -> Self {
    Self::new(ConfigErrorKind::WrongType(T::name(), value.clone()))
  }
  pub fn from_option<T: TomlValue>(value: &Value, opt: Option<T>) -> Result<T> {
    match opt {
      Some(v) => Ok(v),
      None => Err(Self::from_value::<T>(value)),
    }
  }
  pub fn prepend(mut self, element: impl Into<String>) -> Self {
    self.path.insert(0, element.into());
    self
  }
  pub fn prepend_list<'a>(mut self, path: impl Iterator<Item = &'a str>) -> Self {
    for (i, element) in path.enumerate() {
      self.path.insert(i, element.into());
    }
    self
  }
}

pub trait TomlValue {
  /// If this current type matches the toml value, this returns Some(v).
  fn from_toml(v: &Value) -> Result<Self>
  where
    Self: Sized;

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

fn join_dot<'a, I: Iterator<Item = &'a str>>(key: I) -> String {
  {
    let mut s = String::new();
    for section in key {
      if !s.is_empty() {
        s.push('.');
      }
      s.push_str(section);
    }
    s
  }
}

impl Config {
  /// Creates a new config for the given source. The `src` is toml source, which
  /// will be parsed when this is called. The `default_src` is toml source,
  /// which should be loaded with `include_str!`. The default is used whenever
  /// a key is not present in the main config.
  ///
  /// If the path doesn't exist, the default config will be written there.
  pub fn new_src(src: &str, default_src: &str) -> Self {
    Config { primary: Self::load_toml_src(src), default: Self::load_toml_src(default_src) }
  }
  /// Creates a new config for the given path. The path is a runtime path to
  /// load the config file. The `default_src` is toml source, which should be
  /// loaded with `include_str!`. The default is used whenever a key is not
  /// present in the main config.
  ///
  /// If the path doesn't exist, the default config will be written there.
  pub fn new(path: &str, default_src: &str) -> Self {
    if !std::path::Path::new(path).exists() {
      fs::write(path, default_src).unwrap_or_else(|e| {
        error!("could not write default configuration to disk at `{}`: {}", path, e);
      });
    }
    Config { primary: Self::load_toml(path), default: Self::load_toml_src(default_src) }
  }
  /// When this is created, a file at `default_path` will be created, and the
  /// default toml source will be written there. This is for developers, so they
  /// can view the default config as a reference. If the file cannot be written,
  /// a warning will be printed.
  pub fn new_write_default(path: &str, default_path: &str, default_src: &str) -> Self {
    fs::write(default_path, default_src).unwrap_or_else(|e| {
      warn!("could not write default configuration to disk at `{}`: {}", default_path, e);
    });
    Config::new(path, default_src)
  }

  fn load_toml(path: &str) -> Value {
    let src = fs::read_to_string(path).unwrap_or_else(|e| {
      error!("error loading toml at `{path}`: {e}");
      "".into()
    });
    src.parse().unwrap_or_else(|e| {
      error!("error loading toml at `{path}`: {e}");
      Value::new(0, Map::new())
    })
  }
  fn load_toml_src(src: &str) -> Value {
    src.parse().unwrap_or_else(|e| {
      error!("error loading toml: {e}");
      Value::new(0, Map::new())
    })
  }

  /// Reads the entire config as the given type `T`.
  pub fn all<T>(&self) -> Result<T>
  where
    T: TomlValue,
  {
    T::from_toml(&self.primary)
  }
}
