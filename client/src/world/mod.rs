use crate::{
  graphics::{game_vs, MeshChunk, Vert3, WindowData},
  net::Connection,
  player::{MainPlayer, OtherPlayer},
  ui::{LayoutKind, UI},
  Settings,
};
use cgmath::{Angle, Deg, Matrix4, Vector3};
use common::{math::ChunkPos, proto, util::UUID};
use std::{
  collections::HashMap,
  process,
  sync::{Arc, Mutex, RwLock},
  time::Instant,
};
use tokio::sync::Mutex as TokioMutex;

use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  device::Device,
};

pub struct World {
  chunks:      RwLock<HashMap<ChunkPos, Mutex<MeshChunk>>>,
  // This will be set whenever the player is in a game.
  main_player: Mutex<Option<MainPlayer>>,
  // List of other players. Does not include the main player.
  players:     HashMap<UUID, OtherPlayer>,
  vbuf:        Arc<CpuAccessibleBuffer<[Vert3]>>,
  start:       Instant,
  device:      Arc<Device>,
  settings:    TokioMutex<Settings>,
}

impl World {
  pub fn new(win: &WindowData) -> World {
    Self {
      chunks:      RwLock::new(HashMap::new()),
      main_player: Mutex::new(None),
      players:     HashMap::new(),
      vbuf:        CpuAccessibleBuffer::from_iter(
        win.device().clone(),
        BufferUsage::all(),
        false,
        [
          // Bottom face
          Vert3::new(1.0, 1.0, 0.0, 0.0, 0.0),
          Vert3::new(1.0, 0.0, 0.0, 0.0, 0.0),
          Vert3::new(0.0, 0.0, 0.0, 0.0, 0.0),
          Vert3::new(0.0, 0.0, 0.0, 0.0, 0.0),
          Vert3::new(0.0, 1.0, 0.0, 0.0, 0.0),
          Vert3::new(1.0, 1.0, 0.0, 0.0, 0.0),
        ]
        .iter()
        .cloned(),
      )
      .unwrap(),
      start:       Instant::now(),
      device:      win.device().clone(),
      settings:    TokioMutex::new(Settings::new()),
    }
  }

  /// This goes to the authentication servers, and makes sure that the user is
  /// logged in. If not, it will set the UI to the login screen.
  pub async fn login(&self) {
    let mut settings = self.settings.lock().await;
    if !settings.refresh_token().await {
      error!("not logged in!");
      process::exit(1);
    }
  }

  /// Adds the given chunk to the world. Should only be called after receiving a
  /// MapChunk packet.
  pub fn add_chunk(&self, pb: proto::Chunk) {
    let mut chunks = self.chunks.write().unwrap();
    info!("adding chunk at {} {}", pb.x, pb.z);
    chunks.insert(
      ChunkPos::new(pb.x, pb.z),
      Mutex::new(MeshChunk::from_proto(pb, self.device.clone())),
    );
  }

  /// Renders the entire game (without the UI), from the main player's
  /// perspective. This will panic if `main_player` is `None`.
  pub fn render(
    &self,
    win: &mut WindowData,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
  ) {
    let mut p = self.main_player.lock().unwrap();
    let p = p.as_mut().unwrap();
    p.render(win);

    let proj =
      cgmath::perspective(Deg(70.0), win.width() as f32 / win.height() as f32, 0.1, 1000.0);
    let yaw = Deg(-p.yaw());
    let pitch = Deg(p.pitch());
    let view = Matrix4::look_to_rh(
      p.pos(),
      Vector3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()),
      Vector3::unit_y(),
    );
    let model = Matrix4::from_translation(Vector3::new(0.0, 0.0, 5.0));

    let mut pc =
      game_vs::ty::PushData { proj: proj.into(), model: model.into(), view: view.into() };

    for (pos, c) in self.chunks.read().unwrap().iter() {
      pc.model =
        Matrix4::from_translation(Vector3::new((pos.x() * 16) as f32, 0.0, (pos.z() * 16) as f32))
          .into();
      let mut chunk = c.lock().unwrap();
      builder
        .draw(win.game_pipeline().clone(), win.dyn_state(), chunk.get_vbuf().clone(), (), pc, [])
        .unwrap();
    }

    builder
      .draw(win.game_pipeline().clone(), win.dyn_state(), self.vbuf.clone(), (), pc, [])
      .unwrap();
  }

  pub fn connect(self: Arc<Self>, ip: String, win: Arc<Mutex<WindowData>>, ui: Arc<UI>) {
    tokio::spawn(async move {
      let conn;
      {
        let settings = self.settings.lock().await;
        conn = match Connection::new(&ip, &settings).await {
          Some(c) => Arc::new(c),
          None => return,
        };
        self.set_main_player(Some(MainPlayer::new(&settings, conn.clone())));
      }
      win.lock().unwrap().start_ingame(self.clone());
      ui.switch_to(LayoutKind::Game);
      conn.run(&self).await.unwrap();
    });
  }

  fn set_main_player(&self, player: Option<MainPlayer>) {
    *self.main_player.lock().unwrap() = player;
  }
}
