use super::{ToTcp, WriteError};
use crate::{
  gnet::cb::{packet as gpacket, Packet as GPacket},
  stream::PacketStream,
  Conn,
};
use bb_common::{
  net::{cb, cb::packet},
  util::Hand,
  version::ProtocolVersion,
};
use smallvec::SmallVec;

macro_rules! gpacket {
  ( $name:ident $ver:ident { $( $field:ident $(: $value:expr)? ),* $(,)? } ) => {
    concat_idents::concat_idents!(packet_name = $name,$ver {
      GPacket::$name(gpacket::$name::$ver(gpacket::packet_name {
        $(
          $field $(: $value)?,
        )*
      }))
    })
  }
}

macro_rules! to_tcp {
  (
    $packet:ident => ($self:ident, $conn:ident) $block:block
  ) => {
    impl ToTcp for packet::$packet {
      fn to_tcp<S: PacketStream + Send + Sync>(
        $self: Self,
        $conn: &mut Conn<S>,
      ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
        Ok(smallvec![$block])
      }
    }
  };
}
macro_rules! to_tcp_manual {
  (
    $packet:ident => ($self:ident, $conn:ident) $block:block
  ) => {
    impl ToTcp for packet::$packet {
      fn to_tcp<S: PacketStream + Send + Sync>(
        $self: Self,
        $conn: &mut Conn<S>,
      ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
        $block
      }
    }
  };
}

to_tcp!(Abilities => (self, conn) {
  if conn.ver() < ProtocolVersion::V1_16_5 {
    gpacket!(PlayerAbilities V8 {
      invulnerable:  self.invulnerable,
      flying:        self.flying,
      allow_flying:  self.allow_flying,
      creative_mode: self.insta_break,
      fly_speed:     self.fly_speed * 0.05,
      walk_speed:    self.walk_speed * 0.1,
      v_2:           0,
    })
  } else {
    gpacket!(PlayerAbilities V16 {
      invulnerable:  self.invulnerable,
      flying:        self.flying,
      allow_flying:  self.allow_flying,
      creative_mode: self.insta_break,
      fly_speed:     self.fly_speed * 0.05,
      walk_speed:    self.walk_speed * 0.1,
      v_2:           0,
    })
  }
});
to_tcp!(Animation => (self, conn) {
  if conn.ver() == ProtocolVersion::V1_8 {
    gpacket!(Animation V8 {
      entity_id: self.eid,
      ty:        match self.kind {
        cb::AnimationKind::Swing(_) => 0,
        cb::AnimationKind::Damage => 1,
        cb::AnimationKind::LeaveBed => 2,
        cb::AnimationKind::Crit => 4,
        cb::AnimationKind::MagicCrit => 5,
      },
    })
  } else {
    gpacket!(Animation V8 {
      entity_id: self.eid,
      ty:        match self.kind {
        cb::AnimationKind::Swing(Hand::Main) => 0,
        cb::AnimationKind::Damage => 1,
        cb::AnimationKind::LeaveBed => 2,
        cb::AnimationKind::Swing(Hand::Off) => 0,
        cb::AnimationKind::Crit => 4,
        cb::AnimationKind::MagicCrit => 5,
      },
    })
  }
});
to_tcp_manual!(Chunk => (self, conn) {
  Ok(super::super::chunk(
    self,
    conn.ver(),
    conn.conv(),
  ))
});
