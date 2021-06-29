use crate::{
  graphics::{game_vs, MeshChunk, Vert3, WindowData},
  net::Connection,
  player::{MainPlayer, OtherPlayer},
  ui::{LayoutKind, UI},
  Settings,
};
use cgmath::{Deg, Matrix4, Vector3, Vector4};
use common::{math::ChunkPos, util::UUID};
use std::{
  collections::HashMap,
  sync::{Arc, Mutex, RwLock},
  time::Instant,
};

use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
};

pub struct World {
  chunks:      RwLock<HashMap<ChunkPos, Mutex<MeshChunk>>>,
  // This will be set whenever the player is in a game.
  main_player: Mutex<Option<MainPlayer>>,
  // List of other players. Does not include the main player.
  players:     HashMap<UUID, OtherPlayer>,
  vbuf:        Arc<CpuAccessibleBuffer<[Vert3]>>,
  start:       Instant,
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
    }
  }

  /// Renders the entire game (without the UI), from the main player's
  /// perspective. This will panic if `main_player` is `None`.
  pub fn render(
    &self,
    win: &mut WindowData,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
  ) {
    let p = self.main_player.lock().unwrap();
    let p = p.as_ref().unwrap();
    let proj =
      cgmath::perspective(Deg(70.0), win.width() as f32 / win.height() as f32, 0.1, 1000.0);
    let view = Matrix4::from_angle_y(Deg((Instant::now() - self.start).as_secs_f32() * 120.0))
      * Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0));
    let model = Matrix4::from_translation(Vector3::new(0.0, 0.0, 5.0));

    let pc = game_vs::ty::PushData { proj: proj.into(), model: model.into(), view: view.into() };

    builder
      .draw(win.game_pipeline().clone(), win.dyn_state(), self.vbuf.clone(), (), pc, [])
      .unwrap();
  }

  pub fn connect(self: Arc<Self>, ip: String, win: Arc<Mutex<WindowData>>, ui: Arc<UI>) {
    tokio::spawn(async move {
      let settings = Settings::new();
      let conn = match Connection::new(&ip, &settings).await {
        Some(c) => Arc::new(c),
        None => return,
      };
      self.set_main_player(Some(MainPlayer::new(&settings, conn.clone())));
      win.lock().unwrap().start_ingame(self);
      ui.switch_to(LayoutKind::Game);
      conn.run().await.unwrap();
    });
  }

  fn set_main_player(&self, player: Option<MainPlayer>) {
    *self.main_player.lock().unwrap() = player;
  }
}
