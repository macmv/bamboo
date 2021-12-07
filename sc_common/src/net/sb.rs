#[derive(Debug, Clone, sc_macros::Transfer)]
pub enum Packet {
  Chat {
    msg: String,
  },
  KeepAlive {
    id: i32,
  },
  PlayerOnGround {
    on_ground: bool,
  },
  PlayerLook {
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
  PlayerPos {
    x:         f64,
    y:         f64,
    z:         f64,
    on_ground: bool,
  },
  PlayerPosLook {
    x:         f64,
    y:         f64,
    z:         f64,
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
}
