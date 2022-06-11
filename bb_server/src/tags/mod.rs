use crate::{block, entity, item};
use bb_common::net::cb;
use std::{collections::HashMap, str::FromStr};

include!(concat!(env!("OUT_DIR"), "/tag/tags.rs"));

pub struct Tags {
  categories: TagCategories,
  // TODO: Add custom tags here
}

#[derive(Debug, Clone, Copy)]
enum TagKind {
  Block,
  Item,
  Fluid,
  Entity,
  GameEvent,
}

impl Tags {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { Tags { categories: generate_tags() } }

  pub fn serialize(&self) -> cb::Packet {
    cb::Packet::Tags {
      block:       self.categories.block.serialize(TagKind::Block),
      item:        self.categories.item.serialize(TagKind::Item),
      fluid:       self.categories.fluid.serialize(TagKind::Fluid),
      entity_type: self.categories.entity_type.serialize(TagKind::Entity),
      game_event:  self.categories.game_event.serialize(TagKind::GameEvent),
    }
  }
}

impl TagCategory {
  fn serialize(&self, kind: TagKind) -> HashMap<String, Vec<i32>> {
    self
      .tags
      .iter()
      .map(|tag| {
        (
          tag.name.strip_prefix('#').unwrap().to_string(),
          tag.values.iter().copied().flat_map(|elem| self.expand_tag(elem, kind)).collect(),
        )
      })
      .collect()
  }

  fn expand_tag(&self, name: &str, kind: TagKind) -> Vec<i32> {
    if name.starts_with('#') {
      for tag in self.tags {
        if name == tag.name {
          return tag.values.iter().copied().flat_map(|elem| self.expand_tag(elem, kind)).collect();
        }
      }
      panic!();
    } else {
      kind.expand_str(name)
    }
  }
}

impl TagKind {
  pub fn expand_str(&self, name: &str) -> Vec<i32> {
    match self {
      Self::Block => match block::Kind::from_str(name) {
        Ok(kind) => vec![kind.id() as i32],
        Err(_) => vec![],
      },
      Self::Item => match item::Type::from_str(name) {
        Ok(id) => vec![id as i32],
        Err(_) => vec![],
      },
      Self::Entity => match entity::Type::from_str(name) {
        Ok(id) => vec![id as i32],
        Err(_) => vec![],
      },
      Self::Fluid => match name {
        // fuild 0 is `empty`
        "water" => vec![1],
        "flowing_water" => vec![2],
        "lava" => vec![3],
        "flowing_lava" => vec![4],
        _ => panic!("invalid fluid {name}"),
      },
      Self::GameEvent => vec![],
    }
  }
}
