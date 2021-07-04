use super::Generator;

use std::io::{Error, ErrorKind, Result};

use common::{
  net::{cb, tcp, Other},
  proto::player_list,
  util::{Buffer, UUID},
  version::ProtocolVersion,
};

// Same for all versions
pub(super) fn generate_player_info(
  gen: &Generator,
  v: ProtocolVersion,
  p: &cb::Packet,
) -> Result<Vec<tcp::Packet>> {
  let mut out = tcp::Packet::new(gen.convert_id(v, p.id()), v);
  let info = match p.read_other().unwrap() {
    Other::PlayerList(c) => c,
    o => {
      return Err(Error::new(ErrorKind::InvalidData, format!("expected player list, got {:?}", o)))
    }
  };
  out.write_varint(info.action);
  out.write_varint(info.players.len() as i32);
  for p in info.players {
    out.write_uuid(match p.uuid {
      Some(v) => UUID::from_proto(v),
      None => return Err(Error::new(ErrorKind::InvalidData, "empty player uuid in player list")),
    });
    match player_list::Action::from_i32(info.action).unwrap() {
      player_list::Action::AddPlayer => {
        out.write_str(&p.name);
        out.write_varint(p.properties.len() as i32);
        for p in p.properties {
          out.write_str(&p.name);
          out.write_str(&p.value);
          out.write_bool(p.signed);
          if p.signed {
            out.write_str(&p.signature);
          }
        }
        out.write_varint(p.gamemode);
        out.write_varint(p.ping);
        out.write_bool(p.has_display_name);
        if p.has_display_name {
          out.write_str(&p.display_name);
        }
      }
      player_list::Action::UpdateGamemode => {
        out.write_varint(p.gamemode);
      }
      player_list::Action::UpdateLatency => {
        out.write_varint(p.ping);
      }
      player_list::Action::UpdateDisplayName => {
        out.write_bool(p.has_display_name);
        if p.has_display_name {
          out.write_str(&p.display_name);
        }
      }
      player_list::Action::RemovePlayer => {
        // No fields
      }
    }
  }
  Ok(vec![out])
}

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
pub(super) fn generate_1_9_chunk(
  gen: &Generator,
  v: ProtocolVersion,
  p: &cb::Packet,
) -> Result<Vec<tcp::Packet>> {
  // TODO: Error handling should be done within the packet.
  let mut out = tcp::Packet::new(gen.convert_id(v, p.id()), v);
  let chunk = match p.read_other().unwrap() {
    Other::Chunk(c) => c,
    o => return Err(Error::new(ErrorKind::InvalidData, format!("expected chunk, got {:?}", o))),
  };
  out.write_i32(chunk.x);
  out.write_i32(chunk.z);
  out.write_bool(true); // Always a new chunk

  let biomes = true; // Always true with new chunk set
  let skylight = true; // Assume overworld

  let mut bitmask = 0;
  for y in chunk.sections.keys() {
    bitmask |= 1 << y;
  }
  out.write_varint(bitmask);

  let mut buf = Buffer::new(vec![]);
  // Makes an ordered list of chunk sections
  let mut sections = vec![None; 16];
  for (y, s) in &chunk.sections {
    sections[*y as usize] = Some(s);
  }
  // Iterates through chunks in order, from ground up. flatten() skips all None
  // sections.
  for s in sections.into_iter().flatten() {
    // The bits per block
    buf.write_u8(s.bits_per_block as u8);
    // The length of the palette
    buf.write_varint(s.palette.len() as i32);
    for g in &s.palette {
      buf.write_varint(*g as i32);
    }
    // Number of longs in the data array
    buf.write_varint(s.data.len() as i32);
    buf.write_buf(&s.data.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());
    // Light data
    for _ in 0..16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      buf.write_u8(0xff);
    }
    if skylight {
      for _ in 0..16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        buf.write_u8(0xff);
      }
    }
  }

  if biomes {
    for _ in 0..256 {
      buf.write_u8(127); // Void biome
    }
  }

  // if chunk.x == 3 && chunk.z == 5 {
  //   println!("{:x?}", buf);
  // }

  out.write_varint(buf.len() as i32);
  out.write_buf(&buf);
  // No tile entities
  out.write_varint(0);

  Ok(vec![out])
}

pub(super) fn generate_declare_commands(
  gen: &Generator,
  v: ProtocolVersion,
  p: &cb::Packet,
) -> Result<Vec<tcp::Packet>> {
  let mut out = tcp::Packet::new(gen.convert_id(v, p.id()), v);
  // Command data is always the same, so we generate and cache it on the server.
  out.write_buf(p.get_byte_arr("data")?);
  out.write_varint(p.get_int("root")?);
  Ok(vec![out])
}
