use std::{fs, sync::Arc};
use toml::{map::Map, Value};

mod types;

pub struct Config {
  primary: Value,
  default: Value,
}

pub struct ConfigSection {
  config: Arc<Config>,
  path:   Vec<String>,
}

pub trait TomlValue<'a> {
  /// If this current type matches the toml value, this returns Some(v).
  fn from_toml(v: &'a Value) -> Option<Self>
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
      Value::Table(Map::new())
    })
  }
  fn load_toml_src(src: &str) -> Value {
    src.parse().unwrap_or_else(|e| {
      error!("error loading toml: {e}");
      Value::Table(Map::new())
    })
  }

  /// Reads the toml value at the given key. This will always return a value. If
  /// the value doesn't exist in the primary config (or the value is the wrong
  /// type), then it will use the default config. If it doesn't exist there (or
  /// if it's the wrong type), this function will panic.
  ///
  /// See [`YamlKey`] for details on how that is parsed.
  ///
  /// In my opinion, a key should always exist when you try to load it. If there
  /// was a function like `get_opt`, which would only return a value when
  /// present, that would make it much more difficult for users to find out what
  /// that key was. All the keys that can be loaded should be present in the
  /// default config, so that it is easy for users to edit the config
  /// themselves.
  ///
  /// If you really need to get around this, you can implement YamlValue for
  /// your own type. I hightly recommend against this, as that will just cause
  /// confusion for your users. I will not be adding any more implementations
  /// than the ones present in this file.
  pub fn get<'a, T>(&'a self, key: &str) -> T
  where
    T: TomlValue<'a>,
  {
    self.get_at([key].into_iter())
  }

  pub fn get_at<'a, 'b, I, T>(&'a self, key: I) -> T
  where
    I: Iterator<Item = &'b str> + Clone,
    T: TomlValue<'a>,
  {
    match Self::get_val(&self.primary, key.clone()) {
      Some(val) => match T::from_toml(val) {
        Some(v) => v,
        None => {
          warn!(
            "unexpected value at `{}`: {:?}, expected a {}",
            join_dot(key.clone()),
            val,
            T::name()
          );
          self.get_default_at(key)
        }
      },
      None => self.get_default_at(key),
    }
  }

  /// Gets the default value at the given key. This will panic if the key does
  /// not exist, or if it was the wrong type.
  fn get_default_at<'a, 'b, I, T>(&'a self, key: I) -> T
  where
    I: Iterator<Item = &'b str> + Clone,
    T: TomlValue<'a>,
  {
    match Self::get_val(&self.default, key.clone()) {
      Some(val) => match T::from_toml(val) {
        Some(v) => v,
        None => {
          panic!(
            "default had wrong type for key `{}`: {:?}, expected a {}",
            join_dot(key),
            val,
            T::name(),
          );
        }
      },
      None => panic!("default does not have key `{}`", join_dot(key)),
    }
  }

  fn get_val<'a, 'b, I>(toml: &'a Value, key: I) -> Option<&'a Value>
  where
    I: Iterator<Item = &'b str>,
  {
    let mut val = toml;
    for s in key {
      match val {
        Value::Table(map) => match map.get(s) {
          Some(v) => val = v,
          None => return None,
        },
        Value::Array(arr) => match s.parse::<usize>() {
          Ok(idx) => val = &arr[idx],
          Err(_) => return None,
        },
        _ => return None,
      }
    }
    Some(val)
  }

  /// Returns a config section for the given key.
  pub fn section<K: ?Sized>(self: &Arc<Self>, key: &K) -> ConfigSection
  where
    K: Key,
  {
    ConfigSection {
      config: self.clone(),
      path:   key.sections().iter().map(|v| v.to_string()).collect(),
    }
  }
}

impl ConfigSection {
  /// Gets the config value at the given key, prefixed by this reference's path.
  pub fn get<'a, T>(&'a self, key: &str) -> T
  where
    T: TomlValue<'a>,
  {
    self.config.get_at(self.path.iter().map(String::as_str).chain([key]))
  }

  /// Returns a config section for the given key. This new key will be appended
  /// to the current section's key.
  pub fn section<K: ?Sized>(&self, key: &K) -> ConfigSection
  where
    K: Key,
  {
    ConfigSection {
      config: self.config.clone(),
      path:   {
        let mut path = self.path.clone();
        path.extend(key.sections().iter().map(|v| v.to_string()));
        path
      },
    }
  }
}
