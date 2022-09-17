use bb_common::config::TomlValue;
use bb_macros::{Config, Default};

#[derive(Debug, Default, Clone, Config, PartialEq)]
struct MyConfig {
  /// This is what "foo" does.
  /// Here's another line of text with some weirdness: \n \0 \' \" \`
  foo: MyEnum,
}

#[derive(Debug, Default, Clone, Config, PartialEq)]
enum MyEnum {
  #[default]
  A,
  B,
  C,
}

#[test]
fn write_default() {
  assert_eq!(
    MyConfig::default().to_toml().to_toml(),
    r#"
    # This is what "foo" does.
    # Here's another line of text with some weirdness: \n \0 \' \" \`
    foo = "a"
    "#
    .lines()
    .skip(1)
    .map(|line| line.trim())
    .collect::<Vec<&str>>()
    .join("\n"),
  );
}
