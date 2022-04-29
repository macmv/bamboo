use super::{
  super::{Block, Data, Kind, Type},
  Behavior,
};
use crate::{entity, item::Inventory, player::Player, world::World};
use bb_common::{
  math::Pos,
  util::{Chat, Face},
};
use std::sync::Arc;

pub struct Log;
impl Behavior for Log {
  fn place(&self, data: &Data, _: Pos, face: Face) -> Type {
    data.default_type().with_prop(
      "axis",
      match face {
        Face::West | Face::East => "X",
        Face::Top | Face::Bottom => "Y",
        Face::North | Face::South => "Z",
      },
    )
  }
}

pub struct Falling;
impl Behavior for Falling {
  fn update_place(&self, world: &Arc<World>, block: Block) {
    if let Ok(Kind::Air) = world.get_kind(block.pos.add_y(-1)) {
      let _ = world.set_kind(block.pos, Kind::Air);
      world.summon_data(entity::Type::FallingBlock, block.pos.center(), block.ty.id() as i32);
    }
  }
  fn update(&self, world: &Arc<World>, block: Block, _: Block, new: Block) {
    if new.pos.y < block.pos.y && new.kind() == Kind::Air {
      let _ = world.set_kind(block.pos, Kind::Air);
      world.summon_data(entity::Type::FallingBlock, block.pos.center(), block.ty.id() as i32);
    }
  }
}

pub struct CraftingTable;
impl Behavior for CraftingTable {
  fn interact(&self, _: Block, player: &Arc<Player>) -> bool {
    player.show_inventory(Inventory::new(), &Chat::new("Crafting Table"));
    true
  }
}

pub struct Bed;
impl Behavior for Bed {
  fn place(&self, data: &Data, _: Pos, face: Face) -> Type {
    data.default_type().with_prop("part", "foot")
  }
  fn update_place(&self, world: &Arc<World>, block: Block) {
    let dir = Face::North;
    world.set_block(block.pos + dir, block.ty.with_prop("part", "head"));
  }
}
