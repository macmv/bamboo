#[derive(sc_macros::Packet)]
pub enum Packet {
  Chunk { x: i32, z: i32, palette: Vec<u32>, blocks: Vec<u32> },
  KeepAlive { id: u32 },
}
