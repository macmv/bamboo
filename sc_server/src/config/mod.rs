use yaml_rust::{yaml::Yaml, YamlLoader};

pub struct Config {
  yaml:    Yaml,
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

impl Config {
  pub fn get<'a, T>(&'a self, key: &str) -> T
  where
    T: YamlValue<'a>,
  {
    let val = &self.yaml["hello"];
    match T::from_yaml(val) {
      Some(v) => v,
      None => {
        warn!("invalid yaml value at `{}`: {:?}, expected a {}", key, val, T::name());
        self.get_default(key)
      }
    }
  }

  pub fn get_default<'a, T>(&'a self, key: &str) -> T
  where
    T: YamlValue<'a>,
  {
    let val = &self.default["hello"];
    match T::from_yaml(val) {
      Some(v) => v,
      None => {
        panic!("default had wrong type for key `{}`: {:?}, expected a {}", key, val, key);
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

macro_rules! yaml_number {
  ($($ty:ty),*) => {
    $(
      impl YamlValue<'_> for $ty {
        fn from_yaml(v: &Yaml) -> Option<Self> {
          v.as_i64().and_then(|v| v.try_into().ok())
        }

        fn name() -> &'static str {
          stringify!($ty)
        }
      }

      impl YamlValue<'_> for Vec<$ty> {
        fn from_yaml(v: &Yaml) -> Option<Self> {
          v.as_vec().and_then(|v| v.iter().map(|v| <$ty>::from_yaml(&v)).collect::<Option<Vec<$ty>>>())
        }

        fn name() -> &'static str {
          concat!("array of ", stringify!($ty))
        }
      }
    )*
  };
}

yaml_number!(u8, u16, u32, u64, i8, i16, i32, i64);

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
