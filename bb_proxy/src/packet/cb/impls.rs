use super::{ToTcp, WriteError};
use crate::{
  gnet::cb::{packet as gpacket, Packet as GPacket},
  stream::PacketStream,
  Conn,
};
use bb_common::{
  net::{cb, cb::packet},
  util::{Buffer, GameMode, Hand, UUID},
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
    $packet:ident => ($self:ident, $conn:ident, $ver:ident) $block:block
  ) => {
    impl ToTcp for packet::$packet {
      fn to_tcp<S: PacketStream + Send + Sync>(
        $self: Self,
        $conn: &mut Conn<S>,
      ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
        let $ver = $conn.ver();
        Ok(smallvec![$block])
      }
    }
  };
}
macro_rules! to_tcp_manual {
  (
    $packet:ident => ($self:ident, $conn:ident, $ver:ident) $block:block
  ) => {
    impl ToTcp for packet::$packet {
      fn to_tcp<S: PacketStream + Send + Sync>(
        $self: Self,
        $conn: &mut Conn<S>,
      ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
        let $ver = $conn.ver();
        $block
      }
    }
  };
}

to_tcp!(Abilities => (self, conn, ver) {
  if ver < ProtocolVersion::V1_16_5 {
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
to_tcp!(Animation => (self, conn, ver) {
  if ver == ProtocolVersion::V1_8 {
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
to_tcp_manual!(Chunk => (self, conn, ver) {
  Ok(super::super::chunk(
    self,
    ver,
    conn.conv(),
  ))
});
to_tcp!(BlockUpdate => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_19 {
    gpacket!(BlockUpdate V19 { pos: self.pos, state: self.state as i32 })
  } else {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(self.state as i32);
    gpacket!(BlockUpdate V8 { block_position: self.pos, unknown: data })
  }
});
to_tcp!(ChangeGameState => (self, conn, ver) {
  use bb_common::net::cb::ChangeGameStateKind as Action;

  let reason = match self.action {
    Action::InvalidBed => 0,
    Action::EndRaining => 1,
    Action::BeginRaining => 2,
    Action::GameMode(_) => 3,
    Action::EnterCredits => 4,
    Action::DemoMessage(_) => 5,
    Action::ArrowHitPlayer => 6,
    Action::FadeValue(_) => 7,
    Action::FadeTime(_) => 8,
    Action::PufferfishSting => {
      if ver < ProtocolVersion::V1_14_4 {
        return Err(WriteError::InvalidVer);
      } else {
        9
      }
    }
    Action::ElderGuardianAppear => 10,
    Action::EnableRespawnScreen(_) => {
      if ver < ProtocolVersion::V1_15_2 {
        return Err(WriteError::InvalidVer);
      } else {
        9
      }
    }
  };
  let value = match self.action {
    Action::GameMode(mode) => match mode {
      GameMode::Survival => 0.0,
      GameMode::Creative => 1.0,
      GameMode::Adventure => 2.0,
      GameMode::Spectator => 3.0,
    },
    Action::DemoMessage(v) => v,
    Action::FadeValue(v) => v,
    Action::FadeTime(v) => v,
    Action::EnableRespawnScreen(enable) => {
      if enable {
        0.0
      } else {
        1.0
      }
    }
    _ => 0.0,
  };
  if ver >= ProtocolVersion::V1_16_5 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_u8(reason);
    buf.write_f32(value);
    gpacket!(ChangeGameState V16 { unknown: data })
  } else {
    gpacket!(ChangeGameState V8 { state: reason.into(), field_149141_c: value })
  }
});
to_tcp!(ChatMessage => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_19 {
    gpacket!(SystemChat V19 {
      a: self.msg.to_json(), // content
      b: 1,                  // type
    })
  } else if ver >= ProtocolVersion::V1_16_5 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_u8(self.ty);
    buf.write_uuid(UUID::from_u128(0));
    gpacket!(Chat V12 { chat_component: self.msg.to_json(), unknown: data })
  } else if ver >= ProtocolVersion::V1_12_2 {
    gpacket!(Chat V12 { chat_component: self.msg.to_json(), unknown: vec![self.ty] })
  } else {
    gpacket!(Chat V8 { chat_component: self.msg.to_json(), ty: self.ty as i8 })
  }
});
to_tcp!(CommandList => (self, conn, ver) {
  use bb_common::net::cb::CommandType;

  if ver < ProtocolVersion::V1_13 {
    panic!("command tree doesn't exist for version {}", ver);
  }
  if ver >= ProtocolVersion::V1_19 {
    return Ok(smallvec![]);
  }
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  buf.write_list(&self.nodes, |buf, node| {
    let mut flags = match node.ty {
      CommandType::Root => 0,
      CommandType::Literal => 1,
      CommandType::Argument => 2,
    };
    if node.executable {
      flags |= 0x04;
    }
    if node.redirect.is_some() {
      flags |= 0x08;
    }
    if node.suggestion.is_some() {
      flags |= 0x10;
    }
    buf.write_u8(flags);
    buf.write_list(&node.children, |buf, child| buf.write_varint(*child as i32));
    if let Some(redirect) = node.redirect {
      buf.write_varint(redirect as i32);
    }
    if node.ty == CommandType::Literal || node.ty == CommandType::Argument {
      buf.write_str(&node.name);
    }
    if node.ty == CommandType::Argument {
      if ver >= ProtocolVersion::V1_19 {
        // buf.write_varint(conn.conv().command_to_old(node.parser.id(),
        // ver));
      } else {
        buf.write_str(&node.parser);
      }
      buf.write_buf(&node.properties);
    }
    if let Some(suggestion) = &node.suggestion {
      buf.write_str(suggestion);
    }
  });
  buf.write_varint(self.root as i32);
  if ver >= ProtocolVersion::V1_19 {
    gpacket!(CommandTree V19 { unknown: data })
  } else if ver >= ProtocolVersion::V1_16_5 {
    gpacket!(CommandTree V16 { unknown: data })
  } else {
    gpacket!(CommandTree V14 { unknown: data })
  }
});
