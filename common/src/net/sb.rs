use num_derive::{FromPrimitive, ToPrimitive};
use std::{convert::TryInto, fmt};

use crate::{math::Pos, proto, proto::packet_field::Type as FieldType, util::UUID};

#[derive(Clone, Debug)]
pub struct Packet {
  id: ID,
  pb: proto::Packet,
}

// Is the ID enum
include!(concat!(env!("OUT_DIR"), "/protocol/sb.rs"));

/// A grpc packet ID. This is roughly the same as the latest packet version, but
/// in any order.
impl ID {
  /// Returns the id as an i32. Used when serializing protobufs.
  pub fn to_i32(self) -> i32 {
    num::ToPrimitive::to_i32(&self).unwrap()
  }
  /// Creates an id from an i32. Used when deserializing protobufs.
  pub fn from_i32(id: i32) -> Self {
    num::FromPrimitive::from_i32(id).unwrap()
  }
}

macro_rules! add_set {
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty) => {
    pub fn $name(&mut self, n: String, v: $ty) {
      self.pb.fields.insert(
        n,
        proto::PacketField { ty: FieldType::$ty_name.into(), $key: v, ..Default::default() },
      );
    }
  };
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&mut self, n: String, v: $ty) {
      self.pb.fields.insert(
        n,
        proto::PacketField {
          ty: FieldType::$ty_name.into(),
          $key: $convert(v),
          ..Default::default()
        },
      );
    }
  };
}

macro_rules! add_get {
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty) => {
    pub fn $name(&self, n: &str) -> $ty {
      let field = self.get_field(n, FieldType::$ty_name);
      field.$key
    }
  };
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&self, n: &str) -> $ty {
      let field = self.get_field(n, FieldType::$ty_name);
      $convert(field.$key)
    }
  };
}
macro_rules! add_get_ref {
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty) => {
    pub fn $name(&self, n: &str) -> $ty {
      let field = self.get_field(n, FieldType::$ty_name);
      &field.$key
    }
  };
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&self, n: &str) -> $ty {
      let field = self.get_field(n, FieldType::$ty_name);
      $convert(&field.$key)
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
  pub fn into_proto(mut self) -> proto::Packet {
    self.pb.id = self.id.to_i32();
    self.pb
  }
  pub fn id(&self) -> ID {
    self.id
  }
  add_set!(set_bool, bool, Bool, bool);
  add_set!(set_byte, byte, Byte, u8, |v: u8| v.into());
  add_set!(set_short, short, Short, i16, |v: i16| v.into());
  add_set!(set_int, int, Int, i32);
  add_set!(set_long, long, Long, u64);
  add_set!(set_float, float, Float, f32);
  add_set!(set_double, double, Double, f64);
  add_set!(set_str, str, Str, String);
  add_set!(set_uuid, uuid, Uuid, UUID, |v: UUID| { Some(v.as_proto()) });
  add_set!(set_pos, pos, Pos, Pos, |v: Pos| { v.to_u64() });
  add_set!(set_byte_arr, byte_arr, ByteArr, Vec<u8>);
  add_set!(set_int_arr, int_arr, IntArr, Vec<i32>);
  add_set!(set_long_arr, long_arr, LongArr, Vec<u64>);
  add_set!(set_str_arr, str_arr, StrArr, Vec<String>);

  fn get_field(&self, n: &str, ty: FieldType) -> &proto::PacketField {
    let field = match self.pb.fields.get(n) {
      Some(v) => v,
      None => panic!("while deserializing packet {}, got no value for key {}", self, n),
    };
    let got = proto::packet_field::Type::from_i32(field.ty).unwrap();
    if got != ty {
      panic!("expected {} to be a {:?}, got {:?}", n, ty, got);
    }
    field
  }
  fn get_field_any(&self, n: &str) -> &proto::PacketField {
    let field = match self.pb.fields.get(n) {
      Some(v) => v,
      None => panic!("while deserializing packet {}, got no value for key {}", self, n),
    };
    field
  }
  add_get!(get_bool, bool, Bool, bool);
  add_get!(get_byte, byte, Byte, u8, |v: u32| v.try_into().unwrap());
  add_get!(get_float, float, Float, f32);
  add_get!(get_double, double, Double, f64);
  add_get!(get_pos, pos, Pos, Pos, |v: u64| { Pos::from_u64(v) });
  add_get_ref!(get_uuid, uuid, Uuid, UUID, |v: &Option<proto::Uuid>| {
    UUID::from_proto(v.clone().unwrap())
  });
  add_get_ref!(get_str, str, Str, &str);
  add_get_ref!(get_byte_arr, byte_arr, ByteArr, &Vec<u8>);
  add_get_ref!(get_int_arr, int_arr, IntArr, &Vec<i32>);
  add_get_ref!(get_long_arr, long_arr, LongArr, &Vec<u64>);
  add_get_ref!(get_str_arr, str_arr, StrArr, &Vec<String>);

  /// Gets the given field within the packet. If the field is a byte, it will be
  /// casted to a short.
  pub fn get_short(&self, n: &str) -> i16 {
    // let field = self.get_field(n, FieldType::$ty_name);
    // $convert(field.$key)
    let field = self.get_field_any(n);
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Short => field.short.try_into().unwrap(),
      FieldType::Byte => field.byte.try_into().unwrap(),
      v => panic!("expected {} to be a short of byte, got {:?}", n, v),
    }
  }

  /// Gets the given field within the packet. If the field is a short or byte,
  /// it will be casted to an int.
  pub fn get_int(&self, n: &str) -> i32 {
    // let field = self.get_field(n, FieldType::$ty_name);
    // $convert(field.$key)
    let field = self.get_field_any(n);
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Int => field.int,
      FieldType::Short => field.short,
      FieldType::Byte => field.byte.try_into().unwrap(),
      v => panic!("expected {} to be a short of byte, got {:?}", n, v),
    }
  }

  /// Gets the given field within the packet. If the field is an int, short, or
  /// byte, it will be casted to a long.
  pub fn get_long(&self, n: &str) -> u64 {
    // let field = self.get_field(n, FieldType::$ty_name);
    // $convert(field.$key)
    let field = self.get_field_any(n);
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Long => field.long,
      FieldType::Int => field.int as u64,
      FieldType::Short => field.short as u64,
      FieldType::Byte => field.byte.into(),
      v => panic!("expected {} to be a short of byte, got {:?}", n, v),
    }
  }
}

