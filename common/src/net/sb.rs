use num_derive::{FromPrimitive, ToPrimitive};

use crate::{math::UUID, proto};

#[derive(Clone, Debug)]
pub struct Packet {
  id: ID,
  pb: proto::Packet,
}

#[derive(Clone, Copy, ToPrimitive, FromPrimitive, Debug, PartialEq, Eq, Hash)]
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

/// A grpc packet ID. This is roughly the same as the latest packet version, but
/// in any order.
impl ID {
  /// Returns the id as an i32. Used when serializing protobufs.
  pub fn to_i32(&self) -> i32 {
    num::ToPrimitive::to_i32(self).unwrap()
  }
  /// Creates an id from an i32. Used when deserializing protobufs.
  pub fn from_i32(id: i32) -> Self {
    num::FromPrimitive::from_i32(id).unwrap()
  }
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
    ID::TeleportConfirm            => id_init!(ints: 1),
    ID::QueryBlockNBT              => id_init!(ints: 1, positions: 1),
    ID::SetDifficulty              => id_init!(bytes: 1),
    ID::ChatMessage                => id_init!(strs: 1),
    ID::ClientStatus               => id_init!(ints: 1),
    ID::ClientSettings             => id_init!(strs: 1, bytes: 2, ints: 2, bools: 1),
    ID::TabComplete                => id_init!(ints: 1, strs: 1),
    ID::WindowConfirmation         => id_init!(bytes: 1, shorts: 1, bools: 1),
    ID::ClickWindowButton          => id_init!(bytes: 2),
    ID::ClickWindow                => id_init!(bytes: 2, shorts: 2, ints: 1), // TODO: Setup slot type
    ID::CloseWindow                => id_init!(bytes: 1),
    ID::PluginMessage              => id_init!(strs: 1, byte_arrs: 1),
    ID::EditBook                   => id_init!(bools: 1, ints: 1), // TODO: Setup slot type
    ID::EntityNBTRequest           => id_init!(), // TODO: Figure out what this packet is
    ID::InteractEntity             => id_init!(ints: 3, floats: 3, bools: 1),
    ID::KeepAlive                  => id_init!(ints: 1), // Using a long here is dumb
    ID::LockDifficulty             => id_init!(bools: 1),
    ID::PlayerPosition             => id_init!(doubles: 3, bools: 1),
    ID::PlayerPositionAndRotation  => id_init!(doubles: 3, floats: 2, bools: 1),
    ID::PlayerRotation             => id_init!(floats: 2, bools: 1),
    ID::PlayerOnGround             => id_init!(bools: 1),
    ID::VehicleMove                => id_init!(doubles: 3, floats: 2),
    ID::SteerBoat                  => id_init!(bools: 2),
    ID::PickItem                   => id_init!(ints: 1),
    ID::CraftRecipeRequest         => id_init!(bytes: 1, strs: 1, bools: 1),
    ID::PlayerAbilities            => id_init!(bytes: 1),
    ID::PlayerDigging              => id_init!(ints: 1, positions: 1, bytes: 1),
    ID::EntityAction               => id_init!(ints: 3),
    ID::SteerVehicle               => id_init!(floats: 2, bytes: 1),
    ID::RecipeBookData             => id_init!(ints: 1, bools: 2),
    ID::NameItem                   => id_init!(strs: 1),
    ID::ResourcePackStatus         => id_init!(ints: 1),
    ID::AdvancementTab             => id_init!(ints: 1, strs: 1),
    ID::SelectTrade                => id_init!(ints: 1),
    ID::SetBeaconEffect            => id_init!(ints: 2),
    ID::HeldItemChange             => id_init!(shorts: 1),
    ID::UpdateCommandBlock         => id_init!(positions: 1, strs: 1, ints: 1, bytes: 1),
    ID::UpdateCommandBlockMinecart => id_init!(ints: 1, strs: 1, bools: 1),
    ID::CreativeInventoryAction    => id_init!(shorts: 1), // TODO: Setup slot type
    ID::UpdateJigsawBlock          => id_init!(positions: 1, strs: 5),
    ID::UpdateStructureBlock       => id_init!(positions: 1, ints: 4, strs: 2, bytes: 7, floats: 1, longs: 1),
    ID::UpdateSign                 => id_init!(positions: 1, strs: 4),
    ID::Animation                  => id_init!(ints: 1),
    ID::Spectate                   => id_init!(uuids: 1),
    ID::PlayerBlockPlace           => id_init!(ints: 2, positions: 1, floats: 3, bools: 1),
    ID::UseItem                    => id_init!(ints: 1),
  }
}
