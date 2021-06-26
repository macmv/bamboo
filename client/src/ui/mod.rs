use crate::{
  graphics::{ui_vs, GameWindow, Vert, WindowData},
  util::load,
};
use std::{sync::Arc, time::Instant};
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
  set_hover: Arc<
    PersistentDescriptorSet<(
      ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
      PersistentDescriptorSetSampler,
    )>,
  >,
  set_down: Arc<
    PersistentDescriptorSet<(
      ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
      PersistentDescriptorSetSampler,
    )>,
  >,
  vbuf:      Arc<CpuAccessibleBuffer<[Vert]>>,
  start:     Instant,
}

impl UI {
  pub fn new(win: &mut GameWindow) -> Self {
    let (down, down_fut) =
      load::png("client/assets/textures/ui/button-down.png", win.queue().clone()).unwrap();
    let (hover, hover_fut) =
      load::png("client/assets/textures/ui/button-hover.png", win.queue().clone()).unwrap();

    win.add_initial_future(down_fut);
    win.add_initial_future(hover_fut);

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
    let set_down = Arc::new(
      PersistentDescriptorSet::start(layout.clone())
        .add_sampled_image(down, sampler.clone())
        .unwrap()
        .build()
        .unwrap(),
    );
    let set_hover = Arc::new(
      PersistentDescriptorSet::start(layout.clone())
        .add_sampled_image(hover, sampler)
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

    UI { set_hover, set_down, vbuf, start: Instant::now() }
  }

  pub fn draw(
    &self,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<
      GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
    >,
    dyn_state: &DynamicState,
    win: &WindowData,
  ) {
    let t = Instant::now().duration_since(self.start).as_secs_f32() / 10.0 % 1.0;
    // Center of the button
    let x = 0.3;
    let y = 0.4;
    // Width in screen space
    let w = 0.1 + t / 3.0;
    let h = 0.4 - t / 3.0;
    let (mx, my) = win.mouse_screen_pos();
    let (mx, my) = (mx as f32, my as f32);
    let hovering = mx > x - w / 2.0 && mx < x + w / 2.0 && my > y - h / 2.0 && my < y + h / 2.0;
    let pc = ui_vs::ty::PushData {
      pos:         [x, y],
      size:        [w, h],
      corner_size: 0.05,
      ratio:       win.width() as f32 / win.height() as f32,
    };
    if hovering {
      builder.draw(pipeline, dyn_state, self.vbuf.clone(), self.set_hover.clone(), pc, []).unwrap();
    } else {
      builder.draw(pipeline, dyn_state, self.vbuf.clone(), self.set_down.clone(), pc, []).unwrap();
    }
  }
}
