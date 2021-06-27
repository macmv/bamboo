use rusttype::{point, Font, Point, PositionedGlyph, Rect, Scale};
use std::{cmp::max, collections::HashMap, sync::Arc};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
  descriptor::{descriptor_set::PersistentDescriptorSet, PipelineLayoutAbstract},
  device::{Device, Queue},
  format::Format,
  image::{
    view::ImageView, AttachmentImage, ImageCreateFlags, ImageDimensions, ImageLayout, ImageUsage,
    ImmutableImage, SwapchainImage,
  },
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  render_pass::{Framebuffer, FramebufferAbstract, RenderPass, Subpass},
  sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
  swapchain::Swapchain,
};
use winit::window::Window;

#[derive(Default, Debug, Clone)]
struct Vert {
  pos: [f32; 2],
  uv:  [f32; 2],
  col: [f32; 4],
}
vulkano::impl_vertex!(Vert, pos, uv, col);

mod vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    path: "src/shader/text.vs",
  }
}

mod fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    path: "src/shader/text.fs",
  }
}

struct TextData {
  glyphs: Vec<PositionedGlyph<'static>>,
  color:  [f32; 4],
}

pub struct TextRender {
  device: Arc<Device>,
  queue:  Arc<Queue>,
  font:   Font<'static>,
  // If the rect is none, then it is something like a space, and will never be given to us by
  // layout().
  cache:  HashMap<char, Option<Rect<i32>>>,
  size:   Scale,

  buffer:      Arc<CpuAccessibleBuffer<[u8]>>,
  tex:         Arc<AttachmentImage>,
  cache_size:  Point<usize>,
  new_glyph_x: usize,

  pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  /* cache_pixel_buffer: Vec<u8>,
   *
   * texts:              Vec<TextData>, */
}

const CACHE_WIDTH: usize = 1024;
const CACHE_HEIGHT: usize = 1024;

impl TextRender {
  pub fn new<W>(
    size: f32,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<W>>,
  ) -> Self
  where
    W: Send + Sync + 'static,
  {
    let font_data = include_bytes!("/usr/share/fonts/TTF/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

    let render_pass = Arc::new(
      vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
          color: {
            load: Load,
            store: Store,
            format: swapchain.format(),
            samples: 1,
          }
        },
        pass: {
          color: [color],
          depth_stencil: {}
        }
      )
      .unwrap(),
    ) as Arc<RenderPass>;

    let pipeline = Arc::new(
      GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vert>()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .blend_alpha_blending()
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap(),
    );

