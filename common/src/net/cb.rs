use num_derive::{FromPrimitive, ToPrimitive};

use crate::{math::UUID, proto};

#[derive(Clone, Debug)]
pub struct Packet {
  id: ID,
  pb: proto::Packet,
}

macro_rules! add_fn {
  ($name: ident, $arr: ident, $ty: ty) => {
    pub fn $name(&mut self, i: usize, v: $ty) {
      self.pb.$arr[i] = v;
    }
  };
  ($name: ident, $arr: ident, $ty: ty, $convert: ident) => {
    pub fn $name(&mut self, i: usize, v: $ty) {
      self.pb.$arr[i] = v.$convert();
    }
  };
}

impl Packet {
  pub fn new(id: ID) -> Self {
    Packet { id, pb: create_empty(id) }
  }
  pub fn from_proto(pb: proto::Packet) -> Self {
    let id = ID::from_i32(pb.id);
    Packet { id, pb }
  }
  pub fn to_proto(mut self) -> proto::Packet {
    self.pb.id = self.id.to_i32();
    self.pb
  }
  pub fn id(&self) -> ID {
    self.id
  }
  pub fn pb(&self) -> &proto::Packet {
    &self.pb
  }
  add_fn!(set_bool, bools, bool);
  add_fn!(set_byte, bytes, u8);
  add_fn!(set_i32, ints, i32);
  add_fn!(set_u64, longs, u64);
  add_fn!(set_f32, floats, f32);
  add_fn!(set_f64, doubles, f64);
  add_fn!(set_str, strings, String);
  add_fn!(set_uuid, uuids, UUID, as_proto);
  add_fn!(set_byte_arr, byte_arrays, Vec<u8>);

  // repeated uint64 positions          = 13;
  // repeated IntArray intArrays        = 14;
  // repeated LongArray longArrays      = 15;
  // repeated StringArray stringArrays  = 16;
  // repeated google.protobuf.Any other = 17;
}

impl ID {
  pub fn to_i32(&self) -> i32 {
    num::ToPrimitive::to_i32(self).unwrap()
  }
  pub fn from_i32(id: i32) -> Self {
    num::FromPrimitive::from_i32(id).unwrap()
  }
}

#[derive(Clone, Copy, FromPrimitive, ToPrimitive, Debug, PartialEq, Eq, Hash)]
pub enum ID {
  SpawnEntity = 0,
  SpawnExpOrb,
  SpawnWeatherEntity,
  SpawnLivingEntity,
  SpawnPainting,
  SpawnPlayer,
  EntityAnimation,
  Statistics,
  AcknowledgePlayerDigging,
  BlockBreakAnimation,
  BlockEntityData,
  BlockAction,
  BlockChange,
  BossBar,
  ServerDifficulty,
  ChatMessage,
  MultiBlockChange,
  TabComplete,
  DeclareCommands,
  WindowConfirm,
  CloseWindow,
  WindowItems,
  WindowProperty,
  SetSlot,
  SetCooldown,
  PluginMessage,
  NamedSoundEffect,
  Disconnect,
  EntityStatus,
  Explosion,
  UnloadChunk,
  ChangeGameState,
  OpenHorseWindow,
  KeepAlive,
  ChunkData,
  Effect,
  Particle,
  UpdateLight,
  JoinGame,
  MapData,
  TradeList,
  EntityPosition,
  EntityPositionAndRotation,
  EntityRotation,
  EntityOnGround,
  VehicleMove,
  OpenBook,
  OpenWindow,
  OpenSignEditor,
  CraftRecipeResponse,
  PlayerAbilities,
  EnterCombat,
  PlayerInfo,
  FacePlayer,
  PlayerPositionAndLook,
  UnlockRecipies,
  DestroyEntity,
  RemoveEntityEffect,
  ResourcePack,
  Respawn,
  EntityHeadLook,
  SelectAdvancementTab,
  WorldBorder,
  Camera,
  HeldItemChange,
  UpdateViewPosition,
  UpdateViewDistance,
  DisplayScoreboard,
  EntityMetadata,
  AttachEntity,
  EntityVelocity,
  EntityEquipment,
  SetExp,
  UpdateHealth,
  ScoreboardObjective,
  SetPassengers,
  Teams,
  UpdateScore,
  SpawnPosition,
  TimeUpdate,
  Title,
  EntitySoundEffect,
  SoundEffect,
  StopSound,
  PlayerListHeader,
  NBTQueryResponse,
  CollectItem,
  EntityTeleport,
  Advancements,
  EntityProperties,
  EntityEffect,
  DeclareRecipies,
  Tags,
  // Custom packet; should be intercepted by the proxy,
  Login,
  None,
}

