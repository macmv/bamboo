use super::{
  super::{Block, Data, Kind, Type},
  Behavior, BlockDrops, Drops, TileEntity,
};
use crate::{
  entity,
  event::EventFlow::{self, *},
  item::SharedInventory,
  player::{BlockClick, Player, Window},
  world::World,
};
use bb_common::{
  math::Pos,
  util::{Chat, Face},
};
use bb_transfer::{MessageRead, MessageWrite, MessageWriter};
use std::{any::Any, sync::Arc};

pub struct Log;
impl Behavior for Log {
  fn place<'a>(&self, data: &'a Data, _: Pos, click: BlockClick) -> Type<'a> {
    data.default_type().with(
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
  fn interact(&self, _: Block, player: &Arc<Player>) -> EventFlow {
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
    Handled
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
  fn place<'a>(&self, data: &'a Data, _: Pos, click: BlockClick) -> Type<'a> {
    data.default_type().with("part", "FOOT").with("facing", click.dir.as_horz_face().as_str())
  }
  fn update_place(&self, world: &Arc<World>, block: Block) {
    if block.ty.prop("part") == "FOOT" {
      let dir = Face::from(block.ty.prop("facing").as_enum());
      let _ = world.set_block(block.pos + dir, block.ty.with("part", "HEAD"));
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
#[derive(bb_macros::Transfer, Default, Debug, Clone)]
pub struct ChestTE {
  inv: SharedInventory<27>,
}
impl Behavior for Chest {
  fn create_te(&self) -> Option<Arc<dyn TileEntity>> {
    Some(Arc::new(ChestTE { inv: SharedInventory::new() }))
  }
  fn load_te(
    &self,
    r: &mut bb_transfer::MessageReader,
  ) -> Option<Result<Arc<dyn TileEntity>, bb_transfer::ReadError>> {
    Some(match ChestTE::read(r) {
      Ok(v) => Ok(Arc::new(v)),
      Err(e) => Err(e),
    })
  }
  fn interact(&self, block: Block, player: &Arc<Player>) -> EventFlow {
    block.te(|chest: &ChestTE| {
      player.show_inventory(
        Window::Generic9x3(crate::player::window::GenericWindow { inv: chest.inv.clone() }),
        &Chat::new("Chest"),
      );
      Handled
    })
  }
}
impl TileEntity for ChestTE {
  fn save(&self, w: &mut MessageWriter<&mut Vec<u8>>) -> Result<(), bb_transfer::WriteError> {
    self.write(w)
  }
  fn as_any(&self) -> &dyn Any { self }
}

pub struct Trapdoor;
impl Behavior for Trapdoor {
  fn place<'a>(&self, data: &'a Data, _: Pos, click: BlockClick) -> Type<'a> {
    data
      .default_type()
      .with("half", if click.cursor.y > 0.5 { "TOP" } else { "BOTTOM" })
      .with("facing", click.dir.as_horz_face().as_str())
  }
  fn interact(&self, mut block: Block, _: &Arc<Player>) -> EventFlow {
    block.set(block.ty.with("open", !block.ty.prop("open").bool()));
    Handled
  }
}

pub struct Door;
impl Behavior for Door {
  fn place<'a>(&self, data: &'a Data, _: Pos, click: BlockClick) -> Type<'a> {
    data.default_type().with("half", "LOWER").with("facing", click.dir.as_horz_face().as_str())
  }
  fn update_place(&self, world: &Arc<World>, block: Block) {
    if block.ty.prop("half") == "LOWER" {
      let _ = world.set_block(block.pos.add_y(1), block.ty.with("half", "UPPER"));
    }
  }
  fn interact(&self, mut block: Block, _: &Arc<Player>) -> EventFlow {
    let new_open = !block.ty.prop("open").bool();
    block.set(block.ty.with("open", new_open));
    let other = match block.ty.prop("half").str() {
      "UPPER" => block.pos.add_y(-1),
      "LOWER" => block.pos.add_y(1),
      v => unreachable!("door half {v}"),
    };
    let other_ty = block.world.get_block(other).unwrap();
    if other_ty.kind == block.ty.kind() {
      let _ = block.world.set_block(other, other_ty.with("open", new_open).ty());
    }
    Handled
  }
}
