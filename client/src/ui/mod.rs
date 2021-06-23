use crate::graphics::{GameWindow, Vert};
use std::sync::Arc;
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
  descriptor::PipelineLayoutAbstract,
  format::Format,
  image::{ImageDimensions, StorageImage},
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
};

pub struct UI {
  img:  Arc<StorageImage>,
  vbuf: Arc<CpuAccessibleBuffer<[Vert]>>,
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

    let vbuf = CpuAccessibleBuffer::from_iter(
      win.device().clone(),
      BufferUsage::all(),
      false,
      [Vert::new(-0.5, -0.25), Vert::new(0.0, 0.5), Vert::new(0.25, -0.1)].iter().cloned(),
    )
    .unwrap();
    UI { img, vbuf }
  }

  pub fn draw(
    &self,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<
      GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
    >,
    dyn_state: &DynamicState,
  ) {
    builder.draw(pipeline, dyn_state, self.vbuf.clone(), (), (), []).unwrap();
  }
}
