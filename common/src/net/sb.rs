use num_derive::FromPrimitive;

use crate::proto;

#[derive(Clone, Debug)]
pub struct Packet {
  id: ID,
  pb: proto::Packet,
}

#[derive(Clone, Copy, FromPrimitive, Debug, PartialEq, Eq, Hash)]
pub enum ID {
  TeleportConfirm,
  QueryBlockNBT,
  SetDifficulty,
  ChatMessage,
  ClientStatus,
  ClientSettings,
  TabComplete,
  WindowConfirmation,
  ClickWindowButton,
  ClickWindow,
  CloseWindow,
  PluginMessage,
  EditBook,
  EntityNBTRequest,
  InteractEntity,
  KeepAlive,
  LockDifficulty,
  PlayerPosition,
  PlayerPositionAndRotation,
  PlayerRotation,
  PlayerOnGround,
  VehicleMove,
  SteerBoat,
  PickItem,
  CraftRecipeRequest,
  PlayerAbilities,
  PlayerDigging,
  EntityAction,
  SteerVehicle,
  RecipeBookData,
  NameItem,
  ResourcePackStatus,
  AdvancementTab,
  SelectTrade,
  SetBeaconEffect,
  HeldItemChange,
  UpdateCommandBlock,
  UpdateCommandBlockMinecart,
  CreativeInventoryAction,
  UpdateJigsawBlock,
  UpdateStructureBlock,
  UpdateSign,
  Animation,
  Spectate,
  PlayerBlockPlace,
  UseItem,
}

impl Packet {
  pub fn from(pb: proto::Packet) -> Option<Self> {
    match num::FromPrimitive::from_i32(pb.id) {
      Some(id) => Some(Packet { id, pb }),
      None => None,
    }
  }
  pub fn into(self) -> proto::Packet {
    self.pb
  }
}
