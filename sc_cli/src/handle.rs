use super::{ConnReader, ConnWriter};
use sc_common::net::{cb, sb};
use std::{io, sync::Arc};
use tokio::sync::Mutex;

pub struct Handler {
  pub reader: ConnReader,
  pub writer: Arc<Mutex<ConnWriter>>,
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
          cb::Packet::Login { .. } => {
            let mut w = self.writer.lock().await;
            w.write(sb::Packet::Chat { message: "hello world!".into() }).await?;
            w.flush().await?;
          }
          cb::Packet::KickDisconnect { reason } => {
            error!("disconnected: {}", reason);
            break 'all;
          }
          p => warn!("unhandled packet {}...", &format!("{:?}", p)[..40]),
        }
      }
    }
    Ok(())
  }
}
