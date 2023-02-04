use super::{MyConfig, MyOptions};
use bb_common::config::{TomlValue, Value};
use bb_macros::{Config, Default};
use std::str::FromStr;

fn test_toml() -> Value {
  Value::from_str(
    r#"
    foo = 3
    bar = 4

    [options]
    baz = 2
    other = 100
    "#,
  )
  .unwrap()
}

#[test]
fn default_values() {
  assert_eq!(MyOptions::default(), MyOptions { baz: 3, other: 0 });
}

#[test]
fn parse_derived_values() {
  let config = MyConfig::from_toml(&test_toml()).unwrap();

  assert_eq!(config, MyConfig { foo: 3, bar: 4, options: MyOptions { baz: 2, other: 100 } });
}

#[derive(Default, Debug, PartialEq, Config)]
struct ColorConfig {
  color: Color,
}
#[derive(Default, Debug, PartialEq, Config)]
enum Color {
  #[default]
  Red,
  Green,
  Blue,
}

#[derive(Default, Debug, PartialEq, Config)]
struct OtherColorConfig {
  #[default(OtherColor::Green)]
  color: OtherColor,
}
#[derive(Debug, Default, PartialEq, Config)]
enum OtherColor {
  #[default]
  Red,
  Green,
  Blue,
}
#[test]
fn parse_derived_enum() {
  assert_eq!(
    bb_common::config::new_err::<ColorConfig>(
      r#"
      color = "green"
      "#,
    )
    .unwrap(),
    ColorConfig { color: Color::Green }
  );

  assert_eq!(
    bb_common::config::new_err::<ColorConfig>(
      r#"
      color = "invalid_color"
      "#,
    ).unwrap_err().to_string(),
    "value error: at `color`, got invalid option \"invalid_color\", valid options are \"red\", \"green\", or \"blue\"",
  );
}

#[test]
fn error_messages() {
  assert_eq!(
    bb_common::config::new_err::<MyOptions>(
      r#"
      baz = "hello"
      other = 10
      "#,
    )
    .unwrap_err()
    .to_string(),
    "value error: expected integer at `baz`, got \"hello\""
  );

  /*
  let config = Arc::new(Config::new_src(
    r#"
    [options]
    baz = "hello"
    other = 10
    "#,
    "",
  ));
  assert_eq!(
    config.get::<MyOptions>("options").unwrap_err().to_string(),
    "expected integer at `options::baz`, got \"hello\""
  );
  */
}

#[test]
fn default_struct_values() {
  let config = bb_common::config::new_err::<MyOptions>(
    r#"
    other = 10
    "#,
  )
  .unwrap();
  assert_eq!(config, MyOptions { baz: 3, other: 10 });
}

#[test]
fn new_functions() {
  let _: MyConfig = bb_common::config::new_err(
    r#"
    foo = 3
    bar = 5
    [options]
    baz = 2
    other = 100
    "#,
  )
  .unwrap();
}
