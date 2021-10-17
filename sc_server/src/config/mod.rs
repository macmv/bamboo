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

macro_rules! yaml_number {
  ($ty:ty, $name:expr) => {
    impl YamlValue<'_> for $ty {
      fn from_yaml(v: &Yaml) -> Option<Self> {
        v.as_i64().and_then(|v| v.try_into().ok())
      }

      fn name() -> &'static str {
        $name
      }
    }
  };
}

yaml_number!(u8, "u8");
yaml_number!(u16, "u16");
yaml_number!(u32, "u32");
yaml_number!(u64, "u64");
yaml_number!(i8, "i8");
yaml_number!(i16, "i16");
yaml_number!(i32, "i32");
yaml_number!(i64, "i64");

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
