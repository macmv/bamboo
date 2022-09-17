use bb_macros::{Config, Default};

mod read;
mod write;

#[derive(Debug, Default, Clone, Config, PartialEq)]
struct MyConfig {
  pub foo:     i32,
  pub bar:     i32,
  pub options: MyOptions,
}
#[derive(Debug, Default, Clone, Config, PartialEq)]
struct MyOptions {
  #[default(3)]
  pub baz:   i32,
  pub other: i32,
}
