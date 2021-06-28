use super::Generator;

use std::io::Result;

use common::{
  net::{cb, tcp, Other},
  proto,
  proto::player_list,
  version::ProtocolVersion,
};

// Same for all versions
pub(super) fn generate_player_info(
  gen: &Generator,
  v: ProtocolVersion,
  p: &mut tcp::Packet,
) -> Result<Vec<cb::Packet>> {
  let mut player_list = proto::PlayerList::default();
  player_list.action = p.read_varint();
  for _ in 0..p.read_varint() {
    let mut player =
      proto::player_list::Player { uuid: Some(p.read_uuid().as_proto()), ..Default::default() };
    match player_list::Action::from_i32(player_list.action).unwrap() {
      player_list::Action::AddPlayer => {
        player.name = p.read_str();
        for _ in 0..p.read_varint() {
          let mut prop = proto::player_list::Property {
            name:      p.read_str(),
            value:     p.read_str(),
            signed:    p.read_bool(),
            signature: "".into(),
          };
          if prop.signed {
            prop.signature = p.read_str();
          }
          player.properties.push(prop);
        }
        player.gamemode = p.read_varint();
        player.ping = p.read_varint();
        player.has_display_name = p.read_bool();
        if player.has_display_name {
          player.display_name = p.read_str();
        }
      }
      player_list::Action::UpdateGamemode => {
        player.gamemode = p.read_varint();
      }
      player_list::Action::UpdateLatency => {
        player.ping = p.read_varint();
      }
      player_list::Action::UpdateDisplayName => {
        player.has_display_name = p.read_bool();
        if player.has_display_name {
          player.display_name = p.read_str();
        }
      }
      player_list::Action::RemovePlayer => {
        // No fields
      }
    }
    player_list.players.push(player);
  }
  let mut out = cb::Packet::new(gen.convert_id(v, p.id()));
  out.set_other(Other::PlayerList(player_list)).unwrap();
  Ok(vec![out])
}

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
// pub(super) fn generate_1_9_chunk(
//   gen: &Generator,
//   v: ProtocolVersion,
//   p: &cb::Packet,
// ) -> Result<Vec<tcp::Packet>> {
//   // TODO: Error handling should be done within the packet.
//   let mut out = tcp::Packet::new(gen.convert_id(v, p.id()), v);
//   let chunk = match p.read_other().unwrap() {
//     Other::Chunk(c) => c,
//     o => return Err(Error::new(ErrorKind::InvalidData, format!("expected
// chunk, got {:?}", o))),   };
//   out.write_i32(chunk.x);
//   out.write_i32(chunk.z);
//   out.write_bool(true); // Always a new chunk
//
//   let biomes = true; // Always true with new chunk set
//   let skylight = true; // Assume overworld
//
//   let mut bitmask = 0;
//   for y in chunk.sections.keys() {
//     bitmask |= 1 << y;
//   }
//   out.write_varint(bitmask);
//
//   let mut buf = Buffer::new(vec![]);
//   // Makes an ordered list of chunk sections
//   let mut sections = vec![None; 16];
//   for (y, s) in &chunk.sections {
//     sections[*y as usize] = Some(s);
//   }
//   // Iterates through chunks in order, from ground up. flatten() skips all
// None   // sections.
//   for s in sections.into_iter().flatten() {
//     // The bits per block
//     buf.write_u8(s.bits_per_block as u8);
//     // The length of the palette
//     buf.write_varint(s.palette.len() as i32);
//     for g in &s.palette {
//       buf.write_varint(*g as i32);
//     }
//     // Number of longs in the data array
//     buf.write_varint(s.data.len() as i32);
//     buf.write_buf(&s.data.iter().map(|v|
// v.to_be_bytes()).flatten().collect::<Vec<u8>>());     // Light data
//     for _ in 0..16 * 16 * 16 / 2 {
//       // Each lighting value is 1/2 byte
//       buf.write_u8(0xff);
//     }
//     if skylight {
//       for _ in 0..16 * 16 * 16 / 2 {
//         // Each lighting value is 1/2 byte
//         buf.write_u8(0xff);
//       }
//     }
//   }
//
//   if biomes {
//     for _ in 0..256 {
//       buf.write_u8(127); // Void biome
//     }
//   }
//
//   // if chunk.x == 3 && chunk.z == 5 {
//   //   println!("{:x?}", buf);
//   // }
//
//   out.write_varint(buf.len() as i32);
//   out.write_buf(&buf);
//   // No tile entities
//   out.write_varint(0);
//
//   Ok(vec![out])
// }