    TextRender {
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
      device,
      queue,
      font,
      cache: HashMap::new(),
      size: Scale::uniform(size),
      cache_size: Point { x: 1, y: 1 },
      new_glyph_x: 0,
      pipeline,
    }
  }

  // pub fn queue_text(&mut self, x: f32, y: f32, size: f32, color: [f32; 4],
  // text: &str) {   let glyphs: Vec<PositionedGlyph> =
  //     self.font.layout(text, Scale::uniform(size), point(x, y)).map(|g|
  // g.clone()).collect();   for glyph in &glyphs.clone() {
  //     self.cache.queue_glyph(0, glyph.clone());
  //   }
  //   self.texts.push(TextData { glyphs: glyphs.clone(), color });
  // }

  /// Updates the texture cache to include all chars of the given text. Returns
  /// true of the cache was updated.
  fn update_cache(&mut self, text: &str) -> bool {
    let mut new_chars = vec![];
    for c in text.chars() {
      if self.cache.contains_key(&c) {
        continue;
      }
      let g = self.font.glyph(c).scaled(self.size).positioned(Point { x: 0.0, y: 0.0 });
      let bounds = g.pixel_bounding_box();
      self.cache.insert(c, bounds);
      if let Some(b) = bounds {
        let mut buf = vec![0; b.width() as usize * b.height() as usize];
        g.draw(|x, y, v| {
          let v = (v * 255.0).round() as u8;
          buf[y as usize * b.width() as usize + x as usize] = v;
        });
        new_chars.push((b, buf));
      }
    }
    for (bounds, buf) in &new_chars {
      self.add_char(*bounds, buf);
    }
    new_chars.len() > 0
  }

  fn add_char(&mut self, bounds: Rect<i32>, buf: &[u8]) {
    let bw = bounds.width() as usize;
    let bh = bounds.height() as usize;
    let cw = self.cache_size.x as usize;
    let ch = self.cache_size.y as usize;
    let double_x = self.new_glyph_x + bw > cw;
    let double_y = bh > ch;
    if double_x && double_y {
      let new_width = max(bh, ch * 2);
      let new_height = max(self.new_glyph_x + bw, cw * 2);
      self.resize(new_width, new_height);
    } else if double_x {
      let new_width = max(bh, ch * 2);
      self.resize(new_width, ch);
    } else if double_y {
      let new_height = max(self.new_glyph_x + bw, cw * 2);
      self.resize(cw, new_height);
    }

    let mut gpu_buf = self.buffer.write().unwrap();
    for y in 0..bounds.height() as usize {
      for x in 0..bounds.width() as usize {
        let buf_index = y * bounds.width() as usize + x;
        let cache_index = y * cw as usize + x + self.new_glyph_x;
        gpu_buf[cache_index] = buf[buf_index];
      }
    }
  }

  fn resize(&mut self, width: usize, height: usize) {
    self.buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
      self.device.clone(),
      BufferUsage::all(),
      false,
      vec![0; width * height].iter().cloned(),
    )
    .unwrap();
    self.tex = AttachmentImage::with_usage(
      self.device.clone(),
      [width as u32, height as u32],
      Format::R8Unorm,
      ImageUsage { transfer_destination: true, sampled: true, ..ImageUsage::none() },
    )
    .unwrap();
  }

  pub fn draw_text<'a>(
    &mut self,
    command_buffer: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    framebuf: Arc<Framebuffer<((), Arc<ImageView<Arc<SwapchainImage<Window>>>>)>>,
    pos: (f32, f32),
    text: &str,
  ) {
    if self.update_cache(text) {
      command_buffer.copy_buffer_to_image(self.buffer.clone(), self.tex.clone()).unwrap();
    }
    let sampler = Sampler::new(
      self.device.clone(),
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

    // let mut command_buffer = command_buffer
    //   .copy_buffer_to_image(buffer, cache_texture_write)
    //   .unwrap()
    //   .begin_render_pass(
    //     framebuf.clone(),
    //     SubpassContents::Inline,
    //     vec![ClearValue::Float([0.0, 0.0, 0.2, 1.0])],
    //   )
    //   .unwrap();

    // draw
    // for text in &mut self.texts.drain(..) {
    //   let vertices: Vec<Vert> = text
    //     .glyphs
    //     .iter()
    //     .flat_map(|g| {
    //       if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
    //         let gl_rect = Rect {
    //           min: point(
    //             (screen_rect.min.x as f32 / screen_width as f32 - 0.5) * 2.0,
    //             (screen_rect.min.y as f32 / screen_height as f32 - 0.5) *
    // 2.0,           ),
    //           max: point(
    //             (screen_rect.max.x as f32 / screen_width as f32 - 0.5) * 2.0,
    //             (screen_rect.max.y as f32 / screen_height as f32 - 0.5) *
    // 2.0,           ),
    //         };
    //         vec![
    //           Vert {
    //             pos: [gl_rect.min.x, gl_rect.max.y],
    //             uv:  [uv_rect.min.x, uv_rect.max.y],
    //             col: text.color,
    //           },
    //           Vert {
    //             pos: [gl_rect.min.x, gl_rect.min.y],
    //             uv:  [uv_rect.min.x, uv_rect.min.y],
    //             col: text.color,
    //           },
    //           Vert {
    //             pos: [gl_rect.max.x, gl_rect.min.y],
    //             uv:  [uv_rect.max.x, uv_rect.min.y],
    //             col: text.color,
    //           },
    //           Vert {
    //             pos: [gl_rect.max.x, gl_rect.min.y],
    //             uv:  [uv_rect.max.x, uv_rect.min.y],
    //             col: text.color,
    //           },
    //           Vert {
    //             pos: [gl_rect.max.x, gl_rect.max.y],
    //             uv:  [uv_rect.max.x, uv_rect.max.y],
    //             col: text.color,
    //           },
    //           Vert {
    //             pos: [gl_rect.min.x, gl_rect.max.y],
    //             uv:  [uv_rect.min.x, uv_rect.max.y],
    //             col: text.color,
    //           },
    //         ]
    //         .into_iter()
    //       } else {
    //         vec![].into_iter()
    //       }
    //     })
    //     .collect();
    //
    //   let vertex_buffer = CpuAccessibleBuffer::from_iter(
    //     self.device.clone(),
    //     BufferUsage::all(),
    //     false,
    //     vertices.into_iter(),
    //   )
    //   .unwrap();
    //   command_buffer = command_buffer
    //     .draw(
    //       self.pipeline.clone(),
    //       &DynamicState::none(),
    //       vertex_buffer.clone(),
    //       set.clone(),
    //       (),
    //       vec![],
    //     )
    //     .unwrap();
    // }
  }
}
