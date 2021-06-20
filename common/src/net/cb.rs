use super::other::Other;
use crate::{math::Pos, proto, proto::packet_field::Type as FieldType, util::UUID};
use num_derive::{FromPrimitive, ToPrimitive};
use prost::{DecodeError, EncodeError};
use std::{convert::TryInto, fmt, io};

#[derive(Clone, Debug)]
pub struct Packet {
  id: ID,
  pb: proto::Packet,
}

macro_rules! add_set {
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty) => {
    pub fn $name(&mut self, n: &str, v: $ty) {
      self.pb.fields.insert(
        n.into(),
        proto::PacketField { ty: FieldType::$ty_name.into(), $key: v, ..Default::default() },
      );
    }
  };
  ($name: ident, $key: ident, $ty_name: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&mut self, n: &str, v: $ty) {
      self.pb.fields.insert(
        n.into(),
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
  ($name: ident, $ty_name: ident, $key: ident, $ty: ty) => {
    pub fn $name(&self, n: &str) -> io::Result<$ty> {
      let field = self.get_field(n, FieldType::$ty_name)?;
      Ok(field.$key)
    }
  };
  ($name: ident, $ty_name: ident, $key: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&self, n: &str) -> io::Result<$ty> {
      let field = self.get_field(n, FieldType::$ty_name)?;
      Ok($convert(field.$key))
    }
  };
}
macro_rules! add_get_ref {
  ($name: ident, $ty_name: ident, $key: ident, $ty: ty) => {
    pub fn $name(&self, n: &str) -> io::Result<$ty> {
      let field = self.get_field(n, FieldType::$ty_name)?;
      Ok(&field.$key)
    }
  };
  ($name: ident, $ty_name: ident, $key: ident, $ty: ty, $convert: expr) => {
    pub fn $name(&self, n: &str) -> io::Result<$ty> {
      let field = self.get_field(n, FieldType::$ty_name)?;
      Ok($convert(&field.$key))
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
  /// Returns the protobuf stored in this packet. If this packet was created
  /// through [`new`](Self::new), then the id will always be 0. If it was
  /// created with [`from_proto`](Self::from_proto), then this will just
  /// return that proto.
  ///
  /// This should only be used when reading a clientbound packet. If you want to
  /// generate a clientbound packet, use the set_* functions.
  ///
  /// In practice, this is only used in the proxy, when converting a grpc packet
  /// to a tcp packet.
  pub fn pb(&self) -> &proto::Packet {
    &self.pb
  }
  pub fn id(&self) -> ID {
    self.id
  }
  fn get_field(&self, n: &str, ty: FieldType) -> io::Result<&proto::PacketField> {
    let field = match self.pb.fields.get(n) {
      Some(v) => v,
      None => {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("no value for key {}", n)))
      }
    };
    let got = proto::packet_field::Type::from_i32(field.ty).unwrap();
    if got != ty {
      return Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {} to be a {:?}, got {:?}", n, ty, got),
      ));
    }
    Ok(field)
  }
  fn get_field_any(&self, n: &str) -> io::Result<&proto::PacketField> {
    let field = match self.pb.fields.get(n) {
      Some(v) => v,
      None => {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("no value for key {}", n)))
      }
    };
    Ok(field)
  }
  add_set!(set_bool, bool, Bool, bool);
  add_set!(set_byte, byte, Byte, u8, |v: u8| v.into());
  add_set!(set_int, int, Int, i32);
  add_set!(set_short, short, Short, i16, |v: i16| v.into());
  add_set!(set_long, long, Long, u64);
  add_set!(set_float, float, Float, f32);
  add_set!(set_double, double, Double, f64);
  add_set!(set_str, str, Str, String);
  add_set!(set_pos, pos, Pos, Pos, |v: Pos| v.to_u64()); // This will always be in the new position format
  add_set!(set_uuid, uuid, Uuid, UUID, |v: UUID| { Some(v.as_proto()) });
  add_set!(set_byte_arr, byte_arr, ByteArr, Vec<u8>);
  add_set!(set_int_arr, int_arr, IntArr, Vec<i32>);
  add_set!(set_long_arr, long_arr, LongArr, Vec<u64>);
  add_set!(set_str_arr, str_arr, StrArr, Vec<String>);

  add_get!(get_bool, Bool, bool, bool);
  add_get!(get_float, Float, float, f32);
  add_get!(get_double, Double, double, f64);
  add_get!(get_pos, Pos, pos, Pos, |v: u64| Pos::from_u64(v)); // This will always be in the new position format
  add_get_ref!(get_str, Str, str, &str);
  add_get_ref!(get_uuid, Uuid, uuid, UUID, |v: &Option<proto::Uuid>| UUID::from_proto(
    v.clone().unwrap()
  ));
  add_get_ref!(get_byte_arr, ByteArr, byte_arr, &Vec<u8>);
  add_get_ref!(get_i32_arr, IntArr, int_arr, &Vec<i32>);
  add_get_ref!(get_u64_arr, LongArr, long_arr, &Vec<u64>);
  add_get_ref!(get_str_arr, StrArr, str_arr, &Vec<String>);

  /// Gets the given field within the packet. If the field is a float, it will
  /// be casted to a byte (it will be multiplied by 256).
  pub fn get_byte(&self, n: &str) -> io::Result<u8> {
    let field = self.get_field_any(n)?;
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Byte => Ok(field.byte.try_into().unwrap()),
      FieldType::Float => Ok((field.float * 256.0) as u8),
      v => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {} to be a byte or float, got {:?}", n, v),
      )),
    }
  }

  /// Gets the given field within the packet. If the field is a byte, it will be
  /// casted to a short.
  pub fn get_short(&self, n: &str) -> io::Result<i16> {
    let field = self.get_field_any(n)?;
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Short => Ok(field.short.try_into().unwrap()),
      FieldType::Byte => Ok(field.byte.try_into().unwrap()),
      v => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {} to be a short or byte, got {:?}", n, v),
      )),
    }
  }

  /// Gets the given field within the packet. If the field is a byte or a short,
  /// it will be casted to an int. If the field is a float or double, it will be
  /// cast to a fixed in (for 1.8), which just means multiplying it by 32 and
  /// then returning that casted to an int.
  pub fn get_int(&self, n: &str) -> io::Result<i32> {
    let field = self.get_field_any(n)?;
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Int => Ok(field.int),
      FieldType::Short => Ok(field.short),
      FieldType::Byte => Ok(field.byte.try_into().unwrap()),
      // The field is a float in other versions, but is listed as an int in 1.8. This most
      // likely means a fixed int, which is 1.8's method of representing floats. Since clientbound
      // packets don't know the client version, we assume that a fixed int is possible here.
      FieldType::Float => Ok((field.float * 32.0) as i32),
      FieldType::Double => Ok((field.double * 32.0) as i32),
      v => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {} to be an int, short, byte, float, or double, got {:?}", n, v),
      )),
    }
  }

  /// Gets the given field within the packet. If the field is an int, short, or
  /// a byte, it will be casted to a long.
  pub fn get_long(&self, n: &str) -> io::Result<u64> {
    let field = self.get_field_any(n)?;
    match proto::packet_field::Type::from_i32(field.ty).unwrap() {
      FieldType::Long => Ok(field.long),
      FieldType::Int => Ok(field.int.try_into().unwrap()),
      FieldType::Short => Ok(field.short.try_into().unwrap()),
      FieldType::Byte => Ok(field.byte.try_into().unwrap()),
      v => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {} to be a long, int, short, or byte, got {:?}", n, v),
      )),
    }
  }

  /// Generates an any type from the given value, and embeds that into the
  /// protbuf.
  pub fn set_other(&mut self, v: Other) -> Result<(), EncodeError> {
    // if self.pb.other.is_none() {
    //   panic!("packet {:?} does not need an other!", self.id);
    // }
    self.pb.other = Some(v.to_any()?);
    Ok(())
  }

  /// Reads the 'other' field of this protobuf. This is an any type, so the
  /// message returned can be any type.
  pub fn read_other(&self) -> Result<Other, DecodeError> {
    Other::from_any(self.pb.other.clone().unwrap())
  }
}
impl fmt::Display for Packet {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "Packet(")?;
    writeln!(f, "  id: {:?}", self.id)?;
    for (n, v) in self.pb.fields.iter() {
      match FieldType::from_i32(v.ty).unwrap() {
        FieldType::Bool => writeln!(f, "  {}: {}", n, v.bool),
        FieldType::Byte => writeln!(f, "  {}: {} (byte)", n, v.byte),
        FieldType::Short => writeln!(f, "  {}: {} (short)", n, v.short),
        FieldType::Int => writeln!(f, "  {}: {} (int)", n, v.int),
        FieldType::Long => writeln!(f, "  {}: {} (long)", n, v.long),
        FieldType::Float => writeln!(f, "  {}: {} (float)", n, v.float),
        FieldType::Double => writeln!(f, "  {}: {} (double)", n, v.double),
        FieldType::Str => writeln!(f, "  {}: \"{}\"", n, v.str),
        FieldType::Uuid => writeln!(f, "  {}: {:?}", n, v.uuid),
        FieldType::Pos => writeln!(f, "  {}: {}", n, v.pos),
        FieldType::ByteArr => writeln!(
          f,
          "  {}: {:?}\n  byte_arr as string: {:?}",
          n,
          v.byte_arr,
          String::from_utf8(v.byte_arr.clone())
        ),
        FieldType::IntArr => writeln!(f, "  {}: {:?} (int arr)", n, v.int_arr),
        FieldType::LongArr => writeln!(f, "  {}: {:?} (long arr)", n, v.long_arr),
        FieldType::StrArr => writeln!(f, "  {}: {:?}", n, v.str_arr),
      }?;
    }
    writeln!(f, ")")?;
    Ok(())
  }
}

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

