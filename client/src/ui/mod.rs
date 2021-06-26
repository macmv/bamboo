use crate::{
  graphics::{ui_vs, GameWindow, Vert, WindowData},
  util::load,
};
use std::{collections::HashMap, sync::Arc, time::Instant};
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

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum LayoutKind {
  Menu,
  Loading,
  Game,
}

pub struct Layout {
  buttons:    Vec<Button>,
  background: Option<Arc<ImmutableImage>>,
}

pub struct Button {
  pos:  Vert,
  size: Vert,
}

/// A drawing operator. Used to easily pass draw calls to a [`UI`].
pub enum DrawOp {
  /// Draws an image. The first vertex is the top-left of the image, and the
  /// second vertex is the size of the image. Both vertices are in scree-space
  /// coordinates. The string is an image name (the filename of an image within
  /// textures/ui, without the .png extension).
  Image(Vert, Vert, String),
  /// Draws some text to the screen. The vertex is the top-left of the text.
  Text(Vert, String),
}

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

  layouts: HashMap<LayoutKind, Layout>,
  current: LayoutKind,
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

    UI {
      set_hover,
      set_down,
      vbuf,
      start: Instant::now(),
      layouts: HashMap::new(),
      current: LayoutKind::Menu,
    }
  }

  /// Creates a layout. This should only be used in initialization.
  pub fn set_layout(&mut self, k: LayoutKind, l: Layout) {
    self.layouts.insert(k, l);
  }
  /// Switches the current layout to the given layout kind. This should be used
  /// during a button press, or similar.
  pub fn switch_to(&mut self, k: LayoutKind) {
    self.current = k
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
    let l = &self.layouts[&self.current];
    let mut ops = vec![];
    for b in &l.buttons {
      ops.append(&mut b.draw(win));
    }
    for o in ops {
      match o {
        DrawOp::Image(pos, size, name) => {
          let pc = ui_vs::ty::PushData {
            pos:         pos.into(),
            size:        size.into(),
            corner_size: 0.05,
            ratio:       win.width() as f32 / win.height() as f32,
          };
          builder
            .draw(pipeline.clone(), dyn_state, self.vbuf.clone(), self.set_hover.clone(), pc, [])
            .unwrap();
        }
        DrawOp::Text(pos, text) => {}
      }
    }
  }
}

impl Button {
  fn draw(&self, win: &WindowData) -> Vec<DrawOp> {
    let (mx, my) = win.mouse_screen_pos();
    let (mx, my) = (mx as f32, my as f32);
    let hovering = mx > self.pos.x() - self.size.x() / 2.0
      && mx < self.pos.x() + self.size.x() / 2.0
      && my > self.pos.y() - self.size.y() / 2.0
      && my < self.pos.y() + self.pos.y() / 2.0;
    if hovering {
      vec![DrawOp::Image(self.pos, self.size, "button-hover".into())]
    } else {
      vec![DrawOp::Image(self.pos, self.size, "button-up".into())]
    }
  }
}
