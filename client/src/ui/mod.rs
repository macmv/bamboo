use crate::graphics::GameWindow;
use imgui::Context;
use imgui_vulkano_renderer::Renderer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};

pub struct UI {
  ctx:      Context,
  renderer: Renderer,
}

impl UI {
  pub fn new(win: &GameWindow) -> Self {
    let mut ctx = Context::create();
    let mut renderer =
      Renderer::init(&mut ctx, win.device().clone(), win.queue().clone(), win.format()).unwrap();
    UI { ctx, renderer }
  }

  pub fn draw(&self, builer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {}
}
