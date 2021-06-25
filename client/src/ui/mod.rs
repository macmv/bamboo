use crate::{
  graphics::{ui_vs, GameWindow, Vert},
  util::load,
};
use std::sync::Arc;
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
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

pub struct UI {
  set: Arc<
    PersistentDescriptorSet<(
      ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
      PersistentDescriptorSetSampler,
    )>,
  >,
  vbuf: Arc<CpuAccessibleBuffer<[Vert]>>,
}

impl UI {
  pub fn new(win: &mut GameWindow) -> Self {
    let (tex, tex_fut) =
      load::png("client/assets/textures/ui/button-down.png", win.queue().clone()).unwrap();

    win.add_initial_future(tex_fut);

    let sampler = Sampler::new(
      win.device().clone(),
      Filter::Nearest,
      Filter::Nearest,
      MipmapMode::Linear,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      0.0,
      1.0,
      0.0,
      0.0,
    )
    .unwrap();

    let layout = win.ui_pipeline().layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(
      PersistentDescriptorSet::start(layout.clone())
        .add_sampled_image(tex, sampler)
        .unwrap()
        .build()
        .unwrap(),
    );

    let vbuf = CpuAccessibleBuffer::from_iter(
      win.device().clone(),
      BufferUsage::all(),
      false,
      [
        Vert::new(-1.0, -1.0),
        Vert::new(1.0, 1.0),
        Vert::new(1.0, -1.0),
        Vert::new(-1.0, -1.0),
        Vert::new(-1.0, 1.0),
        Vert::new(1.0, 1.0),
      ]
      .iter()
      .cloned(),
    )
    .unwrap();

    UI { set, vbuf }
  }

  pub fn draw(
    &self,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<
      GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
    >,
    dyn_state: &DynamicState,
  ) {
    let pc =
      ui_vs::ty::PushData { pos: [0.3, 0.4], size: [0.4, 0.05], corner_size: 0.1 };
    builder.draw(pipeline, dyn_state, self.vbuf.clone(), self.set.clone(), pc, []).unwrap();
  }
}
