use super::{Map, Value};

#[test]
fn parsing() {
  let val: Value = "a = 3".parse().unwrap();

  let mut map = Map::new();
  map.insert("a".into(), Value::new(1, 3.into()));
  assert_eq!(val, Value::new_table(0, map));
}
