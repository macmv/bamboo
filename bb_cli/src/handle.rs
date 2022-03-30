use super::{conn::ConnStream, status::Status};
use parking_lot::Mutex;
use bb_common::{
  math::ChunkPos,
  util::{Buffer, Chat},
};
use bb_proxy::{
  gnet::{cb, sb},
  Result,
};
use std::time::Instant;

pub fn handle_packet(stream: &mut ConnStream, status: &Mutex<Status>, p: cb::Packet) -> Result<()> {
  match p {
    cb::Packet::JoinGameV8 { .. } => {}
    cb::Packet::ChatV8 { chat_component, .. } => match Chat::from_json(chat_component) {
      Ok(m) => info!("chat: {}", m.to_plain()),
      Err(e) => warn!("invalid chat: {}", e),
    },
    cb::Packet::DisconnectV8 { reason } => {
      error!("disconnected: {}", reason);
      // TODO: disconnect
    }
    cb::Packet::KeepAliveV8 { id } => {
      stream.write(sb::Packet::KeepAliveV8 { key: id });
      status.lock().last_keep_alive = Instant::now();
    }
    cb::Packet::ChunkDataV8 { chunk_x, chunk_z, unknown, .. } => {
      let mut lock = status.lock();
      let pos = ChunkPos::new(chunk_x, chunk_z);
      let mut buf = Buffer::new(unknown);
      let bit_map = buf.read_u16()?;
      let len = buf.read_varint()?;
      if bit_map == 0 && len == 0 {
        lock.loaded_chunks.remove(&pos);
      } else {
        if lock.loaded_chunks.contains(&pos) {
          warn!("leaking chunk at {:?}", pos);
        }
        lock.loaded_chunks.insert(pos);
      }
    }
    cb::Packet::PlayerListHeaderV8 { header, footer } => {
      let mut lock = status.lock();
      match Chat::from_json(header) {
        Ok(m) => lock.header = m.to_plain().replace('\n', ""),
        Err(e) => warn!("invalid header: {}", e),
      }
      match Chat::from_json(footer) {
        Ok(m) => lock.footer = m.to_plain().replace('\n', ""),
        Err(e) => warn!("invalid footer: {}", e),
      }
    }
    p => warn!("unhandled packet {}...", &format!("{:?}", p)[..40]),
  }
  Ok(())
}