macro_rules! value_non_empty {
  ($field: ident, $f: expr, $var: ident, $fmt: expr) => {
    writeln!($f, concat!("  {}: ", $fmt), $field.0, $field.1.$var)?;
  };
  ($field: ident, $f: expr, $var: ident, $fmt: expr, $extra: expr) => {
    writeln!($f, concat!("  {}: ", $fmt), $field.0, $field.1.$var, $extra)?;
  };
}

impl fmt::Display for Packet {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "Packet(")?;
    writeln!(f, "  id: {:?}", self.id)?;
    for v in self.pb.fields.iter() {
      match FieldType::from_i32(v.1.ty).unwrap() {
        FieldType::Bool => value_non_empty!(v, f, bool, "{:?}"),
        FieldType::Byte => value_non_empty!(v, f, byte, "{:?}"),
        FieldType::Short => value_non_empty!(v, f, short, "{:?}"),
        FieldType::Int => value_non_empty!(v, f, int, "{:?}"),
        FieldType::Long => value_non_empty!(v, f, long, "{:?}"),
        FieldType::Float => value_non_empty!(v, f, float, "{:?}"),
        FieldType::Double => value_non_empty!(v, f, double, "{:?}"),
        FieldType::Str => value_non_empty!(v, f, str, "{:?}"),
        FieldType::Uuid => value_non_empty!(v, f, uuid, "{:?}"),
        FieldType::Pos => value_non_empty!(v, f, pos, "{:?}"),
        FieldType::ByteArr => value_non_empty!(
          v,
          f,
          byte_arr,
          "{:?}\n  byte_arrs as strings: {:?}",
          String::from_utf8(v.1.byte_arr.clone())
        ),
        FieldType::IntArr => value_non_empty!(v, f, int_arr, "{:?}"),
        FieldType::LongArr => value_non_empty!(v, f, long_arr, "{:?}"),
        FieldType::StrArr => value_non_empty!(v, f, str_arr, "{:?}"),
      }
    }
    write!(f, ")")?;
    Ok(())
  }
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
// macro_rules! id_init_other {
//   ($($ty: ident: $num: expr),*) => {
//     proto::Packet {
//       $(
//         $ty: vec![Default::default(); $num],
//       )*
//       other: Some(Default::default()),
//       ..Default::default()
//     }
//   };
// }

