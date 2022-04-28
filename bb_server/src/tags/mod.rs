use bb_common::net::cb;
use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/tag/tags.rs"));

pub struct Tags {
  categories: TagCategories,
  // TODO: Add custom tags here
}

impl Tags {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { Tags { categories: generate_tags() } }

  pub fn serialize(&self) -> cb::Packet {
    let mut tags = HashMap::new();
    tags.insert("minecraft:block".into(), self.categories.block.serialize());
    tags.insert("minecraft:item".into(), self.categories.item.serialize());
    tags.insert("minecraft:fluid".into(), self.categories.fluid.serialize());
    tags.insert("minecraft:entity_type".into(), self.categories.entity_type.serialize());
    tags.insert("minecraft:game_event".into(), self.categories.game_event.serialize());
    cb::Packet::Tags { categories: tags }
  }
}

impl TagCategory {
  fn serialize(&self) -> HashMap<String, Vec<String>> {
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

  fn expand_tag(&self, name: &str) -> Vec<String> {
    if name.starts_with('#') {
      for tag in self.tags {
        if name == tag.name {
          return tag.values.iter().copied().flat_map(|elem| self.expand_tag(elem)).collect();
        }
      }
      panic!();
    } else {
      vec![name.into()]
    }
  }
}
