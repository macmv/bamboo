use std::{fmt, fs};
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
  fn name() -> &'static str
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
/// ```
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
/// ```
/// foo         // points to 'bar'
/// hello.name  // points to 'world'
/// items.0     // points to 3
/// items.2.lot // points to 10
/// ```
pub trait YamlKey {
  /// Returns the sections of this key.
  fn sections(&self) -> Vec<&str>;
}

impl YamlKey for &str {
  fn sections(&self) -> Vec<&str> {
    self.split('.').collect()
  }
}
impl YamlKey for String {
  fn sections(&self) -> Vec<&str> {
    self.split('.').collect()
  }
}
impl YamlKey for Vec<&str> {
  fn sections(&self) -> Vec<&str> {
    self.clone()
  }
}

impl Config {
  /// Creates a new config for the given path. The path is a runtime path to
  /// load the config file. The default path is a secondary path, which will
  /// also be loaded. This will never be written to, and will be used as a
  /// fallback if the key doesn't exist in the file.
  pub fn new(path: &str, default: &str) -> Self {
    let conf = Config { primary: Self::load_yaml(path), default: Self::load_yaml(default) };
    dbg!(&conf.primary);
    conf
  }
  /// Creates a new config file, but with source strings, instead of paths. This
  /// is used in the proxy, which stores its default config in the binary, and
  /// will run without any errors when the config isn't present.
  pub fn new_src(primary: &str, default: &str) -> Self {
    Config { primary: Self::load_yaml_src(primary), default: Self::load_yaml_src(default) }
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
  pub fn get<'a, K, T>(&'a self, key: &K) -> T
  where
    K: YamlKey,
    T: YamlValue<'a>,
  {
    let val = &self.primary["hello"];
    match T::from_yaml(val) {
      Some(v) => v,
      None => {
        warn!(
          "invalid yaml value at `{}`: {:?}, expected a {}",
          key.sections().join("."),
          val,
          T::name()
        );
        self.get_default(key)
      }
    }
  }

  /// Gets the default value at the given key. This will panic if the key does
  /// not exist, or if it was the wrong type.
  pub fn get_default<'a, K, T>(&'a self, key: &K) -> T
  where
    K: YamlKey,
    T: YamlValue<'a>,
  {
    let val = &self.default["hello"];
    match T::from_yaml(val) {
      Some(v) => v,
      None => {
        panic!(
          "default had wrong type for key `{}`: {:?}, expected a {}",
          key.sections().join("."),
          val,
          T::name(),
        );
      }
    }
  }
}

impl YamlValue<'_> for bool {
  fn from_yaml(v: &Yaml) -> Option<Self> {
    v.as_bool()
  }

  fn name() -> &'static str {
    "bool"
  }
}

macro_rules! yaml_array {
  ($name:expr, $($ty:ty),*) => {
    $(
      impl YamlValue<'_> for Vec<$ty> {
        fn from_yaml(v: &Yaml) -> Option<Self> {
          v.as_vec().and_then(|v| v.iter().map(|v| <$ty>::from_yaml(&v)).collect::<Option<Vec<$ty>>>())
        }

        fn name() -> &'static str {
          concat!("array of ", $name)
        }
      }
    )*
  };
}

macro_rules! yaml_number {
  ($name:expr, $($ty:ty),*) => {
    $(
      impl YamlValue<'_> for $ty {
        fn from_yaml(v: &Yaml) -> Option<Self> {
          v.as_i64().and_then(|v| v.try_into().ok())
        }

        fn name() -> &'static str {
          $name
        }
      }

      yaml_array!($name, $ty);
    )*
  };
}

yaml_number!("integer", u8, u16, u32, u64, i8, i16, i32, i64);
yaml_array!("float", f32, f64);
yaml_array!("string", String);

impl<'a> YamlValue<'a> for &'a str {
  fn from_yaml(v: &'a Yaml) -> Option<Self> {
    v.as_str()
  }

  fn name() -> &'static str {
    "string"
  }
}

impl YamlValue<'_> for String {
  fn from_yaml(v: &Yaml) -> Option<Self> {
    v.as_str().map(|v| v.into())
  }

  fn name() -> &'static str {
    "string"
  }
}

impl YamlValue<'_> for f32 {
  fn from_yaml(v: &Yaml) -> Option<Self> {
    v.as_f64().map(|v| v as f32)
  }

  fn name() -> &'static str {
    "float"
  }
}

impl YamlValue<'_> for f64 {
  fn from_yaml(v: &Yaml) -> Option<Self> {
    v.as_f64()
  }

  fn name() -> &'static str {
    "float"
  }
}

impl<'a> YamlValue<'a> for &'a Vec<Yaml> {
  fn from_yaml(v: &'a Yaml) -> Option<Self> {
    v.as_vec()
  }

  fn name() -> &'static str {
    "array"
  }
}

impl<'a> YamlValue<'a> for Vec<&'a str> {
  fn from_yaml(v: &'a Yaml) -> Option<Self> {
    v.as_vec().and_then(|v| v.iter().map(|v| <&str>::from_yaml(&v)).collect::<Option<Vec<&str>>>())
  }

  fn name() -> &'static str {
    "array of string"
  }
}
