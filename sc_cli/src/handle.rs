use super::{status::Status, ConnReader, ConnWriter};
use rand::{rngs::OsRng, Rng};
use rsa::PublicKey;
use sc_common::{
  math::{der, ChunkPos},
  net::{cb, sb, tcp},
  util::Chat,
  version::ProtocolVersion,
};
use sc_proxy::{self, conn::State};
use std::{error::Error, io, sync::Arc, time::Instant};
use tokio::sync::Mutex;

pub struct Handler {
  pub reader: ConnReader,
  pub writer: Arc<Mutex<ConnWriter>>,
  pub status: Arc<Mutex<Status>>,
}

impl Handler {
  pub async fn run(&mut self) -> Result<(), io::Error> {
    'all: loop {
      self.reader.poll().await?;

      loop {
        let p = match self.reader.read()? {
          None => break,
          Some(p) => p,
        };

        match p {
          cb::Packet::Login { .. } => {}
          cb::Packet::Chat { message, .. } => match Chat::from_json(message) {
            Ok(m) => info!("chat: {}", m.to_plain()),
            Err(e) => warn!("invalid chat: {}", e),
          },
          cb::Packet::KickDisconnect { reason } => {
            error!("disconnected: {}", reason);
            break 'all;
          }
          cb::Packet::KeepAlive { keep_alive_id_v1_8, keep_alive_id_v1_12_2 } => {
            self.send(sb::Packet::KeepAlive { keep_alive_id_v1_8, keep_alive_id_v1_12_2 }).await?;
            self.status.lock().await.last_keep_alive = Instant::now();
          }
          cb::Packet::MapChunk { x, z, .. } => {
            let mut lock = self.status.lock().await;
            let pos = ChunkPos::new(x, z);
            if lock.loaded_chunks.contains(&pos) {
              warn!("leaking chunk at {:?}", pos);
            }
            lock.loaded_chunks.insert(pos);
          }
          cb::Packet::PlayerlistHeader { header, footer } => {
            let mut lock = self.status.lock().await;
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
    }
    Ok(())
  }

  async fn send(&self, p: sb::Packet) -> Result<(), io::Error> {
    let mut lock = self.writer.lock().await;
    lock.write(p).await?;
    lock.flush().await?;
    Ok(())
  }
}

pub async fn handshake(
  reader: &mut JavaStreamReader,
  writer: &mut JavaStreamWriter,
  ver: ProtocolVersion,
) -> Result<(), Box<dyn Error>> {
  let mut out = tcp::Packet::new(0, ver);
  out.write_varint(ProtocolVersion::V1_8.id() as i32);
  out.write_str("127.0.0.1");
  out.write_u16(25565);
  out.write_varint(2); // login state
  writer.write(out).await?;
  let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8);
  out.write_str("macmv");
  writer.write(out).await?;
  writer.flush().await?;

  let state = State::Login;

  loop {
    reader.poll().await?;

    loop {
      let mut p = match reader.read(ProtocolVersion::V1_8)? {
        None => break,
        Some(p) => p,
      };
      match state {
        State::Handshake => unreachable!(),
        State::Status => unreachable!(),
        State::Login => match p.id() {
          0 => {
            // disconnect
            let reason = p.read_str();
            error!("disconnected: {}", reason);
            return Ok(());
          }
          1 => {
            // encryption request
            warn!("got encryption request, but mojang auth is not implemented");

            let _server_id = p.read_str();
            let pub_key_len = p.read_varint();
            let pub_key = p.read_buf(pub_key_len);
            let token_len = p.read_varint();
            let token = p.read_buf(token_len);
            let key = der::decode(&pub_key).unwrap();

            let mut secret = [0; 16];
            let mut rng = OsRng;
            rng.fill(&mut secret);

            let enc_secret = key.encrypt(&mut rng, rsa::PaddingScheme::PKCS1v15Encrypt, &secret)?;
            let enc_token = key.encrypt(&mut rng, rsa::PaddingScheme::PKCS1v15Encrypt, &token)?;

            let mut out = tcp::Packet::new(1, ProtocolVersion::V1_8);
            out.write_varint(enc_secret.len() as i32);
            out.write_buf(&enc_secret);
            out.write_varint(enc_token.len() as i32);
            out.write_buf(&enc_token);
            writer.write(out).await?;
            writer.flush().await?;

            reader.enable_encryption(&secret);
            writer.enable_encryption(&secret);
          }
          2 => {
            // login success
            let _uuid = p.read_uuid();
            let _username = p.read_str();
            return Ok(());
          }
          3 => {
            // set compression
            let thresh = p.read_varint();
            writer.set_compression(thresh);
            reader.set_compression(thresh);
          }
          _ => unreachable!(),
        },
        State::Play => unreachable!(),
        State::Invalid => unreachable!(),
      }
    }
  }
}
