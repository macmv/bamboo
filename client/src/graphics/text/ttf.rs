use super::{vs, TextRender};
use crate::graphics::{Vert2, WindowData};
use rusttype::{Font, GlyphId, Point, Rect, Scale};
use std::{cmp::max, collections::HashMap, mem, sync::Arc};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  descriptor::{descriptor_set::PersistentDescriptorSet, PipelineLayoutAbstract},
  device::Device,
  format::Format,
  image::{view::ImageView, AttachmentImage, ImageUsage},
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
  swapchain::Swapchain,
};

pub struct TTFRender {
  device: Arc<Device>,
  font:   Font<'static>,
  // If the rect is none, then it is something like a space, and will never be given to us by
  // layout(). The none is stored so that we don't try to add it to the cache every time we render.
  cache:  HashMap<GlyphId, Option<Rect<i32>>>,
  size:   Scale,

  texts:       Vec<((f32, f32), String)>,
  buffer:      Arc<CpuAccessibleBuffer<[u8]>>,
  tex:         Arc<AttachmentImage>,
  cache_size:  Point<usize>,
  new_glyph_x: usize,

  vbuf:     Arc<CpuAccessibleBuffer<[Vert2]>>,
  pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert2>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
}

impl TTFRender {
  pub fn new<W>(size: f32, device: Arc<Device>, swapchain: Arc<Swapchain<W>>) -> Self
  where
    W: Send + Sync + 'static,
  {
    let font_data = include_bytes!("/usr/share/fonts/TTF/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

    TTFRender {
      buffer: CpuAccessibleBuffer::<[u8]>::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        [0].iter().cloned(),
      )
      .unwrap(),
      tex: AttachmentImage::with_usage(
        device.clone(),
        [1, 1],
        Format::R8Unorm,
        ImageUsage { transfer_destination: true, sampled: true, ..ImageUsage::none() },
      )
      .unwrap(),
      vbuf: CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        [
          Vert2::new(0.0, 0.0),
          Vert2::new(1.0, 1.0),
          Vert2::new(1.0, 0.0),
          Vert2::new(0.0, 0.0),
          Vert2::new(0.0, 1.0),
          Vert2::new(1.0, 1.0),
        ]
        .iter()
        .cloned(),
      )
      .unwrap(),
      pipeline: super::create_pipeline(device.clone(), swapchain.clone()),
      device,
      font,
      texts: vec![],
      cache: HashMap::new(),
      size: Scale::uniform(size),
      cache_size: Point { x: 1, y: 1 },
      new_glyph_x: 1,
    }
  }

  /// Updates the texture cache to include all chars of the given text. Returns
  /// true of the cache was updated.
  fn update_cache(&mut self, text: &str) -> bool {
    let mut changed = false;
    for c in text.chars() {
      if self.cache.contains_key(&self.font.glyph(c).id()) {
        continue;
      }
      let g = self.font.glyph(c).scaled(self.size).positioned(Point { x: 0.0, y: 0.0 });
      let bounds = g.pixel_bounding_box();
      if let Some(mut b) = bounds {
        // We want bounds.min to be 0, 0, so that add_char can set the min to be the
        // correct top left of the cached character.
        b.max.x -= b.min.x;
        b.max.y -= b.min.y;
        b.min.x = 0;
        b.min.y = 0;
        let mut buf = vec![0; b.width() as usize * b.height() as usize];
        g.draw(|x, y, v| {
          let v = (v * 255.0).round() as u8;
          buf[y as usize * b.width() as usize + x as usize] = v;
        });
        b = self.add_char(b, &buf);
        self.cache.insert(g.id(), Some(b));
        changed = true;
      } else {
        self.cache.insert(g.id(), None);
      }
    }
    changed
  }

  fn add_char(&mut self, mut bounds: Rect<i32>, buf: &[u8]) -> Rect<i32> {
    let bw = bounds.width() as usize;
    let bh = bounds.height() as usize;
    let cw = self.cache_size.x as usize;
    let ch = self.cache_size.y as usize;
    let double_x = self.new_glyph_x + bw > cw;
    let double_y = bh > ch;
    if double_x && double_y {
      let new_width = max(self.new_glyph_x + bw, cw * 2);
      let new_height = max(bh, ch * 2);
      self.resize(new_width, new_height);
    } else if double_x {
      let new_width = max(self.new_glyph_x + bw, cw * 2);
      self.resize(new_width, ch);
    } else if double_y {
      let new_height = max(bh, ch * 2);
      self.resize(cw, new_height);
    }

    {
      // cache_size may have been changed by resize()
      let cw = self.cache_size.x as usize;
      let mut gpu_buf = self.buffer.write().unwrap();
      for y in 0..bounds.height() as usize {
        for x in 0..bounds.width() as usize {
          let buf_index = y * bounds.width() as usize + x;
          let cache_index = y * cw as usize + x + self.new_glyph_x;
          gpu_buf[cache_index] = buf[buf_index];
        }
      }
    }
    // Return the bounds of the cached glyph
    bounds.min.x += self.new_glyph_x as i32;
    bounds.max.x += self.new_glyph_x as i32;
    self.new_glyph_x += bounds.width() as usize + 1;
    bounds
  }

  fn resize(&mut self, width: usize, height: usize) {
    info!("resizing to {} {}", width, height);
    let old_buf = mem::replace(
      &mut self.buffer,
      CpuAccessibleBuffer::<[u8]>::from_iter(
        self.device.clone(),
        BufferUsage::all(),
        false,
        vec![0; width * height].iter().cloned(),
      )
      .unwrap(),
    );
    {
      let mut gpu_buf = self.buffer.write().unwrap();
      let old_gpu_buf = old_buf.read().unwrap();
      for y in 0..self.cache_size.y {
        for x in 0..self.cache_size.x {
          let old_index = y * self.cache_size.x + x;
          let new_index = y * width + x;
          gpu_buf[new_index] = old_gpu_buf[old_index];
        }
      }
    }
    self.tex = AttachmentImage::with_usage(
      self.device.clone(),
      [width as u32, height as u32],
      Format::R8Unorm,
      ImageUsage { transfer_destination: true, sampled: true, ..ImageUsage::none() },
    )
    .unwrap();
    self.cache_size = Point { x: width, y: height };
  }
}

