use std::{io, net::UdpSocket, sync::Arc};

pub struct StreamReader {
  sock: Arc<UdpSocket>,
}

pub struct StreamWriter {
  sock: Arc<UdpSocket>,
}

impl StreamReader {
  pub fn new(sock: Arc<UdpSocket>) -> Self {
    StreamReader { sock }
  }
}

impl StreamWriter {
  pub fn new(sock: Arc<UdpSocket>) -> Self {
    StreamWriter { sock }
  }
}
