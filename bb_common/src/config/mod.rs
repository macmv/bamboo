use crate::util::GameMode;
use std::{borrow::Borrow, fs, str::FromStr};
use yaml_rust::{yaml::Yaml, YamlLoader};

pub struct Config {
  primary: Yaml,
  default: Yaml,
}

pub trait YamlValue<'a> {
  /// If this current type matches the yaml value, this returns Some(v).
  fn from_yaml(v: &'a Yaml) -> Option<Self>
  where
    Self: Sized;

  /// Returns the name of this yaml value (string, integer, etc).
  fn name() -> String
  where
    Self: Sized;
}

/// A yaml key. This is how a path to a yaml value can be specified. This can be
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
pub trait YamlKey {
  /// Returns the sections of this key.
  fn sections(&self) -> Vec<&str>;
}

impl YamlKey for str {
  fn sections(&self) -> Vec<&str> { self.split('.').collect() }
}
impl YamlKey for [&str] {
  fn sections(&self) -> Vec<&str> { self.to_vec() }
}

impl Config {
  /// Creates a new config for the given path. The path is a runtime path to
  /// load the config file. The default path is yaml source, which should be
  /// loaded with `include_str!`. The defaul is used whenever a key is not
  /// present in the main config. When this is created, a file at `default_path`
  /// will be created, and the default yaml source will be written there.
  /// This is for developers, so they can view the default config as a
  /// reference. If the file cannot be written, a warning will be printed.
  pub fn new(path: &str, default_path: &str, default_src: &str) -> Self {
    fs::write(default_path, default_src).unwrap_or_else(|e| {
      warn!("could not write default configuration to disk at `{}`: {}", default_path, e);
    });
    Config { primary: Self::load_yaml(path), default: Self::load_yaml_src(default_src) }
  }

  fn load_yaml(path: &str) -> Yaml {
    YamlLoader::load_from_str(&fs::read_to_string(path).unwrap_or_else(|e| {
      error!("error loading yaml at `{}`: {}", path, e);
      "".into()
    }))
    .unwrap_or_else(|e| {
      error!("error loading yaml at `{}`: {}", path, e);
      vec![]
    })
    .into_iter()
    .next()
    .unwrap_or(Yaml::Null)
  }
  fn load_yaml_src(src: &str) -> Yaml {
    YamlLoader::load_from_str(src)
      .unwrap_or_else(|e| {
        error!("error loading yaml: {}", e);
        vec![]
      })
      .into_iter()
      .next()
      .unwrap_or(Yaml::Null)
  }

  /// Reads the yaml value at the given key. This will always return a value. If
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
  pub fn get<'a, K: ?Sized, T>(&'a self, key: &K) -> T
  where
    K: YamlKey,
    T: YamlValue<'a>,
  {
    let sections = key.borrow().sections();
    let val = Self::get_val(&self.primary, &sections);
    match T::from_yaml(val) {
      Some(v) => v,
      None => {
        if val != &Yaml::BadValue {
          warn!(
            "unexpected value at `{}`: {:?}, expected a {}",
            sections.join("."),
            val,
            T::name()
          );
        }
        self.get_default(key)
      }
    }
  }

  /// Gets the default value at the given key. This will panic if the key does
  /// not exist, or if it was the wrong type.
  pub fn get_default<'a, K: ?Sized, T>(&'a self, key: &K) -> T
  where
    K: YamlKey,
    T: YamlValue<'a>,
  {
    let sections = key.borrow().sections();
    let val = Self::get_val(&self.default, &sections);
    match T::from_yaml(val) {
      Some(v) => v,
      None => {
        panic!(
          "default had wrong type for key `{}`: {:?}, expected a {}",
          sections.join("."),
          val,
          T::name(),
        );
      }
    }
  }

  fn get_val<'a>(yaml: &'a Yaml, sections: &[&str]) -> &'a Yaml {
    let mut val = yaml;
    for s in sections {
      match val {
        Yaml::Hash(map) => match map.get(&Yaml::String(s.to_string())) {
          Some(v) => val = v,
          None => return &Yaml::BadValue,
        },
        Yaml::Array(arr) => match s.parse::<usize>() {
          Ok(idx) => val = &arr[idx],
          Err(_) => return &Yaml::BadValue,
        },
        _ => return &Yaml::BadValue,
      }
    }
    val
  }
}

impl YamlValue<'_> for bool {
  fn from_yaml(v: &Yaml) -> Option<Self> { v.as_bool() }

  fn name() -> String { "bool".into() }
}
impl YamlValue<'_> for GameMode {
  fn from_yaml(v: &Yaml) -> Option<Self> { GameMode::from_str(v.as_str()?).ok() }

  fn name() -> String { "game mode".into() }
}

impl<'a, T> YamlValue<'a> for Vec<T>
where
  T: YamlValue<'a>,
{
  fn from_yaml(v: &'a Yaml) -> Option<Self> {
    v.as_vec().and_then(|v| v.iter().map(|v| T::from_yaml(v)).collect::<Option<Vec<T>>>())
  }

  fn name() -> String { format!("array of {}", T::name()) }
}

macro_rules! yaml_number {
  ($name:expr, $($ty:ty),*) => {
    $(
      impl YamlValue<'_> for $ty {
        fn from_yaml(v: &Yaml) -> Option<Self> {
          v.as_i64().and_then(|v| v.try_into().ok())
        }

        fn name() -> String {
          $name.into()
        }
      }
    )*
  };
}

yaml_number!("integer", u8, u16, u32, u64, i8, i16, i32, i64);

impl<'a> YamlValue<'a> for &'a str {
  fn from_yaml(v: &'a Yaml) -> Option<Self> { v.as_str() }

  fn name() -> String { "string".into() }
}

impl YamlValue<'_> for String {
  fn from_yaml(v: &Yaml) -> Option<Self> { v.as_str().map(|v| v.into()) }

  fn name() -> String { "string".into() }
}

impl YamlValue<'_> for f32 {
  fn from_yaml(v: &Yaml) -> Option<Self> { v.as_f64().map(|v| v as f32) }

  fn name() -> String { "float".into() }
}

impl YamlValue<'_> for f64 {
  fn from_yaml(v: &Yaml) -> Option<Self> { v.as_f64() }

  fn name() -> String { "float".into() }
}
