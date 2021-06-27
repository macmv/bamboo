use std::net::TcpStream;

pub struct Connection {}

impl Connection {
  pub fn new(ip: &str) -> Option<Self> {
    info!("connecting to {}...", ip);
    let stream = match TcpStream::connect(ip) {
      Ok(s) => s,
      Err(e) => {
        error!("could not connect to {}: {}", ip, e);
        return None;
      }
    };

    Some(Connection {})
  }
}
