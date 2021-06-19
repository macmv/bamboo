use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};

pub struct UI {}

impl UI {
  pub fn new() -> Self {
    UI {}
  }

  pub fn draw(&self, builer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {}
}
