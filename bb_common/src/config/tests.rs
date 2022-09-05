use super::{Config, ConfigError};
use bb_macros::{Config, Default};
use std::sync::Arc;

fn test_config() -> Arc<Config> {
  Arc::new(Config::new_src(
    r#"
    foo = 3
    bar = 4

    [options]
    baz = 2
    other = 100
    "#,
    "",
  ))
}

#[derive(Debug, Default, Clone, Config, PartialEq)]
struct MyConfig {
  pub foo:     i32,
  pub bar:     i32,
  pub options: MyOptions,
}
#[derive(Debug, Default, Clone, Config, PartialEq)]
struct MyOptions {
  #[default = 3]
  pub baz:   i32,
  pub other: i32,
}

#[test]
fn default_values() {
  assert_eq!(MyOptions::default(), MyOptions { baz: 3, other: 0 });
}

#[test]
fn parse_derived_values() {
  let config = test_config();

  let config = config.all::<MyConfig>().unwrap();

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
  #[default = OtherColor::Green]
  color: OtherColor,
}
#[derive(Debug, PartialEq, Config)]
enum OtherColor {
  Red,
  Green,
  Blue,
}
#[test]
fn parse_derived_enum() {
  let config = Arc::new(Config::new_src(
    r#"
    color = "green"
    "#,
    "",
  ));
  assert_eq!(config.all::<ColorConfig>().unwrap(), ColorConfig { color: Color::Green });

  let config = Arc::new(Config::new_src(
    r#"
    color = "invalid_color"
    "#,
    "",
  ));
  assert_eq!(
    config.all::<ColorConfig>().unwrap_err().to_string(), 
    "at `color`, got invalid option \"invalid_color\", valid options are \"red\", \"green\", or \"blue\"",
  );
}

#[test]
fn error_messages() {
  let config = Arc::new(Config::new_src(
    r#"
    baz = "hello"
    other = 10
    "#,
    "",
  ));
  assert_eq!(
    config.all::<MyOptions>().unwrap_err().to_string(),
    "expected integer at `baz`, got \"hello\""
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
  let config = Arc::new(Config::new_src(
    r#"
    other = 10
    "#,
    "",
  ));
  assert_eq!(config.all::<MyOptions>().unwrap(), MyOptions { baz: 3, other: 10 });
}
