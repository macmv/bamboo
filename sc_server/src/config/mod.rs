use yaml_rust::{yaml::Yaml, YamlLoader};

pub struct Config {
  yaml:    YamlLoader,
  default: YamlLoader,
}

pub trait YamlValue {
  /// If this current type matches the yaml value, this returns Ok(v).
  /// Otherwise, this returns the original yaml object.
  fn from_yaml(yaml: Yaml) -> Result<Self, Yaml>
  where
    Self: Sized;

  /// Returns the name of this yaml value (string, integer, etc).
  fn name() -> &'static str
  where
    Self: Sized;
}

impl Config {
  pub fn get<T>(&self, key: &str) -> T
  where
    T: YamlValue,
  {
    let val = Yaml::Integer(0);
    match T::from_yaml(val) {
      Ok(v) => v,
      Err(val) => {
        warn!("invalid yaml value at `{}`: {:?}, expected a {}", key, val, T::name());
        self.get_default(key)
      }
    }
  }

  pub fn get_default<T>(&self, key: &str) -> T
  where
    T: YamlValue,
  {
    let val = Yaml::Integer(0);
    match T::from_yaml(val) {
      Ok(v) => v,
      Err(val) => {
        panic!("default had wrong type for key `{}`: {:?}, expected a {}", key, val, key);
      }
    }
  }
}