#[rustfmt::skip]
fn create_empty(id: ID) -> proto::Packet {
  match id {
    ID::SpawnEntity               => proto::Packet { ..Default::default() },
    ID::SpawnExpOrb               => proto::Packet { ..Default::default() },
    ID::SpawnWeatherEntity        => proto::Packet { ..Default::default() },
    ID::SpawnLivingEntity         => proto::Packet { ..Default::default() },
    ID::SpawnPainting             => proto::Packet { ..Default::default() },
    ID::SpawnPlayer               => proto::Packet { ..Default::default() },
    ID::EntityAnimation           => proto::Packet { ..Default::default() },
    ID::Statistics                => proto::Packet { ..Default::default() },
    ID::AcknowledgePlayerDigging  => proto::Packet { ..Default::default() },
    ID::BlockBreakAnimation       => proto::Packet { ..Default::default() },
    ID::BlockEntityData           => proto::Packet { ..Default::default() },
    ID::BlockAction               => proto::Packet { ..Default::default() },
    ID::BlockChange               => proto::Packet { ..Default::default() },
    ID::BossBar                   => proto::Packet { ..Default::default() },
    ID::ServerDifficulty          => proto::Packet { ..Default::default() },
    ID::ChatMessage               => proto::Packet { ..Default::default() },
    ID::MultiBlockChange          => proto::Packet { ..Default::default() },
    ID::TabComplete               => proto::Packet { ..Default::default() },
    ID::DeclareCommands           => proto::Packet { ..Default::default() },
    ID::WindowConfirm             => proto::Packet { ..Default::default() },
    ID::CloseWindow               => proto::Packet { ..Default::default() },
    ID::WindowItems               => proto::Packet { ..Default::default() },
    ID::WindowProperty            => proto::Packet { ..Default::default() },
    ID::SetSlot                   => proto::Packet { ..Default::default() },
    ID::SetCooldown               => proto::Packet { ..Default::default() },
    ID::PluginMessage             => proto::Packet { ..Default::default() },
    ID::NamedSoundEffect          => proto::Packet { ..Default::default() },
    ID::Disconnect                => proto::Packet { ..Default::default() },
    ID::EntityStatus              => proto::Packet { ..Default::default() },
    ID::Explosion                 => proto::Packet { ..Default::default() },
    ID::UnloadChunk               => proto::Packet { ..Default::default() },
    ID::ChangeGameState           => proto::Packet { ..Default::default() },
    ID::OpenHorseWindow           => proto::Packet { ..Default::default() },
    ID::KeepAlive                 => proto::Packet { ..Default::default() },
    ID::ChunkData                 => proto::Packet { ..Default::default() },
    ID::Effect                    => proto::Packet { ..Default::default() },
    ID::Particle                  => proto::Packet { ..Default::default() },
    ID::UpdateLight               => proto::Packet { ..Default::default() },
    ID::JoinGame                  => proto::Packet { ..Default::default() },
    ID::MapData                   => proto::Packet { ..Default::default() },
    ID::TradeList                 => proto::Packet { ..Default::default() },
    ID::EntityPosition            => proto::Packet { ..Default::default() },
    ID::EntityPositionAndRotation => proto::Packet { ..Default::default() },
    ID::EntityRotation            => proto::Packet { ..Default::default() },
    ID::EntityOnGround            => proto::Packet { ..Default::default() },
    ID::VehicleMove               => proto::Packet { ..Default::default() },
    ID::OpenBook                  => proto::Packet { ..Default::default() },
    ID::OpenWindow                => proto::Packet { ..Default::default() },
    ID::OpenSignEditor            => proto::Packet { ..Default::default() },
    ID::CraftRecipeResponse       => proto::Packet { ..Default::default() },
    ID::PlayerAbilities           => proto::Packet { ..Default::default() },
    ID::EnterCombat               => proto::Packet { ..Default::default() },
    ID::PlayerInfo                => proto::Packet { ..Default::default() },
    ID::FacePlayer                => proto::Packet { ..Default::default() },
    ID::PlayerPositionAndLook     => proto::Packet { ..Default::default() },
    ID::UnlockRecipies            => proto::Packet { ..Default::default() },
    ID::DestroyEntity             => proto::Packet { ..Default::default() },
    ID::RemoveEntityEffect        => proto::Packet { ..Default::default() },
    ID::ResourcePack              => proto::Packet { ..Default::default() },
    ID::Respawn                   => proto::Packet { ..Default::default() },
    ID::EntityHeadLook            => proto::Packet { ..Default::default() },
    ID::SelectAdvancementTab      => proto::Packet { ..Default::default() },
    ID::WorldBorder               => proto::Packet { ..Default::default() },
    ID::Camera                    => proto::Packet { ..Default::default() },
    ID::HeldItemChange            => proto::Packet { ..Default::default() },
    ID::UpdateViewPosition        => proto::Packet { ..Default::default() },
    ID::UpdateViewDistance        => proto::Packet { ..Default::default() },
    ID::DisplayScoreboard         => proto::Packet { ..Default::default() },
    ID::EntityMetadata            => proto::Packet { ..Default::default() },
    ID::AttachEntity              => proto::Packet { ..Default::default() },
    ID::EntityVelocity            => proto::Packet { ..Default::default() },
    ID::EntityEquipment           => proto::Packet { ..Default::default() },
    ID::SetExp                    => proto::Packet { ..Default::default() },
    ID::UpdateHealth              => proto::Packet { ..Default::default() },
    ID::ScoreboardObjective       => proto::Packet { ..Default::default() },
    ID::SetPassengers             => proto::Packet { ..Default::default() },
    ID::Teams                     => proto::Packet { ..Default::default() },
    ID::UpdateScore               => proto::Packet { ..Default::default() },
    ID::SpawnPosition             => proto::Packet { ..Default::default() },
    ID::TimeUpdate                => proto::Packet { ..Default::default() },
    ID::Title                     => proto::Packet { ..Default::default() },
    ID::EntitySoundEffect         => proto::Packet { ..Default::default() },
    ID::SoundEffect               => proto::Packet { ..Default::default() },
    ID::StopSound                 => proto::Packet { ..Default::default() },
    ID::PlayerListHeader          => proto::Packet { ..Default::default() },
    ID::NBTQueryResponse          => proto::Packet { ..Default::default() },
    ID::CollectItem               => proto::Packet { ..Default::default() },
    ID::EntityTeleport            => proto::Packet { ..Default::default() },
    ID::Advancements              => proto::Packet { ..Default::default() },
    ID::EntityProperties          => proto::Packet { ..Default::default() },
    ID::EntityEffect              => proto::Packet { ..Default::default() },
    ID::DeclareRecipies           => proto::Packet { ..Default::default() },
    ID::Tags                      => proto::Packet { ..Default::default() },
    ID::Login                     => proto::Packet { ..Default::default() },
    _                             => proto::Packet::default(),
  }
}
