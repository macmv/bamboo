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

macro_rules! id_init {
  ($($ty: ident: $num: expr),*) => {
    proto::Packet { $(
      $ty: vec![Default::default(); $num],
    )* ..Default::default() }
  };
}

#[rustfmt::skip]
fn create_empty(id: ID) -> proto::Packet {
  match id {
    ID::SpawnEntity               => id_init!(ints: 2, uuids: 1),
    ID::SpawnExpOrb               => id_init!(),
    ID::SpawnWeatherEntity        => id_init!(),
    ID::SpawnLivingEntity         => id_init!(),
    ID::SpawnPainting             => id_init!(),
    ID::SpawnPlayer               => id_init!(),
    ID::EntityAnimation           => id_init!(),
    ID::Statistics                => id_init!(),
    ID::AcknowledgePlayerDigging  => id_init!(),
    ID::BlockBreakAnimation       => id_init!(),
    ID::BlockEntityData           => id_init!(),
    ID::BlockAction               => id_init!(),
    ID::BlockChange               => id_init!(),
    ID::BossBar                   => id_init!(),
    ID::ServerDifficulty          => id_init!(),
    ID::ChatMessage               => id_init!(),
    ID::MultiBlockChange          => id_init!(),
    ID::TabComplete               => id_init!(),
    ID::DeclareCommands           => id_init!(),
    ID::WindowConfirm             => id_init!(),
    ID::CloseWindow               => id_init!(),
    ID::WindowItems               => id_init!(),
    ID::WindowProperty            => id_init!(),
    ID::SetSlot                   => id_init!(),
    ID::SetCooldown               => id_init!(),
    ID::PluginMessage             => id_init!(),
    ID::NamedSoundEffect          => id_init!(),
    ID::Disconnect                => id_init!(),
    ID::EntityStatus              => id_init!(),
    ID::Explosion                 => id_init!(),
    ID::UnloadChunk               => id_init!(),
    ID::ChangeGameState           => id_init!(),
    ID::OpenHorseWindow           => id_init!(),
    ID::KeepAlive                 => id_init!(ints: 1),
    ID::ChunkData                 => id_init!(),
    ID::Effect                    => id_init!(),
    ID::Particle                  => id_init!(),
    ID::UpdateLight               => id_init!(),
    ID::JoinGame                  => id_init!(),
    ID::MapData                   => id_init!(),
    ID::TradeList                 => id_init!(),
    ID::EntityPosition            => id_init!(),
    ID::EntityPositionAndRotation => id_init!(),
    ID::EntityRotation            => id_init!(),
    ID::EntityOnGround            => id_init!(),
    ID::VehicleMove               => id_init!(),
    ID::OpenBook                  => id_init!(),
    ID::OpenWindow                => id_init!(),
    ID::OpenSignEditor            => id_init!(),
    ID::CraftRecipeResponse       => id_init!(),
    ID::PlayerAbilities           => id_init!(),
    ID::EnterCombat               => id_init!(),
    ID::PlayerInfo                => id_init!(),
    ID::FacePlayer                => id_init!(),
    ID::PlayerPositionAndLook     => id_init!(),
    ID::UnlockRecipies            => id_init!(),
    ID::DestroyEntity             => id_init!(),
    ID::RemoveEntityEffect        => id_init!(),
    ID::ResourcePack              => id_init!(),
    ID::Respawn                   => id_init!(),
    ID::EntityHeadLook            => id_init!(),
    ID::SelectAdvancementTab      => id_init!(),
    ID::WorldBorder               => id_init!(),
    ID::Camera                    => id_init!(),
    ID::HeldItemChange            => id_init!(),
    ID::UpdateViewPosition        => id_init!(),
    ID::UpdateViewDistance        => id_init!(),
    ID::DisplayScoreboard         => id_init!(),
    ID::EntityMetadata            => id_init!(),
    ID::AttachEntity              => id_init!(),
    ID::EntityVelocity            => id_init!(),
    ID::EntityEquipment           => id_init!(),
    ID::SetExp                    => id_init!(),
    ID::UpdateHealth              => id_init!(),
    ID::ScoreboardObjective       => id_init!(),
    ID::SetPassengers             => id_init!(),
    ID::Teams                     => id_init!(),
    ID::UpdateScore               => id_init!(),
    ID::SpawnPosition             => id_init!(),
    ID::TimeUpdate                => id_init!(),
    ID::Title                     => id_init!(),
    ID::EntitySoundEffect         => id_init!(),
    ID::SoundEffect               => id_init!(),
    ID::StopSound                 => id_init!(),
    ID::PlayerListHeader          => id_init!(),
    ID::NBTQueryResponse          => id_init!(),
    ID::CollectItem               => id_init!(),
    ID::EntityTeleport            => id_init!(),
    ID::Advancements              => id_init!(),
    ID::EntityProperties          => id_init!(),
    ID::EntityEffect              => id_init!(),
    ID::DeclareRecipies           => id_init!(),
    ID::Tags                      => id_init!(),
    ID::Login                     => id_init!(),
    _                             => proto::Packet::default(),
  }
}
