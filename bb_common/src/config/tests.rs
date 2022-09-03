use super::Config;
use std::sync::Arc;

#[test]
fn parse_simple_values() {
  let config = Arc::new(Config::new_src(
    r#"
    foo = 3
    bar = 4

    [options]
    baz = 2
    other = 100
    "#,
    "",
  ));

  assert_eq!(config.get::<i32>("foo"), 3);
  assert_eq!(config.get::<i32>("bar"), 4);

  let section = config.section("options");
  assert_eq!(section.get::<i32>("baz"), 2);
  assert_eq!(section.get::<i32>("other"), 100);
}
