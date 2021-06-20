use crate::graphics::GameWindow;
use imgui::Context;
use imgui_vulkano_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::sync::{Arc, Mutex};
use vulkano::{
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  device::Queue,
  image::ImageViewAbstract,
  render_pass::FramebufferAbstract,
};

pub struct UI {
  ctx:    Mutex<Context>,
  render: Mutex<Renderer>,
}

impl UI {
  pub fn new(win: &GameWindow) -> Self {
    let mut ctx = Context::create();

    let mut platform = WinitPlatform::init(&mut ctx);
    platform.attach_window(ctx.io_mut(), &win.swapchain().surface().window(), HiDpiMode::Rounded);

    let render =
      Renderer::init(&mut ctx, win.device().clone(), win.queue().clone(), win.format()).unwrap();
    UI { ctx: Mutex::new(ctx), render: Mutex::new(render) }
  }

  pub fn draw<T>(
    &self,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    queue: Arc<Queue>,
    target: Arc<T>,
  ) where
    T: ImageViewAbstract + Sync + Send + 'static,
  {
    let mut ctx = self.ctx.lock().unwrap();
    let mut render = self.render.lock().unwrap();

    let ui = ctx.frame();
    ui.text("Hello world");

    let draw_data = ui.render();
    render.draw_commands(builder, queue, target, draw_data).unwrap();
  }
}
