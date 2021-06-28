use crate::{
  graphics::{ui_vs, GameWindow, Vert, WindowData},
  util::load,
};
use std::{
  collections::HashMap,
  fs,
  sync::{Arc, Mutex},
  time::Instant,
};
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
  buttons:    Vec<Mutex<Button>>,
  background: Option<Arc<ImmutableImage>>,
}

pub struct Button {
  pos:      Vert,
  size:     Vert,
  on_click: Box<dyn FnMut()>,
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
  sets: HashMap<
    String,
    Arc<
      PersistentDescriptorSet<(
        ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
        PersistentDescriptorSetSampler,
      )>,
    >,
  >,
  vbuf:  Arc<CpuAccessibleBuffer<[Vert]>>,
  start: Instant,

  layouts: HashMap<LayoutKind, Layout>,
  current: LayoutKind,
}

impl UI {
  pub fn new(win: &mut GameWindow) -> Self {
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

    let mut futures = vec![];
    let mut sets = HashMap::new();
    let layout = win.ui_pipeline().layout().descriptor_set_layout(0).unwrap();
    for p in fs::read_dir("client/assets/textures/ui").unwrap().map(|res| res.unwrap().path()) {
      if let Some(ext) = p.extension() {
        if ext != "png" {
          continue;
        }
      } else {
        continue;
      }

      info!("{:?}", p);
      let (img, fut) = load::png(p.to_str().unwrap(), win.queue().clone()).unwrap();
      sets.insert(
        p.file_stem().unwrap().to_str().unwrap().into(),
        Arc::new(
          PersistentDescriptorSet::start(layout.clone())
            .add_sampled_image(img, sampler.clone())
            .unwrap()
            .build()
            .unwrap(),
        ),
      );
      futures.push(fut);
    }
    for f in futures {
      win.add_initial_future(f);
    }

    let vbuf = CpuAccessibleBuffer::from_iter(
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
    .unwrap();

    UI { sets, vbuf, start: Instant::now(), layouts: HashMap::new(), current: LayoutKind::Menu }
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
      ops.append(&mut b.lock().unwrap().draw(win));
    }
    for o in ops {
      match o {
        DrawOp::Image(pos, size, name) => {
          let pc = ui_vs::ty::PushData {
            pos:         pos.into(),
            size:        size.into(),
            corner_size: 0.03,
            ratio:       win.width() as f32 / win.height() as f32,
          };
          match self.sets.get(&name) {
            Some(set) => {
              builder
                .draw(pipeline.clone(), dyn_state, self.vbuf.clone(), set.clone(), pc, [])
                .unwrap();
            }
            None => error!("unknown UI image {}", name),
          }
        }
        DrawOp::Text(_pos, _text) => {}
      }
    }
  }

  pub fn on_click(&self, win: &WindowData) {
    let l = &self.layouts[&self.current];
    for b in &l.buttons {
      b.lock().unwrap().on_click(win);
    }
  }
}

impl Layout {
  pub fn new() -> Self {
    Layout { buttons: vec![], background: None }
  }

  pub fn button<F>(mut self, pos: Vert, size: Vert, on_click: F) -> Self
  where
    F: FnMut() + 'static,
  {
    self.buttons.push(Mutex::new(Button::new(pos, size, on_click)));
    self
  }
}

impl Button {
  fn new<F>(pos: Vert, size: Vert, on_click: F) -> Self
  where
    F: FnMut() + 'static,
  {
    Button { pos, size, on_click: Box::new(on_click) }
  }
  fn draw(&self, win: &WindowData) -> Vec<DrawOp> {
    let (mx, my) = win.mouse_screen_pos();
    let (mx, my) = (mx as f32, my as f32);
    let hovering = mx > self.pos.x()
      && mx < self.pos.x() + self.size.x()
      && my > self.pos.y()
      && my < self.pos.y() + self.size.y();
    if hovering {
      vec![DrawOp::Image(self.pos, self.size, "button-hover".into())]
    } else {
      vec![DrawOp::Image(self.pos, self.size, "button-up".into())]
    }
  }
  fn on_click(&mut self, win: &WindowData) {
    let (mx, my) = win.mouse_screen_pos();
    let (mx, my) = (mx as f32, my as f32);
    let hovering = mx > self.pos.x()
      && mx < self.pos.x() + self.size.x()
      && my > self.pos.y()
      && my < self.pos.y() + self.size.y();
    if hovering {
      (self.on_click)();
    }
  }
}
