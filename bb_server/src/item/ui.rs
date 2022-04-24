use super::{Inventory, Stack};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct UI {
  pattern: Vec<String>,
  items:   HashMap<char, Stack>,
}

#[derive(Debug, Clone, Error)]
pub enum UIError {
  #[error("no rows were passed in")]
  EmptyRows,
  #[error("row `{row}` was `{len}` characters long, not `{expected_len}` characters")]
  MismatchedRows { row: usize, len: usize, expected_len: usize },
  #[error("missing item for key `{0}`")]
  MissingItem(char),
}

impl UI {
  pub fn new(pattern: Vec<String>) -> Result<UI, UIError> {
    if pattern.is_empty() {
      return Err(UIError::EmptyRows);
    }
    // TODO: Support things like dropper UIs
    let expected_len = 9;
    for (i, row) in pattern.iter().enumerate() {
      let len = row.chars().count();
      if len != expected_len {
        return Err(UIError::MismatchedRows { row: i, len, expected_len });
      }
    }
    Ok(UI { pattern, items: HashMap::new() })
  }

  pub fn item(&mut self, key: char, stack: Stack) { self.items.insert(key, stack); }

  pub fn to_inventory(&self) -> Result<Inventory<27>, UIError> {
    let mut inv = Inventory::new();
    for (r, row) in self.pattern.iter().enumerate() {
      for (col, c) in row.chars().enumerate() {
        if c == ' ' {
          continue;
        }
        match self.items.get(&c) {
          Some(stack) => inv.set(r as u32 * 9 + col as u32, stack.clone()),
          None => return Err(UIError::MissingItem(c)),
        }
      }
    }
    Ok(inv)
  }
}
