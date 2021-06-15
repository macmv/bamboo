use super::RakNetConn;
use std::{io, net::UdpSocket};

pub struct RakNetListener {
  addr: String,
}

impl RakNetListener {
  pub fn bind<A: Into<String>>(addr: A) -> Self {
    RakNetListener { addr: addr.into() }
  }
  pub fn accept(&self) -> io::Result<RakNetConn> {
    let mut sock = UdpSocket::bind(self.addr)?;
    Ok(RakNetConn { sock })
  }
}
