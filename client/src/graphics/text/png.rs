use super::{vs, TextRender};
use crate::{
  graphics::{GameWindow, Vert, WindowData},
  util::load,
};
use std::sync::Arc;
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  descriptor::{
    descriptor_set::{
      PersistentDescriptorSet, PersistentDescriptorSetImg, PersistentDescriptorSetSampler,
    },
    PipelineLayoutAbstract,
  },
  image::{view::ImageView, ImmutableImage},
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
};

pub struct PNGRender {
  texts: Vec<((f32, f32), String)>,

  vbuf:     Arc<CpuAccessibleBuffer<[Vert]>>,
  set: Arc<
    PersistentDescriptorSet<(
      ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
      PersistentDescriptorSetSampler,
    )>,
  >,
  pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
}

impl PNGRender {
  /// Creates a new png text renderer. This function should be used during init,
  /// and a different function should be called if you need to reload a png
  /// render.
  pub fn new_init(path: &str, size: f32, win: &mut GameWindow) -> Self {
    let (tex, fut) = load::png(path, win.queue().clone()).unwrap();
    win.add_initial_future(fut);
    let sampler = Sampler::new(
      win.device().clone(),
      Filter::Linear,
      Filter::Linear,
      MipmapMode::Nearest,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      0.0,
      1.0,
      0.0,
      0.0,
    )
    .unwrap();

    let pipeline = super::create_pipeline(win.device().clone(), win.swapchain().clone());

    let set = Arc::new(
      PersistentDescriptorSet::start(pipeline.descriptor_set_layout(0).unwrap().clone())
        .add_sampled_image(tex, sampler)
        .unwrap()
        .build()
        .unwrap(),
    );

    PNGRender {
      texts: vec![],
      vbuf: CpuAccessibleBuffer::from_iter(
        win.device().clone(),
        BufferUsage::all(),
        false,
        [
          Vert::new(0.0, 0.0),
          Vert::new(1.0, 1.0),
          Vert::new(1.0, 0.0),
          Vert::new(0.0, 0.0),
          Vert::new(0.0, 1.0),
          Vert::new(1.0, 1.0),
        ]
        .iter()
        .cloned(),
      )
      .unwrap(),
      set,
      pipeline,
    }
  }
}

impl TextRender for PNGRender {
  fn queue_text(
    &mut self,
    text: &str,
    pos: (f32, f32),
    scale: f32,
    buf: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
  ) {
    self.texts.push((pos, text.into()));
  }
  fn draw(
    &mut self,
    command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    win: &WindowData,
  ) {
    for (pos, text) in &self.texts {
      for c in text.chars() {
        if (' '..'~').contains(&c) {
          let index = c as u32 - ' ' as u32;
          let uv_x = (index % 16) as f32 / 16.0;
          let uv_y = (index / 16 + 2) as f32 / 16.0;
          let pc = vs::ty::PushData {
            offset:    [pos.0, pos.1],
            uv_offset: [uv_x, uv_y],
            col:       [0.0, 1.0, 1.0, 1.0],
            size:      [5.0 as f32 / win.width() as f32, 7.0 as f32 / win.height() as f32],
            uv_size:   [1.0 / 16.0, 1.0 / 16.0],
          };
          command_buffer
            .draw(
              self.pipeline.clone(),
              win.dyn_state(),
              self.vbuf.clone(),
              self.set.clone(),
              pc,
              [],
            )
            .unwrap();
        }
      }
    }
  }
}