// Is the ID enum
include!(concat!(env!("OUT_DIR"), "/protocol/cb.rs"));

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
  //   ID::SpawnEntity               => id_init!(ints: 3, uuids: 1, doubles: 3, bytes: 2, shorts: 3),
  //   ID::SpawnExpOrb               => id_init!(ints: 1, doubles: 3, shorts: 1),
  //   ID::SpawnWeatherEntity        => id_init!(), // TODO: I think this packet was removed, idk
  //   ID::SpawnLivingEntity         => id_init!(ints: 2, uuids: 1, doubles: 3, bytes: 3, shorts: 3),
  //   ID::SpawnPainting             => id_init!(ints: 2, uuids: 1, positions: 1, bytes: 1),
  //   ID::SpawnPlayer               => id_init!(ints: 1, uuids: 1, doubles: 3, bytes: 2),
  //   ID::EntityAnimation           => id_init!(ints: 1, bytes: 1),
  //   ID::Statistics                => id_init!(int_arrs: 3),
  //   ID::AcknowledgePlayerDigging  => id_init!(positions: 1, ints: 2, bools: 1),
  //   ID::BlockBreakAnimation       => id_init!(ints: 1, positions: 1, bytes: 1),
  //   ID::BlockEntityData           => id_init!(positions: 1, bytes: 1, nbt_tags: 1),
  //   ID::BlockAction               => id_init!(positions: 1, bytes: 2, ints: 1),
  //   ID::BlockChange               => id_init!(positions: 1, ints: 1),
  //   ID::BossBar                   => id_init_other!(uuids: 1, ints: 1), // TODO: Custom type here
  //   ID::ServerDifficulty          => id_init!(bytes: 1, bools: 1),
  //   ID::ChatMessage               => id_init!(strs: 1, bytes: 1, uuids: 1),
  //   ID::MultiBlockChange          => id_init!(positions: 1, bools: 1, byte_arrs: 1),
  //   ID::TabComplete               => id_init_other!(ints: 3), // TODO: Custom type here
  //   ID::DeclareCommands           => id_init!(ints: 1, byte_arrs: 1),
  //   ID::WindowConfirm             => id_init!(bytes: 1, shorts: 1, bools: 1),
  //   ID::CloseWindow               => id_init!(bytes: 1),
  //   ID::WindowItems               => id_init_other!(bytes: 1, shorts: 1), // TODO: Setup slot type
  //   ID::WindowProperty            => id_init!(bytes: 1, shorts: 2),
  //   ID::SetSlot                   => id_init_other!(bytes: 1, shorts: 1),
  //   ID::SetCooldown               => id_init!(ints: 2),
  //   ID::PluginMessage             => id_init!(strs: 1, byte_arrs: 1),
  //   // There is an extra int here, which should be used to set the sound id.
  //   // This is for backwards compatibility, as older clients do not use a sound by name.
  //   ID::NamedSoundEffect          => id_init!(strs: 1, ints: 5, floats: 2),
  //   ID::Disconnect                => id_init!(strs: 1),
  //   ID::EntityStatus              => id_init!(ints: 1, bytes: 1),
  //   ID::Explosion                 => id_init!(floats: 7, byte_arrs: 1),
  //   ID::UnloadChunk               => id_init!(ints: 2),
  //   ID::ChangeGameState           => id_init!(bytes: 1, floats: 1),
  //   ID::OpenHorseWindow           => id_init!(bytes: 1, ints: 2), // TODO: Find out more about this packet
  //   // Newer clients use a long, which is ridiculous. Just cast this on newer clients.
  //   ID::KeepAlive                 => id_init!(ints: 1),
  //   // This should be its own type. It changes so much that relying on int arrays is too difficult.
  //   ID::ChunkData                 => id_init_other!(), // TODO: Custom type here
  //   ID::Effect                    => id_init!(ints: 2, positions: 1, bools: 1),
  //   ID::Particle                  => id_init!(ints: 2, bools: 1, doubles: 3, floats: 4, byte_arrs: 1),
  //   // Only used on newer versions. For older clients, light data is sent with ChunkData.
  //   ID::UpdateLight               => id_init!(ints: 6, bools: 1, byte_arrs: 2),
  //   ID::JoinGame                  => id_init!(ints: 3, bools: 5, bytes: 1, str_arrs: 1, nbt_tags: 2, strs: 1, longs: 1),
  //   ID::MapData                   => id_init_other!(), // TODO: Custom proto
  //   ID::TradeList                 => id_init_other!(), // TODO: Custom proto
  //   ID::EntityPosition            => id_init!(ints: 1, shorts: 3, bools: 1),
  //   ID::EntityPositionAndRotation => id_init!(ints: 1, shorts: 3, bytes: 2, bools: 1),
  //   ID::EntityRotation            => id_init!(ints: 1, bytes: 2, bools: 1),
  //   ID::EntityOnGround            => id_init!(ints: 1, bools: 1),
  //   ID::VehicleMove               => id_init!(doubles: 3, floats: 2),
  //   ID::OpenBook                  => id_init!(ints: 1),
  //   ID::OpenWindow                => id_init!(ints: 2, strs: 1),
  //   ID::OpenSignEditor            => id_init!(positions: 1),
  //   ID::CraftRecipeResponse       => id_init!(bytes: 1, strs: 1),
  //   ID::PlayerAbilities           => id_init!(bytes: 1, floats: 2),
  //   // Enter combat and End combat are ignored, so the event will always be 2: Entity Dead,
  //   // which will display the death screen.
  //   ID::EnterCombat               => id_init!(ints: 2, strs: 1),
  //   ID::PlayerInfo                => id_init_other!(), // TODO: Custom proto
  //   ID::FacePlayer                => id_init!(ints: 3, doubles: 3, bools: 1),
  //   ID::PlayerPositionAndLook     => id_init!(doubles: 3, floats: 2, bytes: 1, ints: 1),
  //   ID::UnlockRecipies            => id_init!(ints: 1, bools: 8, str_arrs: 2),
  //   ID::DestroyEntity             => id_init!(int_arrs: 1),
  //   ID::RemoveEntityEffect        => id_init!(ints: 1, bytes: 1),
  //   ID::ResourcePack              => id_init!(strs: 2),
  //   ID::Respawn                   => id_init!(nbt_tags: 1, strs: 1, longs: 1, bytes: 2, bools: 3),
  //   ID::EntityHeadLook            => id_init!(ints: 1, bytes: 1),
  //   // Empty string means that the string is not present (in this case).
  //   ID::SelectAdvancementTab      => id_init!(strs: 1),
  //   ID::WorldBorder               => id_init_other!(), // TODO: Custom proto
  //   ID::Camera                    => id_init!(ints: 1),
  //   ID::HeldItemChange            => id_init!(bytes: 1),
  //   ID::UpdateViewPosition        => id_init!(ints: 2),
  //   ID::UpdateViewDistance        => id_init!(ints: 1),
  //   ID::DisplayScoreboard         => id_init!(bytes: 1, strs: 1),
  //   ID::EntityMetadata            => id_init!(ints: 1, byte_arrs: 1),
  //   ID::AttachEntity              => id_init!(ints: 2),
  //   ID::EntityVelocity            => id_init!(ints: 1, shorts: 3),
  //   ID::EntityEquipment           => id_init_other!(ints: 1, bytes: 1),
  //   ID::SetExp                    => id_init!(floats: 1, ints: 2),
  //   ID::UpdateHealth              => id_init!(floats: 2, ints: 1),
  //   ID::ScoreboardObjective       => id_init!(strs: 2, bytes: 1, ints: 1),
  //   ID::SetPassengers             => id_init!(ints: 2, int_arrs: 1),
  //   ID::Teams                     => id_init_other!(), // TODO: Custom proto
  //   ID::UpdateScore               => id_init!(strs: 2, bytes: 1, ints: 1),
  //   ID::SpawnPosition             => id_init!(positions: 1),
  //   ID::TimeUpdate                => id_init!(longs: 2),
  //   ID::Title                     => id_init_other!(), // TODO: Custom proto
  //   // TODO: Sound ids change to a string in 1.17
  //   ID::EntitySoundEffect         => id_init!(ints: 3, floats: 2),
  //   ID::SoundEffect               => id_init!(ints: 5, floats: 2),
  //   ID::StopSound                 => id_init!(bytes: 1, ints: 1, strs: 1),
  //   ID::PlayerListHeader          => id_init!(strs: 2),
  //   ID::NBTQueryResponse          => id_init!(ints: 1, nbt_tags: 1),
  //   ID::CollectItem               => id_init!(ints: 3),
  //   ID::EntityTeleport            => id_init!(ints: 1, doubles: 3, bytes: 2, bools: 1),
  //   ID::Advancements              => id_init_other!(), // TODO: Custom proto
  //   ID::EntityProperties          => id_init_other!(), // TODO: Custom proto
  //   // TODO: Effect ids change to strings in 1.17
  //   ID::EntityEffect              => id_init!(ints: 2, bytes: 3),
  //   ID::DeclareRecipies           => id_init_other!(), // TODO: Custom proto
  //   ID::Tags                      => id_init_other!(), // TODO: Custom proto
  //   // ID::Login                     => id_init!(other: 1),
  //   _                             => proto::Packet::default(),
  // }
  id_init!()
}
