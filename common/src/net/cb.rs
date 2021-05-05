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
  // See https://wiki.vg/Protocol for more.
  // In general, I list out each field in the order that it is used in the packet.
  // If a packet has fields that look like this:
  //   - int
  //   - str
  //   - int
  // Then I would call `id_init!(ints: 2, strs: 1)`.
  //
  // For some packets, I just use a protobuf. See the comments on each packet for more.
  //
  // Lastly, there is conversion between versions of the game. For example, the keep alive
  // packet changed from an int to a long. In general, I will go with the newer version,
  // so that the latest client's features are entirely supported. However, a long for a keep
  // alive id is ridiculous, so I just used an int in that case. All of the conversion to
  // older packets is done on the proxy, so you should look at the implementation there for
  // specifics about how this is done.
  match id {
    ID::SpawnEntity               => id_init!(ints: 3, uuids: 1, doubles: 3, bytes: 2, shorts: 3),
    ID::SpawnExpOrb               => id_init!(ints: 1, doubles: 3, shorts: 1),
    ID::SpawnWeatherEntity        => id_init!(), // TODO: I think this packet was removed, idk
    ID::SpawnLivingEntity         => id_init!(ints: 2, uuids: 1, doubles: 3, bytes: 3, shorts: 3),
    ID::SpawnPainting             => id_init!(ints: 2, uuids: 1, positions: 1, bytes: 1),
    ID::SpawnPlayer               => id_init!(ints: 1, uuids: 1, doubles: 3, bytes: 2),
    ID::EntityAnimation           => id_init!(ints: 1, bytes: 1),
    ID::Statistics                => id_init!(int_arrs: 3),
    ID::AcknowledgePlayerDigging  => id_init!(positions: 1, ints: 2, bools: 1),
    ID::BlockBreakAnimation       => id_init!(ints: 1, positions: 1, bytes: 1),
    ID::BlockEntityData           => id_init!(positions: 1, bytes: 1, nbt_tags: 1),
    ID::BlockAction               => id_init!(positions: 1, bytes: 2, ints: 1),
    ID::BlockChange               => id_init!(positions: 1, ints: 1),
    ID::BossBar                   => id_init!(uuids: 1, ints: 1), // TODO: Custom type here
    ID::ServerDifficulty          => id_init!(bytes: 1, bools: 1),
    ID::ChatMessage               => id_init!(strs: 1, bytes: 1, uuids: 1),
    ID::MultiBlockChange          => id_init!(positions: 1, bools: 1, ints: 1, byte_arrs: 1),
    ID::TabComplete               => id_init!(ints: 3), // TODO: Custom type here
    ID::DeclareCommands           => id_init!(ints: 2, byte_arrs: 1),
    ID::WindowConfirm             => id_init!(bytes: 1, shorts: 1, bools: 1),
    ID::CloseWindow               => id_init!(bytes: 1),
    ID::WindowItems               => id_init!(bytes: 1, shorts: 1), // TODO: Setup slot type
    ID::WindowProperty            => id_init!(bytes: 1, shorts: 2),
    ID::SetSlot                   => id_init!(bytes: 1, shorts: 1), // TODO: Setup slot type
    ID::SetCooldown               => id_init!(ints: 2),
    ID::PluginMessage             => id_init!(strs: 1, byte_arrs: 1),
    // There is an extra int here, which should be used to set the sound id.
    // This is for backwards compatibility, as older clients do not use a sound by name.
    ID::NamedSoundEffect          => id_init!(strs: 1, ints: 5, floats: 2),
    ID::Disconnect                => id_init!(strs: 1),
    ID::EntityStatus              => id_init!(ints: 1, bytes: 1),
    ID::Explosion                 => id_init!(floats: 7, byte_arrs: 1),
    ID::UnloadChunk               => id_init!(ints: 2),
    ID::ChangeGameState           => id_init!(bytes: 1, floats: 1),
    ID::OpenHorseWindow           => id_init!(bytes: 1, ints: 2), // TODO: Find out more about this packet
    // Newer clients use a long, which is ridiculous. Just cast this on newer clients.
    ID::KeepAlive                 => id_init!(ints: 1),
    // This should be its own type. It changes so much that relying on int arrays is too difficult.
    ID::ChunkData                 => id_init!(), // TODO: Custom type here
    ID::Effect                    => id_init!(ints: 2, positions: 1, bools: 1),
    ID::Particle                  => id_init!(ints: 2, bools: 1, doubles: 3, floats: 4, byte_arrs: 1),
    // Only used on newer versions. For older clients, light data is sent with ChunkData.
    ID::UpdateLight               => id_init!(ints: 6, bools: 1, byte_arrs: 2),
    ID::JoinGame                  => id_init!(ints: 3, bools: 5, bytes: 1, str_arrs: 1, nbt_tags: 2, strs: 1, longs: 1),
    ID::MapData                   => id_init!(), // TODO: Custom proto
    ID::TradeList                 => id_init!(ints: 3, bools: 2), // TODO: Custom proto
    ID::EntityPosition            => id_init!(ints: 1, shorts: 3, bools: 1),
    ID::EntityPositionAndRotation => id_init!(ints: 1, shorts: 3, bytes: 2, bools: 1),
    ID::EntityRotation            => id_init!(ints: 1, bytes: 2, bools: 1),
    ID::EntityOnGround            => id_init!(ints: 1, bools: 1),
    ID::VehicleMove               => id_init!(doubles: 3, floats: 2),
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
