use super::TomlValue;
use crate::{math::FPos, util::GameMode};
use log::{Level, LevelFilter};
use std::str::FromStr;
use toml::Value;

impl TomlValue for bool {
  fn from_toml(v: &Value) -> Option<Self> { v.as_bool() }

  fn name() -> String { "bool".into() }
}
impl TomlValue for GameMode {
  fn from_toml(v: &Value) -> Option<Self> { GameMode::from_str(v.as_str()?).ok() }

  fn name() -> String { "game mode".into() }
}
impl TomlValue for FPos {
  fn from_toml(v: &Value) -> Option<Self> {
    let map = v.as_table()?;
    if map.len() == 3 {
      Some(FPos::new(
        map.get("x")?.as_float()?,
        map.get("y")?.as_float()?,
        map.get("z")?.as_float()?,
      ))
    } else {
      None
    }
  }
  fn name() -> String { "position".into() }
}
impl TomlValue for Level {
  fn from_toml(v: &Value) -> Option<Self> { Level::from_str(v.as_str()?).ok() }
  fn name() -> String { "log level".into() }
}
impl TomlValue for LevelFilter {
  fn from_toml(v: &Value) -> Option<Self> { LevelFilter::from_str(v.as_str()?).ok() }
  fn name() -> String { "log level filter".into() }
}

impl<T> TomlValue for Vec<T>
where
  T: TomlValue,
{
  fn from_toml(v: &Value) -> Option<Self> {
    v.as_array().and_then(|v| v.iter().map(|v| T::from_toml(v)).collect::<Option<Vec<T>>>())
  }

  fn name() -> String { format!("array of {}", T::name()) }
}

macro_rules! toml_number {
  ($name:expr, $($ty:ty),*) => {
    $(
      impl TomlValue for $ty {
        fn from_toml(v: &Value) -> Option<Self> {
          v.as_integer().and_then(|v| v.try_into().ok())
        }

        fn name() -> String {
          $name.into()
        }
      }
    )*
  };
}

toml_number!("integer", u8, u16, u32, u64, i8, i16, i32, i64);

impl TomlValue for String {
  fn from_toml(v: &Value) -> Option<Self> { v.as_str().map(|v| v.into()) }

  fn name() -> String { "string".into() }
}

impl TomlValue for f32 {
  fn from_toml(v: &Value) -> Option<Self> { v.as_float().map(|v| v as f32) }

  fn name() -> String { "float".into() }
}

impl TomlValue for f64 {
  fn from_toml(v: &Value) -> Option<Self> { v.as_float() }

  fn name() -> String { "float".into() }
}
