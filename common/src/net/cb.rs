use num_derive::{FromPrimitive, ToPrimitive};
use prost::{DecodeError, EncodeError, Message};
use prost_types::Any;

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
  ($name: ident, $arr: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&mut self, i: usize, v: $ty) {
      self.pb.$arr[i] = $convert(v);
    }
  };
}

macro_rules! create_type_url {
  ($ty: expr) => {
    stringify!(type.googleapis.com/google.rpc.$ty)
  }
}

macro_rules! build_any_decode {
  [$any: expr, $($val: ident),*] => {
    match $any.type_url.as_str() {
      $(
        create_type_url!($val) => {
          match proto::$val::decode($any.value.as_slice()) {
            Ok(msg) => Box::new(msg),
            Err(e) => panic!("error decoding any: {}", e),
          }
        },
      )*
      _ => panic!("unknown type {}", $any.type_url),
    }
  }
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
  add_fn!(set_str, strs, String);
  add_fn!(set_uuid, uuids, UUID, |v: UUID| { v.as_proto() });
  add_fn!(set_byte_arr, byte_arrs, Vec<u8>);
  add_fn!(set_i32_arr, int_arrs, Vec<i32>, |v: Vec<i32>| { proto::IntArray { ints: v } });
  add_fn!(set_u64_arr, long_arrs, Vec<u64>, |v: Vec<u64>| { proto::LongArray { longs: v } });
  add_fn!(set_str_arr, str_arrs, Vec<String>, |v: Vec<String>| { proto::StrArray { strs: v } });

  pub fn set_other<M>(&mut self, v: &M) -> Result<(), EncodeError>
  where
    M: prost::Message + Sized,
  {
    let mut b = bytes::BytesMut::new();
    v.encode(&mut b)?;
    // TODO: Pass names into this function or something
    let name = create_type_url!(v).into();
    dbg!(&name);
    let any = Any { type_url: name, value: b.to_vec() };

    if self.pb.other.is_none() {
      panic!("packet {:?} does not need an other!", self.id);
    }
    self.pb.other = Some(any);
    Ok(())
  }
  pub fn decode_other(&self) -> Box<dyn prost::Message> {
    build_any_decode![self.pb.other.clone().unwrap(), ChunkData, BossBar]
  }
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
    proto::Packet {
      $(
        $ty: vec![Default::default(); $num],
      )*
      ..Default::default()
    }
  };
}
macro_rules! id_init_other {
  ($($ty: ident: $num: expr),*) => {
    proto::Packet {
      $(
        $ty: vec![Default::default(); $num],
      )*
      other: Some(Default::default()),
      ..Default::default()
    }
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
  // Then I would call `id_init!(ints: 2, strs: 1)`. This ordering doesn't matter, it's just
  // nice to be consistent.
  //
  // As for the order of each individual int, they are always in the same order that the
  // latest version has declared. For example, if 1.15 has a packet call foobar:
  //   - username: str
  //   - sound: str
  // And 1.8 has the same packet, but in reverse order:
  //   - sound: str
  //   - username: str
  // Then this packet (in protobuf form) should always be represented as [username, sound].
  // It is up to the proxy to sway the values for older versions.
  //
  // For some packets, I just use a protobuf. See the comments on each packet for more.
  //
  // Lastly, there is conversion between versions of the game. For example, the keep alive
  // packet changed from an int to a long. In general, I will go with the newer version,
  // so that the latest client's features are entirely supported. However, a long for a keep
  // alive id is ridiculous, so I just used an int in that case. All of the conversion to
  // older packets is done on the proxy, so you should look at the implementation there for
  // specifics about how this is done.
  //
  // Optional fields (things like an int that only exists of a bool is true) will always be
  // included, as that is simplest. For custom protos, optional fields are specific to how
  // that one packet is implemented.
  //
  // Some ints that are the length of an array are usually excluded, as protobuf arrays know
  // their own length. Any fields that are never used by the client are mostly removed.
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
    ID::BossBar                   => id_init_other!(uuids: 1, ints: 1), // TODO: Custom type here
    ID::ServerDifficulty          => id_init!(bytes: 1, bools: 1),
    ID::ChatMessage               => id_init!(strs: 1, bytes: 1, uuids: 1),
    ID::MultiBlockChange          => id_init!(positions: 1, bools: 1, byte_arrs: 1),
    ID::TabComplete               => id_init_other!(ints: 3), // TODO: Custom type here
    ID::DeclareCommands           => id_init!(ints: 1, byte_arrs: 1),
    ID::WindowConfirm             => id_init!(bytes: 1, shorts: 1, bools: 1),
    ID::CloseWindow               => id_init!(bytes: 1),
    ID::WindowItems               => id_init_other!(bytes: 1, shorts: 1), // TODO: Setup slot type
    ID::WindowProperty            => id_init!(bytes: 1, shorts: 2),
    ID::SetSlot                   => id_init_other!(bytes: 1, shorts: 1),
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
    ID::ChunkData                 => id_init_other!(), // TODO: Custom type here
    ID::Effect                    => id_init!(ints: 2, positions: 1, bools: 1),
    ID::Particle                  => id_init!(ints: 2, bools: 1, doubles: 3, floats: 4, byte_arrs: 1),
    // Only used on newer versions. For older clients, light data is sent with ChunkData.
    ID::UpdateLight               => id_init!(ints: 6, bools: 1, byte_arrs: 2),
    ID::JoinGame                  => id_init!(ints: 3, bools: 5, bytes: 1, str_arrs: 1, nbt_tags: 2, strs: 1, longs: 1),
    ID::MapData                   => id_init_other!(), // TODO: Custom proto
    ID::TradeList                 => id_init_other!(), // TODO: Custom proto
    ID::EntityPosition            => id_init!(ints: 1, shorts: 3, bools: 1),
    ID::EntityPositionAndRotation => id_init!(ints: 1, shorts: 3, bytes: 2, bools: 1),
    ID::EntityRotation            => id_init!(ints: 1, bytes: 2, bools: 1),
    ID::EntityOnGround            => id_init!(ints: 1, bools: 1),
    ID::VehicleMove               => id_init!(doubles: 3, floats: 2),
    ID::OpenBook                  => id_init!(ints: 1),
    ID::OpenWindow                => id_init!(ints: 2, strs: 1),
    ID::OpenSignEditor            => id_init!(positions: 1),
    ID::CraftRecipeResponse       => id_init!(bytes: 1, strs: 1),
    ID::PlayerAbilities           => id_init!(bytes: 1, floats: 2),
    // Enter combat and End combat are ignored, so the event will always be 2: Entity Dead,
    // which will display the death screen.
    ID::EnterCombat               => id_init!(ints: 2, strs: 1),
    ID::PlayerInfo                => id_init_other!(), // TODO: Custom proto
    ID::FacePlayer                => id_init!(ints: 3, doubles: 3, bools: 1),
    ID::PlayerPositionAndLook     => id_init!(doubles: 3, floats: 2, bytes: 1, ints: 1),
    ID::UnlockRecipies            => id_init!(ints: 1, bools: 8, str_arrs: 2),
    ID::DestroyEntity             => id_init!(int_arrs: 1),
    ID::RemoveEntityEffect        => id_init!(ints: 1, bytes: 1),
    ID::ResourcePack              => id_init!(strs: 2),
    ID::Respawn                   => id_init!(nbt_tags: 1, strs: 1, longs: 1, bytes: 2, bools: 3),
    ID::EntityHeadLook            => id_init!(ints: 1, bytes: 1),
    // Empty string means that the string is not present (in this case).
    ID::SelectAdvancementTab      => id_init!(strs: 1),
    ID::WorldBorder               => id_init_other!(), // TODO: Custom proto
    ID::Camera                    => id_init!(ints: 1),
    ID::HeldItemChange            => id_init!(bytes: 1),
    ID::UpdateViewPosition        => id_init!(ints: 2),
    ID::UpdateViewDistance        => id_init!(ints: 1),
    ID::DisplayScoreboard         => id_init!(bytes: 1, strs: 1),
    ID::EntityMetadata            => id_init!(ints: 1, byte_arrs: 1),
    ID::AttachEntity              => id_init!(ints: 2),
    ID::EntityVelocity            => id_init!(ints: 1, shorts: 3),
    ID::EntityEquipment           => id_init_other!(ints: 1, bytes: 1),
    ID::SetExp                    => id_init!(floats: 1, ints: 2),
    ID::UpdateHealth              => id_init!(floats: 2, ints: 1),
    ID::ScoreboardObjective       => id_init!(strs: 2, bytes: 1, ints: 1),
    ID::SetPassengers             => id_init!(ints: 2, int_arrs: 1),
    ID::Teams                     => id_init_other!(), // TODO: Custom proto
    ID::UpdateScore               => id_init!(strs: 2, bytes: 1, ints: 1),
    ID::SpawnPosition             => id_init!(positions: 1),
    ID::TimeUpdate                => id_init!(longs: 2),
    ID::Title                     => id_init_other!(), // TODO: Custom proto
    // TODO: Sound ids change to a string in 1.17
    ID::EntitySoundEffect         => id_init!(ints: 3, floats: 2),
    ID::SoundEffect               => id_init!(ints: 5, floats: 2),
    ID::StopSound                 => id_init!(bytes: 1, ints: 1, strs: 1),
    ID::PlayerListHeader          => id_init!(strs: 2),
    ID::NBTQueryResponse          => id_init!(ints: 1, nbt_tags: 1),
    ID::CollectItem               => id_init!(ints: 3),
    ID::EntityTeleport            => id_init!(ints: 1, doubles: 3, bytes: 2, bools: 1),
    ID::Advancements              => id_init_other!(), // TODO: Custom proto
    ID::EntityProperties          => id_init_other!(), // TODO: Custom proto
    // TODO: Effect ids change to strings in 1.17
    ID::EntityEffect              => id_init!(ints: 2, bytes: 3),
    ID::DeclareRecipies           => id_init_other!(), // TODO: Custom proto
    ID::Tags                      => id_init_other!(), // TODO: Custom proto
    // ID::Login                     => id_init!(other: 1),
    _                             => proto::Packet::default(),
  }
}
