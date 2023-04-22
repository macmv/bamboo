use crate::{gnet::cb::Packet as GPacket, stream::PacketStream, Conn};
use bb_common::net::cb::Packet;

use smallvec::SmallVec;
use std::{error::Error, fmt};

mod impls;

#[derive(Debug, Clone)]
pub enum WriteError {
  InvalidVer,
}

impl fmt::Display for WriteError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::InvalidVer => write!(f, "invalid version"),
    }
  }
}

impl Error for WriteError {}

pub trait ToTcp {
  fn to_tcp<S: PacketStream + Send + Sync>(
    self,
    conn: &mut Conn<S>,
  ) -> Result<SmallVec<[GPacket; 2]>, WriteError>;
}

impl ToTcp for Packet {
  fn to_tcp<S: PacketStream + Send + Sync>(
    self,
    conn: &mut Conn<S>,
  ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
    match self {
      Packet::Abilities(p) => p.to_tcp(conn),
      Packet::Animation(p) => p.to_tcp(conn),
      Packet::Chunk(p) => p.to_tcp(conn),
      Packet::BlockUpdate(p) => p.to_tcp(conn),
      Packet::ChangeGameState(p) => p.to_tcp(conn),
      Packet::ChatMessage(p) => p.to_tcp(conn),
      Packet::CommandList(p) => p.to_tcp(conn),
      Packet::CollectItem(p) => p.to_tcp(conn),
      Packet::EntityEquipment(p) => p.to_tcp(conn),
      Packet::EntityHeadLook(p) => p.to_tcp(conn),
      Packet::EntityLook(p) => p.to_tcp(conn),
      Packet::EntityMove(p) => p.to_tcp(conn),
      Packet::EntityMoveLook(p) => p.to_tcp(conn),
      Packet::EntityPos(p) => p.to_tcp(conn),
      Packet::EntityStatus(p) => p.to_tcp(conn),
      Packet::EntityMetadata(p) => p.to_tcp(conn),
      Packet::EntityVelocity(p) => p.to_tcp(conn),
      Packet::JoinGame(p) => p.to_tcp(conn),
      Packet::KeepAlive(p) => p.to_tcp(conn),
      Packet::MultiBlockChange(p) => p.to_tcp(conn),
      Packet::Particle(p) => p.to_tcp(conn),
      Packet::PlayerHeader(p) => p.to_tcp(conn),
      Packet::PlayerList(p) => p.to_tcp(conn),
      Packet::PlaySound(p) => p.to_tcp(conn),
      Packet::PluginMessage(p) => p.to_tcp(conn),
      Packet::RemoveEntities(p) => p.to_tcp(conn),
      Packet::Respawn(p) => p.to_tcp(conn),
      Packet::ScoreboardDisplay(p) => p.to_tcp(conn),
      Packet::ScoreboardObjective(p) => p.to_tcp(conn),
      Packet::ScoreboardUpdate(p) => p.to_tcp(conn),
      Packet::SetPosLook(p) => p.to_tcp(conn),
      Packet::SpawnEntity(p) => p.to_tcp(conn),
      Packet::SpawnPlayer(p) => p.to_tcp(conn),
      Packet::Tags(p) => p.to_tcp(conn),
      Packet::Title(p) => p.to_tcp(conn),
      Packet::Teams(p) => p.to_tcp(conn),
      Packet::UnloadChunk(p) => p.to_tcp(conn),
      Packet::UpdateHealth(p) => p.to_tcp(conn),
      Packet::UpdateViewPos(p) => p.to_tcp(conn),
      Packet::WindowOpen(p) => p.to_tcp(conn),
      Packet::WindowItems(p) => p.to_tcp(conn),
      Packet::WindowItem(p) => p.to_tcp(conn),
      _ => todo!("convert {:?} into generated packet", self),
    }
  }
}

fn object_ty(entity: i32) -> i32 {
  // I cannot find the normal entity ids for these objects:
  // _ => 11,   // Minecart (storage, unused)
  // _ => 12,   // Minecart (powered, unused)
  // _ => 74,   // Falling Dragon Egg
  // _ => 90,   // Fishing Float
  // _ => 92,  // Tipped Arrow
  match entity {
    41 => 1,   // Boat
    1 => 2,    // Item Stack (Slot)
    3 => 3,    // Area Effect Cloud
    42 => 10,  // Minecart
    20 => 50,  // Activated TNT
    200 => 51, // EnderCrystal
    10 => 60,  // Arrow (projectile)
    11 => 61,  // Snowball (projectile)
    7 => 62,   // Egg (projectile)
    12 => 63,  // FireBall (ghast projectile)
    13 => 64,  // FireCharge (blaze projectile)
    14 => 65,  // Thrown Enderpearl
    19 => 66,  // Wither Skull (projectile)
    25 => 67,  // Shulker Bullet
    21 => 70,  // Falling Objects
    18 => 71,  // Item frames
    15 => 72,  // Eye of Ender
    16 => 73,  // Thrown Potion
    17 => 75,  // Thrown Exp Bottle
    22 => 76,  // Firework Rocket
    8 => 77,   // Leash Knot
    30 => 78,  // ArmorStand
    24 => 91,  // Spectral Arrow
    26 => 93,  // Dragon Fireball
    _ => panic!("not an object: {entity}"),
  }
}
