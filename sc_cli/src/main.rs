#[macro_use]
extern crate log;

use crossterm::terminal;
use sc_common::{
  net::{cb, sb},
  version::ProtocolVersion,
};
use sc_proxy::stream::{
  java::{JavaStreamReader, JavaStreamWriter},
  StreamReader, StreamWriter,
};
use std::{error::Error, io, io::Write, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};

mod cli;
mod command;
mod handle;

pub struct ConnWriter {
  stream: JavaStreamWriter,
  ver:    ProtocolVersion,
}
pub struct ConnReader {
  stream: JavaStreamReader,
  ver:    ProtocolVersion,
}

impl ConnWriter {
  pub async fn write(&mut self, p: sb::Packet) -> Result<(), io::Error> {
    self.stream.write(p.to_tcp(self.ver)).await
  }

  pub async fn flush(&mut self) -> Result<(), io::Error> {
    self.stream.flush().await
  }
}
impl ConnReader {
  pub async fn poll(&mut self) -> Result<(), io::Error> {
    self.stream.poll().await
  }

  pub fn read(&mut self) -> Result<Option<cb::Packet>, io::Error> {
    Ok(self.stream.read(self.ver)?.map(|p| cb::Packet::from_tcp(p, self.ver)))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let (_cols, rows) = terminal::size()?;
  cli::setup()?;
  sc_common::init_with_stdout("cli", cli::skip_appender(15, rows - 30));

  let ver = ProtocolVersion::V1_8;

  let ip = "127.0.0.1:25565";
  info!("connecting to {}", ip);
  let stream = TcpStream::connect(ip).await?;
  info!("connection established");

  let (read, write) = stream.into_split();
  let mut reader = JavaStreamReader::new(read);
  let mut writer = JavaStreamWriter::new(write);

  handle::handshake(&mut reader, &mut writer, ver).await?;
  info!("login complete");

  let reader = ConnReader { stream: reader, ver };
  let writer = Arc::new(Mutex::new(ConnWriter { stream: writer, ver }));

  let w = writer.clone();
  tokio::spawn(async move {
    let mut handler = handle::Handler { reader, writer: w };
    handler.run().await.unwrap();
  });

  let mut lr = cli::LineReader::new("> ", rows - 15, 15);
  loop {
    match lr.read_line() {
      Ok(line) => {
        if line.is_empty() {
          continue;
        }
        let mut sections = line.split(' ');
        let command = sections.next().unwrap();
        let args: Vec<_> = sections.collect();
        let mut w = writer.lock().await;
        command::handle(command, &args, &mut w, &mut lr).await?;
      }
      Err(_) => break,
    }
  }

  info!("closing");

  Ok(())
}
