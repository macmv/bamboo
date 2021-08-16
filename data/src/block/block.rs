use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
  // In fixed data, this is block id << 4 | meta
  // In paletted data, this is a global state id
  id:    u32,
  // All properties for this state. Empty on fixed states.
  //
  // Since this is a single state, the first item is the property
  // name, and the second item is the property value.
  props: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
  // In fixed data, id is the block id << 4
  // In paletted data, this is the min state id
  id:            u32,
  // In fixed data, this is an array of all variations
  // In paletted data, this is an array of all states
  states:        Vec<State>,
  // Is 0 in fixed data
  // In paletted data, this is an index into states
  default_index: u32,
  // Always the full name of the block (for example, grass_block)
  name:          String,
}

impl Block {
  // pub fn prop_strs(&self) -> Vec<String> {
  //   if self.states.is_empty() {
  //     return vec![self.name.clone()];
  //   }
  //   self.states.iter().map(|s| s.prop_str(&self.name)).collect()
  // }
  pub fn new(name: String, id: u32, states: Vec<State>, default_index: u32) -> Self {
    Block { id, states, default_index, name }
  }
  pub fn id(&self) -> u32 {
    self.id
  }
  pub fn states(&self) -> &Vec<State> {
    &self.states
  }
  pub fn default_index(&self) -> u32 {
    self.default_index
  }
  pub fn name(&self) -> &str {
    &self.name
  }
}

impl State {
  pub fn new(id: u32, props: Vec<(String, String)>) -> Self {
    State { id, props }
  }
  pub fn prop_str(&self, block_name: &str) -> String {
    let mut out = format!("{}[", block_name);
    let mut sorted_properties: Vec<(String, String)> = self.props.clone();
    sorted_properties.sort();
    for (k, v) in sorted_properties {
      if !out.ends_with('[') {
        out += ",";
      }
      out += &k;
      out += "=";
      out += &v;
    }
    out + "]"
  }
  pub fn id(&self) -> u32 {
    self.id
  }
  pub fn props(&self) -> &Vec<(String, String)> {
    &self.props
  }
  pub fn matches(&self, props: &[(String, String)]) -> bool {
    let mut found_keys = HashSet::new();
    for (key, val) in &self.props {
      for (other_key, other_val) in props {
        if key == other_key {
          found_keys.insert(other_key);
          if val != other_val {
            return false;
          }
          break;
        }
      }
    }
    for (k, _) in props {
      if !found_keys.contains(k) {
        eprintln!("INVALID property key `{}`. valid keys are:", k);
        for (k, _) in &self.props {
          eprintln!("{}", k);
        }
        eprintln!();
        panic!();
      }
    }
    true
  }
}
