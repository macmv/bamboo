use std::{io, net::UdpSocket, sync::Arc};

pub struct BedrockStreamReader {
  sock: Arc<UdpSocket>,
}

pub struct BedrockStreamWriter {
  sock: Arc<UdpSocket>,
}

impl BedrockStreamReader {
  pub fn new(sock: Arc<UdpSocket>) -> Self {
    BedrockStreamReader { sock }
  }
}

impl BedrockStreamWriter {
  pub fn new(sock: Arc<UdpSocket>) -> Self {
    BedrockStreamWriter { sock }
  }
}
