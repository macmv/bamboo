use super::{super::metadata, ToTcp, WriteError};
use crate::{
  gnet::{
    cb::{packet as gpacket, Packet as GPacket},
    tcp,
  },
  stream::PacketStream,
  Conn,
};
use bb_common::{
  net::{cb, cb::packet},
  util::{chat, Buffer, GameMode, Hand, UUID},
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
    $packet:ident => (mut $self:ident, $conn:ident, $ver:ident) $block:block
  ) => {
    impl ToTcp for packet::$packet {
      fn to_tcp<S: PacketStream + Send + Sync>(
        mut $self: Self,
        $conn: &mut Conn<S>,
      ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
        let $ver = $conn.ver();
        Ok(smallvec![$block])
      }
    }
  };
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
      b: true,               // type
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
to_tcp!(CollectItem => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_11_2 {
    gpacket!(CollectItem V11 {
      collected_item_entity_id: self.item_eid,
      entity_id:                self.player_eid,
      field_191209_c:           self.amount.into(),
    })
  } else {
    gpacket!(CollectItem V8 {
      collected_item_entity_id: self.item_eid,
      entity_id:                self.player_eid,
    })
  }
});
to_tcp!(EntityEquipment => (mut self, conn, ver) {
  use bb_common::net::cb::{ArmorSlot, EquipmentSlot};

  conn.conv().item(&mut self.item, ver.block());
  if ver >= ProtocolVersion::V1_16_5 {
    let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
    // TODO: Multiple equipment updates can be sent in one packet on this version.
    // This is serialized as an array, where this byte has the top bit set if there
    // is another entry. We keep this top bit unset, as this is the single (and
    // last) entry.
    buf.write_u8(match self.slot {
      EquipmentSlot::Hand(Hand::Main) => 0,
      EquipmentSlot::Hand(Hand::Off) => 1,
      EquipmentSlot::Armor(ArmorSlot::Boots) => 2,
      EquipmentSlot::Armor(ArmorSlot::Leggings) => 3,
      EquipmentSlot::Armor(ArmorSlot::Chestplate) => 4,
      EquipmentSlot::Armor(ArmorSlot::Helmet) => 5,
    });
    buf.write_item(&self.item, conn.conv());
    gpacket!(EntityEquipment V16 { id: self.eid, unknown: buf.serialize() })
  } else if ver >= ProtocolVersion::V1_9_4 {
    let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
    buf.write_item(&self.item, conn.conv());
    gpacket!(EntityEquipment V9 {
      entity_id:      self.eid,
      equipment_slot: match self.slot {
        EquipmentSlot::Hand(Hand::Main) => 0,
        EquipmentSlot::Hand(Hand::Off) => 1,
        EquipmentSlot::Armor(ArmorSlot::Boots) => 2,
        EquipmentSlot::Armor(ArmorSlot::Leggings) => 3,
        EquipmentSlot::Armor(ArmorSlot::Chestplate) => 4,
        EquipmentSlot::Armor(ArmorSlot::Helmet) => 5,
      },
      unknown:        buf.serialize(),
    })
  } else {
    let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
    buf.write_item(&self.item, conn.conv());
    gpacket!(EntityEquipment V8 {
      entity_id:      self.eid,
      equipment_slot: match self.slot {
        EquipmentSlot::Hand(Hand::Main) => 0,
        // 1.8 client can't see offhand, so we can't really send them anything
        EquipmentSlot::Hand(Hand::Off) => return Ok(smallvec![]),
        EquipmentSlot::Armor(ArmorSlot::Boots) => 1,
        EquipmentSlot::Armor(ArmorSlot::Leggings) => 2,
        EquipmentSlot::Armor(ArmorSlot::Chestplate) => 3,
        EquipmentSlot::Armor(ArmorSlot::Helmet) => 4,
      },
      unknown:        buf.serialize(),
    })
  }
});
to_tcp!(EntityHeadLook => (self, conn, _ver) {
  gpacket!(EntityHeadLook V8 { entity_id: self.eid, yaw: self.yaw })
});
to_tcp!(EntityLook => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_17_1 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(self.eid);
    buf.write_i8(self.yaw);
    buf.write_i8(self.pitch);
    buf.write_bool(self.on_ground);
    gpacket!(EntityLook V17 { unknown: data, v_1: 0, v_2: 0, v_3: 0, v_4: 0 })
  } else {
    gpacket!(EntityLook V8 {
      entity_id: self.eid,
      yaw:       self.yaw,
      pitch:     self.pitch,
      on_ground: self.on_ground,
    })
  }
});
to_tcp!(EntityMove => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_17_1 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(self.eid);
    buf.write_i16(self.x);
    buf.write_i16(self.y);
    buf.write_i16(self.z);
    buf.write_bool(self.on_ground);
    gpacket!(EntityRelMove V17 {
      unknown: data,
      v_1:     0,
      v_2:     0,
      v_3:     0,
      v_4:     0,
      v_5:     0,
    })
  } else if ver >= ProtocolVersion::V1_9_4 {
    gpacket!(EntityRelMove V9 {
      entity_id: self.eid,
      pos_x:     self.x.into(),
      pos_y:     self.y.into(),
      pos_z:     self.z.into(),
      on_ground: self.on_ground,
    })
  } else {
    gpacket!(EntityRelMove V8 {
      entity_id: self.eid,
      pos_x:     (self.x / (4096 / 32)) as i8,
      pos_y:     (self.y / (4096 / 32)) as i8,
      pos_z:     (self.z / (4096 / 32)) as i8,
      on_ground: self.on_ground,
    })
  }
});
to_tcp!(EntityMoveLook => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_17_1 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(self.eid);
    buf.write_i16(self.x);
    buf.write_i16(self.y);
    buf.write_i16(self.z);
    buf.write_i8(self.yaw);
    buf.write_i8(self.pitch);
    buf.write_bool(self.on_ground);
    gpacket!(EntityLookMove V17 {
      unknown: data,
      v_1:     0,
      v_2:     0,
      v_3:     0,
      v_4:     0,
      v_5:     0,
      v_6:     0,
      v_7:     0,
    })
  } else if ver >= ProtocolVersion::V1_9_4 {
    gpacket!(EntityLookMove V9 {
      entity_id: self.eid,
      pos_x:     self.x.into(),
      pos_y:     self.y.into(),
      pos_z:     self.z.into(),
      yaw:       self.yaw,
      pitch:     self.pitch,
      on_ground: self.on_ground,
    })
  } else {
    gpacket!(EntityLookMove V8 {
      entity_id: self.eid,
      pos_x:     (self.x / (4096 / 32)) as i8,
      pos_y:     (self.y / (4096 / 32)) as i8,
      pos_z:     (self.z / (4096 / 32)) as i8,
      yaw:       self.yaw,
      pitch:     self.pitch,
      on_ground: self.on_ground,
    })
  }
});
to_tcp!(EntityPos => (self, conn, ver) {
  if ver == ProtocolVersion::V1_8 {
    gpacket!(EntityTeleport V8 {
      entity_id: self.eid,
      pos_x:     (self.x * 32.0) as i32,
      pos_y:     (self.y * 32.0) as i32,
      pos_z:     (self.z * 32.0) as i32,
      yaw:       self.yaw,
      pitch:     self.pitch,
      on_ground: self.on_ground,
    })
  } else {
    gpacket!(EntityTeleport V9 {
      entity_id: self.eid,
      pos_x:     self.x,
      pos_y:     self.y,
      pos_z:     self.z,
      yaw:       self.yaw,
      pitch:     self.pitch,
      on_ground: self.on_ground,
    })
  }
});
to_tcp!(EntityStatus => (self, conn, _ver) {
  gpacket!(EntityStatus V8 { entity_id: self.eid, logic_opcode: self.status as i8 })
});
to_tcp!(EntityMetadata => (self, conn, ver) {
  gpacket!(EntityMetadata V8 {
    entity_id: self.eid,
    unknown:   match metadata(self.ty, &self.meta, ver, conn.conv()) {
      Some(m) => m,
      None => return Ok(smallvec![]),
    },
  })
});
to_tcp!(EntityVelocity => (self, conn, _ver) {
  gpacket!(EntityVelocity V8 {
    entity_id: self.eid,
    motion_x:  self.x.into(),
    motion_y:  self.y.into(),
    motion_z:  self.z.into(),
  })
});
to_tcp!(JoinGame => (self, conn, ver) {
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  if ver >= ProtocolVersion::V1_16_5 {
    buf.write_u8(self.game_mode.id());
    buf.write_i8(-1); // no previous_game_mode

    // List of worlds
    buf.write_varint(1);
    buf.write_str("minecraft:overworld");

    crate::registry::write_codec(&mut buf, ver, self.world_min_y, self.world_height);

    // Hashed world seed, used for biomes client side.
    buf.write_u64(0);
    // Max players (ignored)
    buf.write_varint(0);

    buf.write_varint(self.view_distance.into());
    if ver >= ProtocolVersion::V1_18 {
      // The simulation distance
      buf.write_varint(self.view_distance.into());
    }
    buf.write_bool(self.reduced_debug_info);
    buf.write_bool(self.enable_respawn_screen);
    buf.write_bool(false); // Is debug; cannot be modified, has preset blocks
    buf.write_bool(false); // Is flat; changes fog
    if ver >= ProtocolVersion::V1_19 {
      // Last death location.
      buf.write_option(&None, |_, _: &()| {});
    }
  } else if ver >= ProtocolVersion::V1_15_2 {
    buf.write_i32(self.dimension.into());
    // Hashed world seed, used for biomes
    buf.write_u64(0);
    // Max players (ignored)
    buf.write_u8(0);
    // World type
    buf.write_str("default");
    buf.write_varint(self.view_distance.into());
    buf.write_bool(self.reduced_debug_info);
    buf.write_bool(self.enable_respawn_screen);
  } else if ver >= ProtocolVersion::V1_14_4 {
    buf.write_i32(self.dimension.into());
    // Max players (ignored)
    buf.write_u8(0);
    // World type
    buf.write_str("default");
    buf.write_varint(self.view_distance.into());
    buf.write_bool(self.reduced_debug_info);
  } else {
    buf.write_bool(self.reduced_debug_info);
  }

  match ver.maj().unwrap() {
    8 => gpacket!(JoinGame V8 {
      entity_id:     self.eid,
      hardcore_mode: self.hardcore_mode,
      game_type:     self.game_mode.id(),
      dimension:     self.dimension.into(),
      difficulty:    self.difficulty.into(),
      max_players:   0,
      world_type:    self.level_type,
      unknown:       data,
    }),
    9..=13 => gpacket!(JoinGame V9 {
      player_id:     self.eid,
      hardcore_mode: self.hardcore_mode,
      game_type:     self.game_mode.id(),
      dimension:     self.dimension.into(),
      difficulty:    self.difficulty.into(),
      max_players:   0,
      world_type:    self.level_type,
      unknown:       data,
    }),
    14..=15 => gpacket!(JoinGame V14 {
      player_entity_id: self.eid,
      hardcore:         self.hardcore_mode,
      unknown:          data,
      v_2:              0,
    }),
    16 => gpacket!(JoinGame V16 {
      player_entity_id: self.eid,
      hardcore:         self.hardcore_mode,
      unknown:          data,
    }),
    17 | 18 | 19 => gpacket!(JoinGame V17 {
      player_entity_id: self.eid,
      hardcore:         self.hardcore_mode,
      unknown:          data,
    }),
    _ => unimplemented!(),
  }
});
to_tcp!(KeepAlive => (self, conn, ver) {
  if ver < ProtocolVersion::V1_12_2 {
    gpacket!(KeepAlive V8 { id: self.id as i32 })
  } else {
    gpacket!(KeepAlive V12 { id: self.id.into() })
  }
});
to_tcp!(MultiBlockChange => (self, conn, ver) {
  super::super::multi_block_change(self.pos, self.y, self.changes, ver, conn.conv())
});
to_tcp!(Particle => (self, conn, ver) {
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  let old_id = match conn.conv().particle_to_old(self.id as u32, ver.block()) {
    Some(id) => id as i32,
    None => return Ok(smallvec![]),
  };
  if ver >= ProtocolVersion::V1_14_4 {
    buf.write_i32(old_id);
    buf.write_bool(self.long);
    buf.write_f64(self.pos.x());
    buf.write_f64(self.pos.y());
    buf.write_f64(self.pos.z());
    buf.write_f32(self.offset.x() as f32);
    buf.write_f32(self.offset.y() as f32);
    buf.write_f32(self.offset.z() as f32);
    buf.write_f32(self.data_float);
    buf.write_i32(self.count);
    buf.write_buf(&self.data);
    gpacket!(Particle V14 { unknown: data })
  } else {
    buf.write_i32(old_id);
    buf.write_bool(self.long);
    buf.write_f32(self.pos.x() as f32);
    buf.write_f32(self.pos.y() as f32);
    buf.write_f32(self.pos.z() as f32);
    buf.write_f32(self.offset.x() as f32);
    buf.write_f32(self.offset.y() as f32);
    buf.write_f32(self.offset.z() as f32);
    buf.write_f32(self.data_float);
    buf.write_i32(self.count);
    buf.write_buf(&self.data);
    gpacket!(Particle V8 { unknown: data })
  }
});
to_tcp!(PlayerHeader => (self, conn, _ver) {
  gpacket!(PlayerListHeader V8 { header: self.header, footer: self.footer })
});
to_tcp!(PlayerList => (self, conn, ver) {
  let id;
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  match self.action {
    cb::PlayerListAction::Add(v) => {
      id = 0;
      if ver >= ProtocolVersion::V1_19_3 {
        buf.write_const_bit_set(&[0x01]);
      }
      buf.write_list(&v, |buf, v| {
        buf.write_uuid(v.id);
        buf.write_str(&v.name);
        buf.write_varint(0);
        // This info is no longer sent as of 1.19.3.
        if ver < ProtocolVersion::V1_19_3 {
          buf.write_varint(v.game_mode.id().into());
          buf.write_varint(v.ping);
          buf.write_option(&v.display_name, |buf, v| buf.write_str(v));
          // The user's public key
          if ver >= ProtocolVersion::V1_19 {
            buf.write_option(&None, |_, _: &()| {});
          }
        }
      });
    }
    // TODO: InitializeChat action, which is bit 1 (0x02)
    cb::PlayerListAction::UpdateGameMode(v) => {
      id = 1;
      if ver >= ProtocolVersion::V1_19_3 {
        buf.write_const_bit_set(&[0x04]);
      }
      buf.write_list(&v, |buf, v| {
        buf.write_uuid(v.id);
        buf.write_varint(v.game_mode.id().into());
      });
    }
    // TODO: UpdateListed action, which is bit 3 (0x08)
    cb::PlayerListAction::UpdateLatency(v) => {
      id = 2;
      if ver >= ProtocolVersion::V1_19_3 {
        buf.write_const_bit_set(&[0x10]);
      }
      buf.write_list(&v, |buf, v| {
        buf.write_uuid(v.id);
        buf.write_varint(v.ping);
      });
    }
    cb::PlayerListAction::UpdateDisplayName(v) => {
      id = 3;
      if ver >= ProtocolVersion::V1_19_3 {
        buf.write_const_bit_set(&[0x20]);
      }
      buf.write_list(&v, |buf, v| {
        buf.write_uuid(v.id);
        buf.write_option(&v.display_name, |buf, v| buf.write_str(&v.to_json()));
      });
    }
    cb::PlayerListAction::Remove(v) => {
      id = 4;
      // There is no remove action in this version.
      if ver >= ProtocolVersion::V1_19_3 {
        return Ok(smallvec![]);
      }
      buf.write_list(&v, |buf, v| {
        buf.write_uuid(v.id);
      });
    }
  }
  if ver >= ProtocolVersion::V1_19_3 {
    gpacket!(PlayerList V19 { unknown: data })
  } else if ver >= ProtocolVersion::V1_17_1 {
    gpacket!(PlayerList V17 { action: id, unknown: data })
  } else {
    gpacket!(PlayerList V8 { action: id, unknown: data, v_2: 0 })
  }
});
to_tcp!(PlaySound => (self, conn, ver) {
  use bb_common::net::cb::SoundCategory;

  if ver >= ProtocolVersion::V1_14_4 {
    gpacket!(CustomSound V14 {
      id:       self.name,
      category: match self.category {
        SoundCategory::Master => 0,
        SoundCategory::Music => 1,
        SoundCategory::Records => 2,
        SoundCategory::Weather => 3,
        SoundCategory::Blocks => 4,
        SoundCategory::Hostile => 5,
        SoundCategory::Neutral => 6,
        SoundCategory::Players => 7,
        SoundCategory::Ambient => 8,
        SoundCategory::Voice => 9,
      },
      fixed_x:  (self.pos.x() * 8.0) as i32,
      fixed_y:  (self.pos.y() * 8.0) as i32,
      fixed_z:  (self.pos.z() * 8.0) as i32,
      volume:   self.volume,
      pitch:    self.pitch,
    })
  } else if ver >= ProtocolVersion::V1_10_2 {
    gpacket!(CustomSound V10 {
      sound_name: self.name,
      category:   match self.category {
        SoundCategory::Master => 0,
        SoundCategory::Music => 1,
        SoundCategory::Records => 2,
        SoundCategory::Weather => 3,
        SoundCategory::Blocks => 4,
        SoundCategory::Hostile => 5,
        SoundCategory::Neutral => 6,
        SoundCategory::Players => 7,
        SoundCategory::Ambient => 8,
        SoundCategory::Voice => 9,
      },
      x:          (self.pos.x() * 8.0) as i32,
      y:          (self.pos.y() * 8.0) as i32,
      z:          (self.pos.z() * 8.0) as i32,
      volume:     self.volume,
      pitch:      self.pitch,
    })
  } else if ver >= ProtocolVersion::V1_9_4 {
    gpacket!(CustomSound V9 {
      sound_name: self.name,
      category:   match self.category {
        SoundCategory::Master => 0,
        SoundCategory::Music => 1,
        SoundCategory::Records => 2,
        SoundCategory::Weather => 3,
        SoundCategory::Blocks => 4,
        SoundCategory::Hostile => 5,
        SoundCategory::Neutral => 6,
        SoundCategory::Players => 7,
        SoundCategory::Ambient => 8,
        SoundCategory::Voice => 9,
      },
      x:          (self.pos.x() * 8.0) as i32,
      y:          (self.pos.y() * 8.0) as i32,
      z:          (self.pos.z() * 8.0) as i32,
      volume:     self.volume,
      pitch:      (self.pitch * 128.0) as i32,
    })
  } else {
    gpacket!(PlaySound V8 {
      sound_name:   self.name,
      pos_x:        (self.pos.x() * 8.0) as i32,
      pos_y:        (self.pos.y() * 8.0) as i32,
      pos_z:        (self.pos.z() * 8.0) as i32,
      sound_volume: self.volume,
      sound_pitch:  (self.pitch * 128.0) as i32,
    })
  }
});
to_tcp!(PluginMessage => (self, conn, ver) {
  // No length prefix for data, it is inferred from packet length.
  if ver < ProtocolVersion::V1_14_4 {
    gpacket!(CustomPayload V8 { channel: self.channel, unknown: self.data, v_2: 0 })
  } else {
    gpacket!(CustomPayload V14 { channel: self.channel, unknown: self.data, v_2: 0 })
  }
});
to_tcp!(RemoveEntities => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_17_1 {
    gpacket!(DestroyEntities V17 { entity_ids: self.eids })
  } else {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_list(&self.eids, |buf, &e| buf.write_varint(e));
    gpacket!(DestroyEntities V8 { unknown: data })
  }
});
to_tcp!(Respawn => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_14 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_i32(self.dimension.into());
    if ver >= ProtocolVersion::V1_16_5 {
      crate::registry::write_single_dimension(&mut buf, ver, 0, 256);
      buf.write_str("minecraft:overworld");
    }
    if ver >= ProtocolVersion::V1_15_2 {
      // hashed seed, same as join game
      buf.write_u64(0);
    }
    buf.write_u8(self.game_mode.id());
    // previous game mode
    if ver >= ProtocolVersion::V1_16_5 {
      buf.write_u8(self.game_mode.id());
      buf.write_bool(false); // Is debug; cannot be modified, has preset blocks
      buf.write_bool(false); // Is flat; changes fog
      buf.write_bool(self.reset_meta);
    } else {
      buf.write_str(&self.level_type);
    }
    gpacket!(Respawn V14 {
      unknown: data,
    })
  } else {
    gpacket!(Respawn V8 {
      dimension_id: self.dimension.into(),
      difficulty:   self.difficulty.into(),
      game_type:    self.game_mode.id(),
      world_type:   self.level_type,
      unknown:      vec![],
    })
  }
});
to_tcp!(ScoreboardDisplay => (self, conn, ver) {
  use bb_common::net::cb::ScoreboardDisplayPosition;

  let pos = match self.position {
    ScoreboardDisplayPosition::List => 0,
    ScoreboardDisplayPosition::Sidebar => 1,
    ScoreboardDisplayPosition::BelowName => 2,
  };
  if ver < ProtocolVersion::V1_18 {
    gpacket!(ScoreboardDisplay V8 { position: pos, score_name: self.objective })
  } else {
    gpacket!(ScoreboardDisplay V18 { slot: pos, name: self.objective })
  }
});
to_tcp!(ScoreboardObjective => (self, conn, ver) {
  use bb_common::net::cb::{ObjectiveAction, ObjectiveType};

  let m = match self.mode {
    ObjectiveAction::Create { .. } => 0,
    ObjectiveAction::Remove => 1,
    ObjectiveAction::Update { .. } => 2,
  };
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  match self.mode {
    ObjectiveAction::Create { value, ty } | ObjectiveAction::Update { value, ty } => {
      if ver <= ProtocolVersion::V1_12_2 {
        buf.write_str(&value.to_codes());
      } else {
        buf.write_str(&value.to_json());
      }
      buf.write_varint(match ty {
        ObjectiveType::Integer => 0,
        ObjectiveType::Hearts => 1,
      });
    }
    _ => {}
  }
  if ver < ProtocolVersion::V1_18 {
    gpacket!(ScoreboardObjective V8 {
      objective_name: self.objective,
      field_149342_c: m,
      unknown:        data,
    })
  } else {
    gpacket!(ScoreboardObjective V18 { name: self.objective, mode: m, unknown: data })
  }
});
to_tcp!(ScoreboardUpdate => (self, conn, ver) {
  use bb_common::net::cb::ScoreboardAction;

  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  if ver >= ProtocolVersion::V1_18 {
    buf.write_str(&self.objective);
    match self.action {
      ScoreboardAction::Create(score) => buf.write_varint(score),
      ScoreboardAction::Remove => {}
    }
    gpacket!(UpdateScore V18 {
      player_name: self.username,
      mode:        match self.action {
        ScoreboardAction::Create(_) => 0,
        ScoreboardAction::Remove => 1,
      },
      unknown:     data,
      v_2:         "".into(),
    })
  } else if ver >= ProtocolVersion::V1_14_4 {
    buf.write_str(&self.objective);
    match self.action {
      ScoreboardAction::Create(score) => buf.write_varint(score),
      ScoreboardAction::Remove => {}
    }
    gpacket!(UpdateScore V14 {
      player_name: self.username,
      mode:        match self.action {
        ScoreboardAction::Create(_) => 0,
        ScoreboardAction::Remove => 1,
      },
      unknown:     data,
      v_2:         "".into(),
    })
  } else {
    match self.action {
      ScoreboardAction::Create(score) => buf.write_varint(score),
      ScoreboardAction::Remove => {}
    }
    gpacket!(UpdateScore V8 {
      name:      self.username,
      objective: self.objective,
      action:    match self.action {
        ScoreboardAction::Create(_) => 0,
        ScoreboardAction::Remove => 1,
      },
      unknown:   data,
    })
  }
});
to_tcp!(SetPosLook => (self, conn, ver) {
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  buf.write_u8(self.flags);
  if ver >= ProtocolVersion::V1_9 {
    buf.write_varint(self.teleport_id as i32);
  }
  if ver >= ProtocolVersion::V1_17_1 && ver <= ProtocolVersion::V1_19_3 {
    buf.write_bool(self.should_dismount);
  }
  gpacket!(PlayerPosLook V8 {
    x:       self.pos.x(),
    y:       self.pos.y(),
    z:       self.pos.z(),
    yaw:     self.yaw,
    pitch:   self.pitch,
    unknown: data,
  })
});
to_tcp!(SpawnEntity => (self, conn, ver) {
  let ty = conn.conv().entity_to_old(self.ty, ver.block()) as i32;
  if ver >= ProtocolVersion::V1_19 {
    gpacket!(SpawnObject V19 {
      id:          self.eid,
      uuid:        self.id,
      entity_type: ty as u32,
      x:           self.pos.x(),
      y:           self.pos.y(),
      z:           self.pos.z(),
      pitch:       self.pitch,
      yaw:         self.yaw,
      head_yaw:    self.head_yaw,
      entity_data: self.data,
      velocity_x:  self.vel_x.into(),
      velocity_y:  self.vel_y.into(),
      velocity_z:  self.vel_z.into(),
    })
  } else if self.living {
    // Handle living mobs for 1.8-1.18
    if ver >= ProtocolVersion::V1_15_2 {
      let spawn = gpacket!(SpawnMob V15 {
        id:             self.eid,
        uuid:           self.id,
        entity_type_id: ty,
        x:              self.pos.x(),
        y:              self.pos.y(),
        z:              self.pos.z(),
        velocity_x:     self.vel_x.into(),
        velocity_y:     self.vel_y.into(),
        velocity_z:     self.vel_z.into(),
        yaw:            self.yaw,
        pitch:          self.pitch,
        head_yaw:       self.head_yaw,
      });
      if !self.meta.fields.is_empty() {
        match metadata(self.ty, &self.meta, ver, conn.conv()) {
          Some(data) => {
            return Ok(smallvec![
              spawn,
              gpacket!(EntityMetadata V8 { entity_id: self.eid, unknown: data })
            ])
          }
          None => spawn,
        }
      } else {
        spawn
      }
    } else if ver >= ProtocolVersion::V1_11 {
      gpacket!(SpawnMob V11 {
        entity_id: self.eid,
        unique_id: self.id,
        ty,
        x: self.pos.x(),
        y: self.pos.y(),
        z: self.pos.z(),
        velocity_x: self.vel_x.into(),
        velocity_y: self.vel_y.into(),
        velocity_z: self.vel_z.into(),
        yaw: self.yaw,
        pitch: self.pitch,
        head_pitch: self.head_yaw,
        unknown: match metadata(self.ty, &self.meta, ver, conn.conv()) {
          Some(m) => m,
          None => return Ok(smallvec![]),
        },
      })
    } else if ver >= ProtocolVersion::V1_9 {
      gpacket!(SpawnMob V9 {
        entity_id: self.eid,
        unique_id: self.id,
        ty,
        x: self.pos.x(),
        y: self.pos.y(),
        z: self.pos.z(),
        velocity_x: self.vel_x.into(),
        velocity_y: self.vel_y.into(),
        velocity_z: self.vel_z.into(),
        yaw: self.yaw,
        pitch: self.pitch,
        head_pitch: self.head_yaw,
        unknown: match metadata(self.ty, &self.meta, ver, conn.conv()) {
          Some(m) => m,
          None => return Ok(smallvec![]),
        },
      })
    } else {
      gpacket!(SpawnMob V8 {
        entity_id: self.eid,
        ty,
        x: (self.pos.x() * 32.0) as i32,
        y: (self.pos.y() * 32.0) as i32,
        z: (self.pos.z() * 32.0) as i32,
        velocity_x: self.vel_x.into(),
        velocity_y: self.vel_y.into(),
        velocity_z: self.vel_z.into(),
        yaw: self.yaw,
        pitch: self.pitch,
        head_pitch: self.head_yaw,
        unknown: match metadata(self.ty, &self.meta, ver, conn.conv()) {
          Some(m) => m,
          None => return Ok(smallvec![]),
        },
      })
    }
  } else {
    // Handle non-living mobs for 1.8-1.18
    let spawn = if ver >= ProtocolVersion::V1_14_4 {
      let mut data = vec![];
      let mut buf = Buffer::new(&mut data);
      buf.write_varint(ty);
      buf.write_f64(self.pos.x());
      buf.write_f64(self.pos.y());
      buf.write_f64(self.pos.z());
      buf.write_i8(self.pitch);
      buf.write_i8(self.yaw);
      buf.write_i32(self.data);
      buf.write_i16(self.vel_x);
      buf.write_i16(self.vel_y);
      buf.write_i16(self.vel_z);
      gpacket!(SpawnObject V14 { id: self.eid, uuid :self.id, unknown: data })
    } else if ver >= ProtocolVersion::V1_9 {
      gpacket!(SpawnObject V9 {
        entity_id: self.eid,
        unique_id: self.id,
        ty:        super::object_ty(ty),
        x:         self.pos.x(),
        y:         self.pos.y(),
        z:         self.pos.z(),
        yaw:       self.yaw.into(),
        pitch:     self.pitch.into(),
        speed_x:   self.vel_x.into(),
        speed_y:   self.vel_y.into(),
        speed_z:   self.vel_z.into(),
        data:      self.data,
      })
    } else {
      let mut data = vec![];
      let mut buf = Buffer::new(&mut data);
      buf.write_i16(self.vel_x);
      buf.write_i16(self.vel_y);
      buf.write_i16(self.vel_z);
      gpacket!(SpawnObject V8 {
        entity_id:      self.eid,
        ty:             super::object_ty(ty),
        x:              (self.pos.x() * 32.0) as i32,
        y:              (self.pos.y() * 32.0) as i32,
        z:              (self.pos.z() * 32.0) as i32,
        yaw:            self.yaw.into(),
        pitch:          self.pitch.into(),
        field_149020_k: self.data,
        unknown:        data,
      })
    };
    if !self.meta.fields.is_empty() {
      match metadata(self.ty, &self.meta, ver, conn.conv()) {
        Some(data) => {
          return Ok(smallvec![
            spawn,
            gpacket!(EntityMetadata V8 { entity_id: self.eid, unknown: data })
          ])
        }
        None => spawn,
      }
    } else {
      spawn
    }
  }
});
to_tcp!(SpawnPlayer => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_15_2 {
    let spawn = gpacket!(SpawnPlayer V15 {
      id:    self.eid,
      uuid:  self.id,
      x:     self.pos.x(),
      y:     self.pos.y(),
      z:     self.pos.z(),
      yaw:   self.yaw,
      pitch: self.pitch,
    });
    if !self.meta.fields.is_empty() {
      match metadata(self.ty, &self.meta, ver, conn.conv()) {
        Some(data) => {
          return Ok(smallvec![
            spawn,
            gpacket!(EntityMetadata V8 { entity_id: self.eid, unknown: data })
          ])
        }
        None => spawn,
      }
    } else {
      spawn
    }
  } else if ver >= ProtocolVersion::V1_9_4 {
    gpacket!(SpawnPlayer V9 {
      entity_id: self.eid,
      unique_id: self.id,
      x:         self.pos.x(),
      y:         self.pos.y(),
      z:         self.pos.z(),
      yaw:       self.yaw,
      pitch:     self.pitch,
      unknown:   match metadata(self.ty, &self.meta, ver, conn.conv()) {
        Some(m) => m,
        None => return Ok(smallvec![]),
      },
    })
  } else {
    gpacket!(SpawnPlayer V8 {
      entity_id:    self.eid,
      player_id:    self.id,
      x:            (self.pos.x() * 32.0) as i32,
      y:            (self.pos.y() * 32.0) as i32,
      z:            (self.pos.z() * 32.0) as i32,
      yaw:          self.yaw,
      pitch:        self.pitch,
      current_item: 0,
      unknown:      match metadata(self.ty, &self.meta, ver, conn.conv()) {
        Some(m) => m,
        None => return Ok(smallvec![]),
      },
    })
  }
});
to_tcp_manual!(Tags => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_14_4 {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(5);
    macro_rules! tag {
      ( $name:expr, $tag:expr ) => {
        buf.write_str($name);
        buf.write_varint($tag.len() as i32);
        for (name, tag) in &$tag {
          buf.write_str(name);
          buf.write_varint(tag.len() as i32);
          for elem in tag {
            buf.write_varint(*elem);
          }
        }
      };
    }
    tag!("minecraft:block", self.block);
    tag!("minecraft:item", self.item);
    tag!("minecraft:fluid", self.fluid);
    tag!("minecraft:entity_type", self.entity_type);
    tag!("minecraft:game_event", self.game_event);
    // gpacket!(SynchronizeTagsV14 { unknown: data }
    Ok(smallvec![])
  } else {
    Err(WriteError::InvalidVer)
  }
});
to_tcp!(Title => (self, conn, ver) {
  use bb_common::net::cb::TitleAction;

  if ver >= ProtocolVersion::V1_17_1 {
    match self.action {
      TitleAction::Title(chat) => gpacket!(Title V17 { title: chat.to_json() }),
      TitleAction::Subtitle(chat) => gpacket!(Subtitle V17 { subtitle: chat.to_json() }),
      TitleAction::Times { fade_in, stay, fade_out } => gpacket!(TitleFade V17 {
        fade_in_ticks:  fade_in as i32,
        remain_ticks:   stay as i32,
        fade_out_ticks: fade_out as i32,
      }),
      TitleAction::Clear(reset) => gpacket!(ClearTitle V17 { reset }),
    }
  } else {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    match self.action {
      TitleAction::Title(ref chat) => buf.write_str(&chat.to_json()),
      TitleAction::Subtitle(ref chat) => buf.write_str(&chat.to_json()),
      TitleAction::Times { fade_in, stay, fade_out } => {
        buf.write_i32(fade_in as i32);
        buf.write_i32(stay as i32);
        buf.write_i32(fade_out as i32);
      }
      _ => {}
    }
    gpacket!(Title V8 {
      ty:      if ver >= ProtocolVersion::V1_12_2 {
        match self.action {
          TitleAction::Title(_) => 0,
          TitleAction::Subtitle(_) => 1,
          TitleAction::Times { .. } => 3,
          TitleAction::Clear(false) => 4,
          TitleAction::Clear(true) => 5,
        }
      } else {
        match self.action {
          TitleAction::Title(_) => 0,
          TitleAction::Subtitle(_) => 1,
          TitleAction::Times { .. } => 2,
          TitleAction::Clear(false) => 3,
          TitleAction::Clear(true) => 4,
        }
      },
      unknown: data,
    })
  }
});
to_tcp!(Teams => (self, conn, ver) {
  use bb_common::net::cb::{TeamAction, TeamInfo, TeamRule};

  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  fn write_entities(buf: &mut Buffer<&mut Vec<u8>>, entities: &[String]) {
    buf.write_list(entities, |buf, n| buf.write_str(n.as_str()));
  }
  fn write_info(ver: ProtocolVersion, buf: &mut Buffer<&mut Vec<u8>>, info: &TeamInfo) {
    if ver >= ProtocolVersion::V1_14_4 {
      buf.write_str(&info.display_name.to_json());
      buf.write_u8(
        if info.friendly_fire { 0x01 } else { 0x00 }
          | if info.see_invis { 0x02 } else { 0x00 },
      );
      buf.write_str(match info.name_tag {
        TeamRule::Always => "always",
        TeamRule::ForOtherTeams => "hideForOtherTeams",
        TeamRule::ForOwnTeam => "hideForOwnTeam",
        TeamRule::Never => "never",
      });
      buf.write_str(match info.collisions {
        TeamRule::Always => "always",
        TeamRule::ForOtherTeams => "pushrOtherTeams",
        TeamRule::ForOwnTeam => "pushOwnTeam",
        TeamRule::Never => "never",
      });
      buf.write_varint(info.color.id().into());
      buf.write_str(&info.prefix.to_json());
      buf.write_str(&info.postfix.to_json());
    } else if ver >= ProtocolVersion::V1_9_4 {
      buf.write_str(&info.display_name.to_codes());
      // Team colors are broken. This code makes titles match the functionality of
      // 1.14+ clients.
      let mut prefix = info.prefix.to_codes();
      prefix.push(chat::CODE_SEP);
      prefix.push(info.color.code());
      buf.write_str(&prefix);
      buf.write_str(&info.postfix.to_codes());
      buf.write_u8(
        if info.friendly_fire { 0x01 } else { 0x00 }
          | if info.see_invis { 0x02 } else { 0x00 },
      );
      buf.write_str(match info.name_tag {
        TeamRule::Always => "always",
        TeamRule::ForOtherTeams => "hideForOtherTeams",
        TeamRule::ForOwnTeam => "hideForOwnTeam",
        TeamRule::Never => "never",
      });
      buf.write_str(match info.collisions {
        TeamRule::Always => "always",
        TeamRule::ForOtherTeams => "pushrOtherTeams",
        TeamRule::ForOwnTeam => "pushOwnTeam",
        TeamRule::Never => "never",
      });
      // This is pointless, as the client will never render it. But theres no real
      // reason not to send it.
      buf.write_varint(info.color.id().into());
    } else {
      buf.write_str(&info.display_name.to_codes());
      // Team colors are broken. This code makes titles match the functionality of
      // 1.14+ clients.
      let mut prefix = info.prefix.to_codes();
      prefix.push(chat::CODE_SEP);
      prefix.push(info.color.code());
      buf.write_str(&prefix);
      buf.write_str(&info.postfix.to_codes());
      buf.write_u8(
        if info.friendly_fire { 0x01 } else { 0x00 }
          | if info.see_invis { 0x02 } else { 0x00 },
      );
      buf.write_str(match info.name_tag {
        TeamRule::Always => "always",
        TeamRule::ForOtherTeams => "hideForOtherTeams",
        TeamRule::ForOwnTeam => "hideForOwnTeam",
        TeamRule::Never => "never",
      });
      // This is pointless, as the client will never render it. But theres no real
      // reason not to send it.
      buf.write_u8(info.color.id());
    }
  }
  match &self.action {
    TeamAction::Create { info, entities } => {
      write_info(ver, &mut buf, info);
      write_entities(&mut buf, entities);
    }
    TeamAction::Remove => {}
    TeamAction::UpdateInfo { info } => {
      write_info(ver, &mut buf, info);
    }
    TeamAction::AddEntities { entities } => {
      write_entities(&mut buf, entities);
    }
    TeamAction::RemoveEntities { entities } => {
      write_entities(&mut buf, entities);
    }
  }
  let ty = match self.action {
    TeamAction::Create { .. } => 0,
    TeamAction::Remove => 1,
    TeamAction::UpdateInfo { .. } => 2,
    TeamAction::AddEntities { .. } => 3,
    TeamAction::RemoveEntities { .. } => 4,
  };
  if ver >= ProtocolVersion::V1_18 {
    gpacket!(Teams V18 { packet_type: ty, team_name: self.team, unknown: data })
  } else if ver >= ProtocolVersion::V1_17_1 {
    gpacket!(Teams V17 { packet_type: ty, team_name: self.team, unknown: data })
  } else {
    gpacket!(Teams V8 { field_149314_f: ty, field_149320_a: self.team, unknown: data })
  }
});
to_tcp!(UnloadChunk => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_9 {
    gpacket!(UnloadChunk V9 { x: self.pos.x(), z: self.pos.z() })
  } else {
    gpacket!(ChunkData V8 {
      chunk_x:        self.pos.x(),
      chunk_z:        self.pos.z(),
      field_149279_g: true,
      // Zero bit mask, then zero length varint
      unknown:        vec![0, 0, 0],
    })
  }
});
to_tcp!(UpdateHealth => (self, conn, _ver) {
  gpacket!(UpdateHealth V8 {
    health: self.health,
    food_level: self.food,
    saturation_level: self.saturation,
  })
});
to_tcp!(UpdateViewPos => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_14 {
    gpacket!(ChunkRenderDistanceCenter V14 { chunk_x: self.pos.x(), chunk_z: self.pos.z() })
  } else {
    return Err(WriteError::InvalidVer);
  }
});
to_tcp!(WindowOpen => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_14_4 {
    let id = match self.ty.as_str() {
      "minecraft:generic_9x1" => 0,
      "minecraft:generic_9x2" => 1,
      "minecraft:generic_9x3" => 2,
      "minecraft:generic_9x4" => 3,
      "minecraft:generic_9x5" => 4,
      "minecraft:generic_9x6" => 5,
      "minecraft:generic_3x3" => 6,
      "minecraft:anvil" => 7,
      "minecraft:beacon" => 8,
      "minecraft:blast_furnace" => 9,
      "minecraft:brewing_stand" => 10,
      "minecraft:crafting" => 11,
      "minecraft:enchantment" => 12,
      "minecraft:furnace" => 13,
      "minecraft:grindstone" => 14,
      "minecraft:hopper" => 15,
      "minecraft:lectern" => 16,
      "minecraft:loom" => 17,
      "minecraft:merchant" => 18,
      "minecraft:shulker_box" => 19,
      "minecraft:smithing" => 20,
      "minecraft:smoker" => 21,
      "minecraft:cartography" => 22,
      "minecraft:stonecutter" => 23,
      _ => 0,
    };
    if ver >= ProtocolVersion::V1_16_5 {
      gpacket!(OpenScreen V16 {
        sync_id:           self.wid.into(),
        screen_handler_id: id,
        name:              self.title,
      })
    } else {
      gpacket!(OpenWindow V14 {
        sync_id:      self.wid.into(),
        container_id: id,
        name:         self.title,
      })
    }
  } else {
    gpacket!(OpenWindow V8 {
      window_id:      self.wid.into(),
      inventory_type: self.ty,
      window_title:   self.title,
      slot_count:     self.size as i32,
      unknown:        vec![],
    })
  }
});
to_tcp!(WindowItems => (self, conn, ver) {
  if ver >= ProtocolVersion::V1_17_1 {
    let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
    buf.write_varint(self.items.len() as i32);
    for mut it in self.items {
      conn.conv().item(&mut it, ver.block());
      buf.write_item(&it, conn.conv());
    }
    buf.write_item(&self.held, conn.conv());
    gpacket!(WindowItems V17 { sync_id: self.wid.into(), revision: 0, unknown: buf.serialize() })
  } else {
    let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
    buf.write_i16(self.items.len() as i16);
    for mut it in self.items {
      conn.conv().item(&mut it, ver.block());
      buf.write_item(&it, conn.conv());
    }
    gpacket!(WindowItems V8 { window_id: self.wid.into(), unknown: buf.serialize(), v_2: 0 })
  }
});
to_tcp!(WindowItem => (mut self, conn, ver) {
  let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
  conn.conv().item(&mut self.item, ver.block());
  buf.write_item(&self.item, conn.conv());
  if ver >= ProtocolVersion::V1_17_1 {
    gpacket!(SetSlot V17 {
      sync_id: self.wid.into(),
      revision: 0,
      slot: self.slot,
      unknown: buf.serialize(),
    })
  } else {
    gpacket!(SetSlot V8 {
      window_id: self.wid.into(),
      slot: self.slot,
      unknown: buf.serialize(),
    })
  }
});
