use super::{
  super::{Block, Data, Kind, Type},
  Behavior, BlockDrops, Drops, TileEntity,
};
use crate::{
  entity,
  item::SharedInventory,
  player::{BlockClick, Player, Window},
  world::World,
};
use bb_common::{
  math::Pos,
  util::{Chat, Face},
};
use std::{any::Any, sync::Arc};

pub struct Log;
impl Behavior for Log {
  fn place(&self, data: &Data, _: Pos, click: BlockClick) -> Type {
    data.default_type().with_prop(
      "axis",
      match click.face {
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
  fn update(&self, world: &Arc<World>, block: Block, _: Block, _: Block) {
    if world.get_kind(block.pos.add_y(-1)) == Ok(Kind::Air) {
      let _ = world.set_kind(block.pos, Kind::Air);
      world.summon_data(entity::Type::FallingBlock, block.pos.center(), block.ty.id() as i32);
    }
  }
}

pub struct CraftingTable;
impl Behavior for CraftingTable {
  fn interact(&self, _: Block, player: &Arc<Player>) -> bool {
    let grid = SharedInventory::new();
    let output = SharedInventory::new();
    player.show_inventory(
      Window::Crafting(crate::player::window::CraftingWindow {
        grid,
        output,
        wm: player.world().world_manager().clone(),
      }),
      &Chat::new("Crafting Table"),
    );
    true
  }
}

pub struct Bed;
impl Bed {
  fn other_half(&self, block: Block) -> Pos {
    let face = Face::from(block.ty.prop("facing").as_enum());
    if block.ty.prop("part") == "FOOT" {
      block.pos + face
    } else {
      block.pos - face
    }
  }
}
impl Behavior for Bed {
  fn place(&self, data: &Data, _: Pos, click: BlockClick) -> Type {
    data
      .default_type()
      .with_prop("part", "FOOT")
      .with_prop("facing", click.dir.as_horz_face().as_str())
  }
  fn update_place(&self, world: &Arc<World>, block: Block) {
    if block.ty.prop("part") == "FOOT" {
      let dir = Face::from(block.ty.prop("facing").as_enum());
      let _ = world.set_block(block.pos + dir, block.ty.with_prop("part", "HEAD"));
    }
  }
  fn update(&self, world: &Arc<World>, block: Block, old: Block, new: Block) {
    if new.kind() == Kind::Air && old.kind() == block.kind() && self.other_half(block) == old.pos {
      let _ = world.set_kind(block.pos, Kind::Air);
    }
  }
  fn drops(&self, block: Block) -> BlockDrops {
    if block.ty.prop("part") == "FOOT" {
      BlockDrops::Normal
    } else {
      BlockDrops::Custom(Drops::empty())
    }
  }
}

pub struct Chest;
pub struct ChestTE {
  inv: SharedInventory<27>,
}
impl Behavior for Chest {
  fn create_te(&self) -> Option<Arc<dyn TileEntity>> {
    Some(Arc::new(ChestTE { inv: SharedInventory::new() }))
  }
  fn interact(&self, block: Block, player: &Arc<Player>) -> bool {
    block.te(|chest: &ChestTE| {
      player.show_inventory(
        Window::Generic9x3(crate::player::window::GenericWindow { inv: chest.inv.clone() }),
        &Chat::new("Chest"),
      );
      true
    })
  }
}
impl TileEntity for ChestTE {
  fn as_any(&self) -> &dyn Any { self }
}
