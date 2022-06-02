#[macro_use]
extern crate log;

use bb_proxy::stream::java::JavaStream;
use conn::ConnStream;
use crossterm::{execute, terminal};
use mio::{net::TcpStream, Events, Interest, Poll, Token, Waker};
use parking_lot::Mutex;
use std::{env, error::Error, io, sync::Arc, thread};

mod cli;
mod command;
mod conn;
mod handle;
mod status;

fn main() {
  let (_cols, rows) = terminal::size().unwrap();
  cli::setup().unwrap();
  bb_common::init_with_writer("cli", cli::skip_appender(15, rows - 30));
  match run(rows) {
    Ok(_) => (),
    Err(e) => {
      terminal::disable_raw_mode().unwrap();
      execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
      error!("error: {}", e);
      std::process::exit(1);
    }
  };
}

fn run(rows: u16) -> Result<(), Box<dyn Error>> {
  let mut args = env::args();
  args.next(); // current process
  let ip = args.next().unwrap_or_else(|| "127.0.0.1:25565".into());

  // let ver = ProtocolVersion::V1_8;

  info!("connecting to {}", ip);
  let mut stream = TcpStream::connect(ip.parse()?)?;
  info!("connection established");

  const WAKE_TOKEN: Token = Token(0xfffffffd);

  let mut poll = Poll::new()?;
  let mut events = Events::with_capacity(1024);
  // let (needs_flush_tx, needs_flush_rx) = crossbeam_channel::bounded(1024);

  let waker = Arc::new(Waker::new(poll.registry(), WAKE_TOKEN)?);

  poll.registry().register(&mut stream, Token(0), Interest::READABLE | Interest::WRITABLE)?;

  let mut conn = ConnStream::new(JavaStream::new(stream));
  conn.start_handshake();
  let conn = Arc::new(Mutex::new(conn));

  let status = Arc::new(Mutex::new(status::Status::new()));
  status::Status::enable_drawing(status.clone());

  // let mut next_token = 0;

  let c = conn.clone();
  let w = waker;
  thread::spawn(move || {
    let mut lr = cli::LineReader::new("> ", rows - 15, 15);
    while let Ok(line) = lr.read_line() {
      if line.is_empty() {
        continue;
      }
      let mut sections = line.split(' ');
      let command = sections.next().unwrap();
      let args: Vec<_> = sections.collect();
      let mut conn = c.lock();
      match command::handle(command, &args, &mut conn, &mut lr) {
        Ok(_) => {}
        Err(e) => {
          error!("error handling command: {}", e);
        }
      }
      w.wake().unwrap();
    }
  });

  loop {
    // Wait for events
    poll.poll(&mut events, None)?;

    for event in &events {
      let tok = event.token();

      let mut closed = false;
      let mut conn = conn.lock();
      if event.is_readable() && event.token() != WAKE_TOKEN {
        // let conn = clients.get_mut(&token).expect("client doesn't exist!");
        loop {
          match conn.poll() {
            Ok(_) => {
              // If we got a good poll result, then there can be any number of packets in the
              // internal buffer.
              loop {
                match conn.read() {
                  Ok(p) => {
                    if conn.closed() {
                      closed = true;
                      break;
                    } else if let Some(p) = p {
                      handle::handle_packet(&mut conn, &status, p)?;
                    } else {
                      // We are done reading packet from the internal buffer
                      break;
                    }
                  }
                  Err(e) => {
                    error!("error while parsing packet from client {:?}: {}", tok, e);
                    closed = true;
                    break;
                  }
                }
              }
              if closed {
                break;
              }
            }
            Err(ref e) if e.is_would_block() => break,
            Err(e) => {
              error!("error while listening to client {:?}: {}", tok, e);
              closed = true;
              break;
            }
          }
        }
      }
      // The order here is important. If we are handshaking, then reading a packet
      // will probably prompt a direct response. In this arrangement, we can send more
      // packets before going back to poll().
      if (event.is_writable() || event.token() == WAKE_TOKEN) && !closed {
        // let conn = clients.get_mut(&token).expect("client doesn't exist!");
        while conn.needs_flush() {
          match conn.flush() {
            Ok(_) => {}
            Err(ref e) if e.is_would_block() => break,
            Err(e) => {
              error!("error while flushing packets to the client {:?}: {}", tok, e);
              break;
            }
          }
        }
      }
    }
  }

  // handle::handshake(&mut reader, &mut writer, ver).await?;
  // info!("login complete");
  //
  // let reader = ConnReader { stream: reader, ver };
  // let writer = Arc::new(Mutex::new(ConnWriter { stream: writer, ver }));
  // let status = Arc::new(Mutex::new(status::Status::new()));
  // status::Status::enable_drawing(status.clone());
  //
  // let w = writer.clone();
  // let s = status.clone();
  // tokio::spawn(async move {
  //   let mut handler = handle::Handler { reader, writer: w, status: s };
  //   match handler.run().await {
  //     Ok(_) => warn!("handler exited"),
  //     Err(e) => error!("handler error: {}", e),
  //   }
  // });

  // info!("closing");
  //
  // Ok(())
}
