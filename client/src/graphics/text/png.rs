use super::{vs, TextRender};
use crate::{
  graphics::{GameWindow, Vert, WindowData},
  util::load,
};
use std::{collections::HashMap, sync::Arc};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  descriptor::{
    descriptor_set::{
      PersistentDescriptorSet, PersistentDescriptorSetImg, PersistentDescriptorSetSampler,
    },
    PipelineLayoutAbstract,
  },
  format::Format,
  image::{view::ImageView, ImmutableImage, MipmapsCount},
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
};

pub struct PNGRender {
  texts: Vec<((f32, f32), String)>,
  size:  f32,

  chars: HashMap<char, (f32, f32)>,

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
  pub fn new(path: &str, size: f32, win: &mut GameWindow) -> Self {
    let (img_data, dims) = load::png_data(path).unwrap();
    let mut chars = HashMap::new();

    let char_width = dims.width() as usize / 16;
    let char_height = dims.height() as usize / 16;
    {
      let x = 13;
      let y = 5;
      for row in 0..char_height {
        for col in 0..char_width {
          let index =
            (((y * char_height + row) * dims.width() as usize) + (x * char_width + col)) * 4;
          if img_data[index] > 128 {
            print!("#");
          } else {
            print!(".");
          }
        }
        println!();
      }
    }

    let mut x = 0;
    let mut y = 2;
    for c in ' '..'~' {
      let mut width = char_width;
      for col in 0..char_width {
        let mut end = true;
        for row in 0..char_height {
          let index =
            (((y * char_height + row) * dims.width() as usize) + (x * char_width + col)) * 4;
          if img_data[index] > 0 {
            end = false;
            break;
          }
        }
        if end {
          width = col;
          break;
        }
      }
      chars.insert(c, (width as f32, 8.0));
      x += 1;
      if x >= 16 {
        x = 0;
        y += 1;
      }
    }

    let (tex, fut) = ImmutableImage::from_iter(
      img_data.iter().cloned(),
      dims,
      MipmapsCount::One,
      Format::R8G8B8A8Srgb,
      win.queue().clone(),
    )
    .unwrap();
    win.add_initial_future(fut);

    let sampler = Sampler::new(
      win.device().clone(),
      Filter::Nearest,
      Filter::Nearest,
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
        .add_sampled_image(ImageView::new(tex).unwrap(), sampler)
        .unwrap()
        .build()
        .unwrap(),
    );

    PNGRender {
      size,
      chars,
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
    // The image is 16x16 characters. Within that image, every character is 5x7
    // pixels. It does not matter if the image is double the size, because images
    // are accessed from 0-1.
    for (pos, text) in &self.texts {
      let mut x = pos.0;
      for c in text.chars() {
        if (' '..'~').contains(&c) {
          let index = c as u32 - ' ' as u32;
          let uv_x = (index % 16) as f32 / 16.0;
          let uv_y = (index / 16 + 2) as f32 / 16.0;
          let size = self.chars[&c];
          let pc = vs::ty::PushData {
            offset:    [x, pos.1],
            uv_offset: [uv_x, uv_y],
            col:       [0.0, 1.0, 1.0, 1.0],
            size:      [
              size.0 * self.size / win.width() as f32,
              size.1 * self.size / win.height() as f32,
            ],
            uv_size:   [size.0 / 128.0, size.1 / 128.0],
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
          x += (size.0 + 1.0) * self.size / win.width() as f32;
        }
      }
    }
  }
}