impl TextRender for TTFRender {
  /// This text will be rendered on the next draw call. Because of how render
  /// passes work, this call will rasterize any new characters, and add them to
  /// the cache. This updated cache is then used during the draw call.
  fn queue_text(
    &mut self,
    text: &str,
    pos: (f32, f32),
    scale: f32,
    buf: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
  ) {
    if self.update_cache(text) {
      buf.copy_buffer_to_image(self.buffer.clone(), self.tex.clone()).unwrap();
    }
    self.texts.push((pos, text.into()));
  }

  fn draw(
    &mut self,
    command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    win: &WindowData,
  ) {
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

    let tex_view = ImageView::new(self.tex.clone()).unwrap();

    let set = Arc::new(
      PersistentDescriptorSet::start(self.pipeline.descriptor_set_layout(0).unwrap().clone())
        .add_sampled_image(tex_view, sampler)
        .unwrap()
        .build()
        .unwrap(),
    );

    for (pos, text) in self.texts.drain(..) {
      for g in self.font.layout(&text, self.size, Point { x: 0.0, y: 0.0 }) {
        let uv_bounds = match self.cache[&g.id()] {
          Some(v) => Rect {
            min: Point { x: v.min.x as f32, y: v.min.y as f32 },
            max: Point { x: v.max.x as f32, y: v.max.y as f32 },
          },
          None => continue,
        };
        let bounds = g.pixel_bounding_box().unwrap();
        let offset = [
          bounds.min.x as f32 / win.width() as f32 + pos.0,
          bounds.min.y as f32 / win.height() as f32 + pos.1,
        ];
        let pc = vs::ty::PushData {
          offset,
          uv_offset: [
            uv_bounds.min.x / self.cache_size.x as f32,
            uv_bounds.min.y / self.cache_size.y as f32,
          ],
          col: [0.0, 1.0, 1.0, 1.0],
          size: [
            bounds.width() as f32 / win.width() as f32,
            bounds.height() as f32 / win.height() as f32,
          ],
          uv_size: [
            uv_bounds.width() / self.cache_size.x as f32,
            uv_bounds.height() / self.cache_size.y as f32,
          ],
        };
        command_buffer
          .draw(self.pipeline.clone(), win.dyn_state(), self.vbuf.clone(), set.clone(), pc, [])
          .unwrap();
      }
    }
  }
}
