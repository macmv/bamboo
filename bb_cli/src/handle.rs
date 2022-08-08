use super::{conn::ConnStream, status::Status};
use bb_common::{
  math::ChunkPos,
  util::{Buffer, Chat},
};
use bb_proxy::{
  gnet::{cb, sb},
  Result,
};
use parking_lot::Mutex;
use std::time::Instant;

pub fn handle_packet(stream: &mut ConnStream, status: &Mutex<Status>, p: cb::Packet) -> Result<()> {
  match p {
    cb::Packet::JoinGame(cb::packet::JoinGame::V8(_)) => {}
    cb::Packet::Chat(cb::packet::Chat::V8(p)) => match Chat::from_json(&p.chat_component) {
      Ok(m) => {
        if p.ty == 0 {
          info!("chat: {}", m.to_plain())
        } else {
          status.lock().hotbar = m.to_plain();
        }
      }
      Err(e) => warn!("invalid chat: {}", e),
    },
    cb::Packet::Disconnect(cb::packet::Disconnect::V8(p)) => {
      error!("disconnected: {}", p.reason);
      // TODO: disconnect
    }
    cb::Packet::KeepAlive(cb::packet::KeepAlive::V8(p)) => {
      stream.send(sb::packet::KeepAliveV8 { key: p.id });
      status.lock().last_keep_alive = Instant::now();
    }
    cb::Packet::ChunkData(cb::packet::ChunkData::V8(p)) => {
      let mut lock = status.lock();
      let pos = ChunkPos::new(p.chunk_x, p.chunk_z);
      let mut buf = Buffer::new(p.unknown);
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
    cb::Packet::PlayerListHeader(cb::packet::PlayerListHeader::V8(p)) => {
      let mut lock = status.lock();
      match Chat::from_json(&p.header) {
        Ok(m) => lock.header = m.to_plain().replace('\n', ""),
        Err(e) => warn!("invalid header: {}", e),
      }
      match Chat::from_json(&p.footer) {
        Ok(m) => lock.footer = m.to_plain().replace('\n', ""),
        Err(e) => warn!("invalid footer: {}", e),
      }
    }
    cb::Packet::Particle(cb::packet::Particle::V8(_p)) => {}
    p => warn!("unhandled packet {}...", &format!("{:?}", p)[..40]),
  }
  Ok(())
}
