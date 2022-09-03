use super::Config;

#[test]
fn parse_simple_values() {
  let config = Config::new_src(
    r#"
    foo = 3
    bar = 4

    [options]
    baz = 2
    other = 100
    "#,
    "",
  );

  assert_eq!(config.get("foo"), 3);
  assert_eq!(config.get("bar"), 4);

  let section = config.section("options");
  assert_eq!(section.get("baz"), 2);
  assert_eq!(section.get("other"), 100);
}
