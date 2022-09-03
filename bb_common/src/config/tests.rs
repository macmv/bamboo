use super::Config;
use bb_macros::Config;
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

#[test]
fn parse_simple_values() {
  let config = test_config();

  assert_eq!(config.get::<i32>("foo"), 3);
  assert_eq!(config.get::<i32>("bar"), 4);

  let section = config.section("options");
  assert_eq!(section.get::<i32>("baz"), 2);
  assert_eq!(section.get::<i32>("other"), 100);
}

#[derive(Config)]
struct MyConfig {
  pub foo:     i32,
  pub bar:     i32,
  pub options: MyOptions,
}
#[derive(Config)]
struct MyOptions {
  pub baz:   i32,
  pub other: i32,
}

#[test]
fn parse_derived_values() {
  let config = test_config();

  let config = config.all::<MyConfig>();

  assert_eq!(config.foo, 3);
  assert_eq!(config.bar, 4);
  assert_eq!(config.options.baz, 2);
  assert_eq!(config.options.other, 100);
}

#[derive(Debug, PartialEq, Config)]
enum Color {
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
  let color = config.get::<Color>("color");

  assert_eq!(color, Color::Green);
}
