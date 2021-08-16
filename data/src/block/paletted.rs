// This handles loading all block versions 1.13 and up
use serde_derive::Deserialize;
use std::{collections::HashMap, io};

use super::{Block, BlockVersion, State};

#[derive(Default, Debug, Deserialize)]
struct JsonBlockState {
  name:       String,
  // One of 'int', 'bool', or 'enum'
  #[serde(alias = "type")]
  ty:         String,
  num_values: u32,
  // Only present if ty is 'enum'
  values:     Option<Vec<String>>,
}

#[derive(Default, Debug, Deserialize)]
struct JsonBlock {
  id:            u32,
  #[serde(alias = "displayName")]
  display_name:  String,
  name:          String,
  // If this is None or 0, then it is unbreakable
  hardness:      Option<f32>,
  #[serde(alias = "minStateId")]
  min_state_id:  u32,
  #[serde(alias = "maxStateId")]
  max_state_id:  u32,
  states:        Vec<JsonBlockState>,
  // Vec of item ids
  drops:         Vec<u32>,
  diggable:      bool,
  transparent:   bool,
  #[serde(alias = "filterLight")]
  filter_light:  u32,
  #[serde(alias = "emitLight")]
  emit_light:    u32,
  #[serde(alias = "boundingBox")]
  bounding_box:  String,
  #[serde(alias = "stackSize")]
  stack_size:    u32,
  #[serde(alias = "defaultState")]
  default_state: u32,
  resistance:    f32,

  material:      Option<String>,
  #[serde(alias = "harvestTools")]
  harvest_tools: Option<HashMap<String, bool>>,
}

pub(super) fn load_data(file: &str) -> io::Result<BlockVersion> {
  let data: Vec<JsonBlock> = serde_json::from_str(file)?;
  let mut ver = BlockVersion::new();
  for b in data {
    ver.add_block(Block::new(
      b.name.clone(),
      b.min_state_id,
      generate_states(&b),
      b.default_state - b.min_state_id,
    ));
  }
  Ok(ver)
}

fn generate_states(b: &JsonBlock) -> Vec<State> {
  if b.states.is_empty() {
    return vec![];
  }
  let mut indicies = vec![0; b.states.len()];
  let mut states = vec![];
  let mut i = 0;
  let mut finished = false;
  while !finished {
    let mut props = vec![];
    for (k, v) in indicies.iter().enumerate() {
      props.push((b.states[k].name.clone(), state_value(&b.states[k], *v)));
    }
    states.push(State::new(b.min_state_id + i as u32, props));
    i += 1;

    finished = true;
    // This iterates through indicies to crawl over all possible combinations of
    // states
    for (i, val) in indicies.iter_mut().enumerate() {
      *val += 1;
      if *val < b.states[i].num_values as usize {
        finished = false;
        break;
      }
      *val = 0;
    }
  }
  // Sanity check
  assert_eq!(states.len() as u32, b.max_state_id - b.min_state_id + 1);
  states
}

fn state_value(s: &JsonBlockState, index: usize) -> String {
  match s.ty.as_ref() {
    "bool" => match index {
      0 => "false".into(),
      1 => "true".into(),
      v => panic!("state index is invalid: bool should be within 0..2, but got: {}", v),
    },
    "int" => {
      if index >= s.num_values as usize {
        panic!(
          "state index is invalid: int should be within 0..{}, but got: {}",
          s.num_values, index
        )
      }
      index.to_string()
    }
    "enum" => match &s.values {
      Some(values) => match values.get(index) {
        Some(v) => v.clone(),
        None => panic!(
          "state index is invalid: enum should be within 0..{}, but got: {}",
          values.len(),
          index,
        ),
      },
      None => panic!("got enum, but no values"),
    },
    v => panic!("state type is invalid: {}", v),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_states() {
    let expected = vec![
      State::new(
        20,
        [("small", "false"), ("big", "0")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        21,
        [("small", "true"), ("big", "0")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        22,
        [("small", "false"), ("big", "1")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        23,
        [("small", "true"), ("big", "1")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        24,
        [("small", "false"), ("big", "2")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        25,
        [("small", "true"), ("big", "2")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        26,
        [("small", "false"), ("big", "3")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
      State::new(
        27,
        [("small", "true"), ("big", "3")]
          .iter()
          .map(|(key, val)| ((*key).into(), (*val).into()))
          .collect(),
      ),
    ];
    let got = generate_states(&JsonBlock {
      states: vec![
        JsonBlockState {
          ty: "bool".into(),
          name: "small".into(),
          num_values: 2,
          ..Default::default()
        },
        JsonBlockState {
          ty: "int".into(),
          name: "big".into(),
          num_values: 4,
          ..Default::default()
        },
      ],
      min_state_id: 20,
      max_state_id: 27,
      ..Default::default()
    });
    dbg!(&expected, &got);
    if expected.len() != got.len() {
      assert!(false, "different length arrays");
    }
    assert!(expected.iter().zip(got.iter()).all(|(a, b)| a == b), "different arrays");
  }

  #[test]
  fn test_state_value() {
    let bs = &JsonBlockState { ty: "int".into(), num_values: 3, ..Default::default() };
    assert_eq!(state_value(bs, 0), "0");
    assert_eq!(state_value(bs, 1), "1");
    assert_eq!(state_value(bs, 2), "2");

    let bs = &JsonBlockState { ty: "bool".into(), num_values: 2, ..Default::default() };
    assert_eq!(state_value(bs, 0), "false");
    assert_eq!(state_value(bs, 1), "true");

    let bs = &JsonBlockState {
      ty: "enum".into(),
      num_values: 4,
      values: Some(vec!["a".into(), "b".into(), "c".into(), "d".into()]),
      ..Default::default()
    };
    assert_eq!(state_value(bs, 0), "a");
    assert_eq!(state_value(bs, 1), "b");
    assert_eq!(state_value(bs, 2), "c");
    assert_eq!(state_value(bs, 3), "d");
  }

  #[test]
  #[should_panic]
  fn panic_int_state_value() {
    state_value(&JsonBlockState { ty: "int".into(), num_values: 3, ..Default::default() }, 3);
  }

  #[test]
  #[should_panic]
  fn panic_bool_state_value() {
    state_value(&JsonBlockState { ty: "bool".into(), num_values: 2, ..Default::default() }, 2);
  }

  #[test]
  #[should_panic]
  fn panic_enum_state_value() {
    state_value(
      &JsonBlockState {
        ty: "enum".into(),
        num_values: 2,
        values: Some(vec!["a".into(), "b".into()]),
        ..Default::default()
      },
      2,
    );
  }
}
