use crate::block;
use bb_common::net::cb;
use std::{collections::HashMap, str::FromStr};

include!(concat!(env!("OUT_DIR"), "/tag/tags.rs"));

pub struct Tags {
  categories: TagCategories,
  // TODO: Add custom tags here
}

impl Tags {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { Tags { categories: generate_tags() } }

  pub fn serialize(&self) -> cb::Packet {
    cb::Packet::Tags {
      block:       self.categories.block.serialize(),
      item:        self.categories.item.serialize(),
      fluid:       self.categories.fluid.serialize(),
      entity_type: self.categories.entity_type.serialize(),
      game_event:  self.categories.game_event.serialize(),
    }
  }
}

impl TagCategory {
  fn serialize(&self) -> HashMap<String, Vec<i32>> {
    self
      .tags
      .iter()
      .map(|tag| {
        (
          tag.name.strip_prefix('#').unwrap().to_string(),
          tag.values.iter().copied().flat_map(|elem| self.expand_tag(elem)).collect(),
        )
      })
      .collect()
  }

  fn expand_tag(&self, name: &str) -> Vec<i32> {
    if name.starts_with('#') {
      for tag in self.tags {
        if name == tag.name {
          return tag.values.iter().copied().flat_map(|elem| self.expand_tag(elem)).collect();
        }
      }
      panic!();
    } else {
      match block::Kind::from_str(name) {
        Ok(id) => vec![id as i32],
        Err(_) => vec![],
      }
    }
  }
}
