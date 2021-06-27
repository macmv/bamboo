mod png;
mod ttf;

pub use self::png::PNGRender;
pub use ttf::TTFRender;

use super::WindowData;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};

pub trait TextRender {
  /// This queues the given text to be rendered on the next draw call. This is
  /// required for the ttf renderer, as it needs to update a cache image. This
  /// cannot be done during a render pass, so this needs to be a seperate
  /// operation from draw().
  fn queue_text(
    &mut self,
    text: &str,
    pos: (f32, f32),
    scale: f32,
    buf: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
  );
  /// This renders all queued text onto the screen.
  fn draw(
    &mut self,
    command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    win: &WindowData,
  );
}