#[rustfmt::skip]
fn create_empty(_id: ID) -> proto::Packet {
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
  // match id {
  //   ID::TeleportConfirm            => id_init!(ints: 1),
  //   ID::QueryBlockNBT              => id_init!(ints: 1, positions: 1),
  //   ID::SetDifficulty              => id_init!(bytes: 1),
  //   ID::ChatMessage                => id_init!(strs: 1),
  //   ID::ClientStatus               => id_init!(ints: 1),
  //   ID::ClientSettings             => id_init!(strs: 1, bytes: 2, ints: 2, bools: 1),
  //   ID::TabComplete                => id_init!(ints: 1, strs: 1),
  //   ID::WindowConfirmation         => id_init!(bytes: 1, shorts: 1, bools: 1),
  //   ID::ClickWindowButton          => id_init!(bytes: 2),
  //   ID::ClickWindow                => id_init!(bytes: 2, shorts: 2, ints: 1), // TODO: Setup slot type
  //   ID::CloseWindow                => id_init!(bytes: 1),
  //   ID::PluginMessage              => id_init!(strs: 1, byte_arrs: 1),
  //   ID::EditBook                   => id_init!(bools: 1, ints: 1), // TODO: Setup slot type
  //   ID::EntityNBTRequest           => id_init!(), // TODO: Figure out what this packet is
  //   ID::InteractEntity             => id_init!(ints: 3, floats: 3, bools: 1),
  //   ID::KeepAlive                  => id_init!(ints: 1), // Using a long here is dumb
  //   ID::LockDifficulty             => id_init!(bools: 1),
  //   ID::PlayerPosition             => id_init!(doubles: 3, bools: 1),
  //   ID::PlayerPositionAndRotation  => id_init!(doubles: 3, floats: 2, bools: 1),
  //   ID::PlayerRotation             => id_init!(floats: 2, bools: 1),
  //   ID::PlayerOnGround             => id_init!(bools: 1),
  //   ID::VehicleMove                => id_init!(doubles: 3, floats: 2),
  //   ID::SteerBoat                  => id_init!(bools: 2),
  //   ID::PickItem                   => id_init!(ints: 1),
  //   ID::CraftRecipeRequest         => id_init!(bytes: 1, strs: 1, bools: 1),
  //   ID::PlayerAbilities            => id_init!(bytes: 1),
  //   ID::PlayerDigging              => id_init!(ints: 1, positions: 1, bytes: 1),
  //   ID::EntityAction               => id_init!(ints: 3),
  //   ID::SteerVehicle               => id_init!(floats: 2, bytes: 1),
  //   ID::RecipeBookData             => id_init!(ints: 1, bools: 2),
  //   ID::NameItem                   => id_init!(strs: 1),
  //   ID::ResourcePackStatus         => id_init!(ints: 1),
  //   ID::AdvancementTab             => id_init!(ints: 1, strs: 1),
  //   ID::SelectTrade                => id_init!(ints: 1),
  //   ID::SetBeaconEffect            => id_init!(ints: 2),
  //   ID::HeldItemChange             => id_init!(shorts: 1),
  //   ID::UpdateCommandBlock         => id_init!(positions: 1, strs: 1, ints: 1, bytes: 1),
  //   ID::UpdateCommandBlockMinecart => id_init!(ints: 1, strs: 1, bools: 1),
  //   ID::CreativeInventoryAction    => id_init!(shorts: 1), // TODO: Setup slot type
  //   ID::UpdateJigsawBlock          => id_init!(positions: 1, strs: 5),
  //   ID::UpdateStructureBlock       => id_init!(positions: 1, ints: 4, strs: 2, bytes: 7, floats: 1, longs: 1),
  //   ID::UpdateSign                 => id_init!(positions: 1, strs: 4),
  //   ID::Animation                  => id_init!(ints: 1),
  //   ID::Spectate                   => id_init!(uuids: 1),
  //   ID::PlayerBlockPlace           => id_init!(ints: 2, positions: 1, floats: 3, bools: 1),
  //   ID::UseItem                    => id_init!(ints: 1),
  // }
  id_init!()
}
