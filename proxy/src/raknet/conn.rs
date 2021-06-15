use std::{io, net::UdpSocket};

pub struct RakNetConn {
  sock: UdpSocket,
}
