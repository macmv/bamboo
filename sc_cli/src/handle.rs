use super::{conn::ConnStream, status::Status};
use parking_lot::Mutex;
use sc_common::{
  math::ChunkPos,
  net::{cb, sb},
  util::Chat,
};
use std::time::Instant;

pub fn handle_packet(stream: &mut ConnStream, status: &Mutex<Status>, p: cb::Packet) {
  match p {
    cb::Packet::Login { .. } => {}
    cb::Packet::Chat { message, .. } => match Chat::from_json(message) {
      Ok(m) => info!("chat: {}", m.to_plain()),
      Err(e) => warn!("invalid chat: {}", e),
    },
    cb::Packet::KickDisconnect { reason } => {
      error!("disconnected: {}", reason);
      // TODO: disconnect
    }
    cb::Packet::KeepAlive { keep_alive_id_v1_8, keep_alive_id_v1_12_2 } => {
      stream.write(sb::Packet::KeepAlive { keep_alive_id_v1_8, keep_alive_id_v1_12_2 });
      status.lock().last_keep_alive = Instant::now();
    }
    cb::Packet::MapChunk { x, z, .. } => {
      let mut lock = status.lock();
      let pos = ChunkPos::new(x, z);
      if lock.loaded_chunks.contains(&pos) {
        warn!("leaking chunk at {:?}", pos);
      }
      lock.loaded_chunks.insert(pos);
    }
    cb::Packet::PlayerlistHeader { header, footer } => {
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
}
