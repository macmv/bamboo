use crate::graphics::GameWindow;
use std::sync::Arc;
use vulkano::{
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  device::Queue,
  format::Format,
  image::{view::ImageView, ImageDimensions, StorageImage, SwapchainImage},
  render_pass::Framebuffer,
  sampler::Filter,
};

pub struct UI {
  img: Arc<StorageImage>,
}

impl UI {
  pub fn new(win: &GameWindow) -> Self {
    let img = StorageImage::new(
      win.device().clone(),
      ImageDimensions::Dim2d { array_layers: 1, width: 1024, height: 1024 },
      Format::B8G8R8A8Unorm,
      Some(win.queue().family()),
    )
    .unwrap();
    UI { img }
  }

  pub fn draw(
    &self,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    queue: Arc<Queue>,
    target: Arc<Framebuffer<((), Arc<ImageView<Arc<SwapchainImage<winit::window::Window>>>>)>>,
  ) {
  }
}
