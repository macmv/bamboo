use super::{Yaml, YamlValue};
use crate::{math::FPos, util::GameMode};
use std::str::FromStr;

impl YamlValue<'_> for bool {
  fn from_yaml(v: &Yaml) -> Option<Self> { v.as_bool() }

  fn name() -> String { "bool".into() }
}
impl YamlValue<'_> for GameMode {
  fn from_yaml(v: &Yaml) -> Option<Self> { GameMode::from_str(v.as_str()?).ok() }

  fn name() -> String { "game mode".into() }
}
impl YamlValue<'_> for FPos {
  fn from_yaml(v: &Yaml) -> Option<Self> {
    let mut sections = v.as_str()?.split(' ');
    let x = sections.next()?.parse().ok()?;
    let y = sections.next()?.parse().ok()?;
    let z = sections.next()?.parse().ok()?;
    if sections.next().is_some() {
      None
    } else {
      Some(FPos::new(x, y, z))
    }
  }

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
