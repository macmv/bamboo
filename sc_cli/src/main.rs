#[macro_use]
extern crate log;

use crossterm::{execute, terminal};
use sc_common::{
  net::{cb, sb},
  version::ProtocolVersion,
};
use sc_proxy::stream::{
  java::{JavaStreamReader, JavaStreamWriter},
  StreamReader, StreamWriter,
};
use std::{env, error::Error, io, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};

mod cli;
mod command;
mod handle;
mod status;

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
async fn main() {
  let (_cols, rows) = terminal::size().unwrap();
  cli::setup().unwrap();
  sc_common::init_with_stdout("cli", cli::skip_appender(15, rows - 30));
  match run(rows).await {
    Ok(_) => (),
    Err(e) => {
      terminal::disable_raw_mode().unwrap();
      execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
      error!("error: {}", e);
      std::process::exit(1);
    }
  };
}

async fn run(rows: u16) -> Result<(), Box<dyn Error>> {
  let mut args = env::args();
  args.next(); // current process
  let ip = args.next().unwrap_or("127.0.0.1:25565".into());

  let ver = ProtocolVersion::V1_8;

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
  let status = Arc::new(Mutex::new(status::Status::new()));
  status::Status::enable_drawing(status.clone());

  let w = writer.clone();
  let s = status.clone();
  tokio::spawn(async move {
    let mut handler = handle::Handler { reader, writer: w, status: s };
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
