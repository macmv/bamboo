use crate::ui::UI;
use std::{error::Error, fmt, ops::Deref, sync::Arc, time::Instant};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents},
  descriptor::pipeline_layout::PipelineLayoutAbstract,
  device::{Device, Features, Queue},
  format::Format,
  image::{view::ImageView, ImageUsage, SwapchainImage},
  instance::{Instance, PhysicalDevice},
  pipeline::{vertex::SingleBufferDefinition, viewport::Viewport, GraphicsPipeline},
  render_pass::{Framebuffer, RenderPass, Subpass},
  swapchain::{self, AcquireError, Swapchain, SwapchainCreationError},
  sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::{Window, WindowBuilder},
};

mod chunk;
pub use chunk::MeshChunk;

#[derive(Debug)]
pub struct InitError(String);

impl InitError {
  pub fn new<S: Into<String>>(s: S) -> Self {
    InitError(s.into())
  }
}

impl fmt::Display for InitError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "error while initializing graphics: {}", self.0)
  }
}

impl Error for InitError {}

pub struct GameWindow {
  event_loop: EventLoop<()>,
  data:       WindowData,

  initial_future: Option<Box<dyn GpuFuture>>,
}

pub struct WindowData {
  render_pass:   Arc<RenderPass>,
  device:        Arc<Device>,
  queue:         Arc<Queue>,
  game_pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  ui_pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  buffers:       Vec<Arc<Framebuffer<((), Arc<ImageView<Arc<SwapchainImage<Window>>>>)>>>,
  dyn_state:     DynamicState,
  swapchain:     Arc<Swapchain<Window>>,
  format:        Format,
}

#[derive(Default, Copy, Clone)]
pub struct Vert {
  pos: [f32; 2],
}
vulkano::impl_vertex!(Vert, pos);

impl Vert {
  pub fn new(x: f32, y: f32) -> Self {
    Vert { pos: [x, y] }
  }
}

mod game_vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    src: "
#version 450

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 uv;

layout(push_constant) uniform PushData {
  vec2 offset;
} pc;

void main() {
  uv = pos;
  gl_Position = vec4(pos + pc.offset, 0.0, 1.0);
}"
  }
}
mod game_fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    src: "
#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 f_color;

void main() {
  f_color = vec4(uv.x / 2 + 0.5, uv.y / 2 + 0.5, 1.0, 1.0);
}"
  }
}

pub mod ui_vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    src: "
#version 450

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 uv;
layout(location = 1) out float corner_size;
layout(location = 2) out float ratio;

layout(push_constant) uniform PushData {
  vec2 pos;
  vec2 size;
  float corner_size;
} pc;

void main() {
  uv = (pos + 1) / 2;
  corner_size = pc.corner_size;
  ratio = pc.size.y / pc.size.x;
  gl_Position = vec4(pos * pc.size + pc.pos, 0.0, 1.0);
}"
  }
}
mod ui_fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    src: "
#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in float cs;
layout(location = 2) in float ratio;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D img;

void main() {
  vec2 mapped = uv;
  float cs_x = cs * ratio;
  float cs_y = cs / ratio;
  if (uv.x < cs_x) {
    // mapped.x is within 0 - cs. We want it at 0 to 0.333
    mapped.x /= cs_x;
    mapped.x /= 3;
  } else if (uv.x > 1 - cs_x) {
    // mapped.x is within (1-cs) - 1. We want it at 0.666 to 1.
    mapped.x -= 1 - cs_x;
    mapped.x /= cs_x;
    // It is now within the range 0-1
    mapped.x /= 3;
    mapped.x += 0.666;
  } else {
    mapped.x -= cs_x;
    mapped.x /= (1 - cs_x * 2);
    // mapped.x is now within the range 0-1. We want it to be within 0.333 to 0.666
    mapped.x /= 3;
    mapped.x += 0.333;
  }
  if (uv.y < cs_y) {
    // mapped.y is within 0 - cs. We want it at 0 to 0.333
    mapped.y /= cs_y;
    mapped.y /= 3;
  } else if (uv.y > 1 - cs_y) {
    // mapped.y is within (1-cs) - 1. We want it at 0.666 to 1.
    mapped.y -= 1 - cs_y;
    mapped.y /= cs_y;
    // It is now within the range 0-1
    mapped.y /= 3;
    mapped.y += 0.666;
  } else {
    mapped.y -= cs_y;
    mapped.y /= (1 - cs_y * 2);
    // mapped.y is now within the range 0-1. We want it to be within 0.333 to 0.666
    mapped.y /= 3;
    mapped.y += 0.333;
  }
  vec4 col = texture(img, mapped);
  f_color = col;
}"
  }
}

pub fn init() -> Result<GameWindow, InitError> {
  let inst = {
    let layers = vec!["VK_LAYER_KHRONOS_validation"];

    let extensions = vulkano_win::required_extensions();
    Instance::new(None, &extensions, layers)
      .map_err(|e| InitError::new(format!("failed to create vulkan instance: {}", e)))?
  };

  let physical =
    PhysicalDevice::enumerate(&inst).next().ok_or(InitError::new("no vulkan devices available"))?;
  let queue_family = physical
    .queue_families()
    .find(|q| q.supports_graphics())
    .ok_or(InitError::new("no vulkan queue families support graphics"))?;

  info!("using device: {} (type: {:?})", physical.name(), physical.ty());

  let (device, mut queues) = {
    let device_ext = vulkano::device::DeviceExtensions {
      khr_swapchain: true,
      ..vulkano::device::DeviceExtensions::none()
    };
    Device::new(physical, &Features::none(), &device_ext, [(queue_family, 0.5)].iter().cloned())
      .map_err(|e| InitError::new(format!("failed to create a vulkan device: {}", e)))?
  };

  let queue = queues.next().unwrap();

  let v1 = Vert::new(-0.5, -0.5);
  let v2 = Vert::new(0.0, 0.5);
  let v3 = Vert::new(0.5, -0.25);

  CpuAccessibleBuffer::from_iter(
    device.clone(),
    BufferUsage::all(),
    false,
    vec![v1, v2, v3].into_iter(),
  )
  .unwrap();

  let game_vs = game_vs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create vertex shader: {}", e)))?;
  let game_fs = game_fs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create fragment shader: {}", e)))?;

  let ui_vs = ui_vs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create vertex shader: {}", e)))?;
  let ui_fs = ui_fs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create fragment shader: {}", e)))?;

  let event_loop = EventLoop::new();
  let surface =
    VkSurfaceBuild::build_vk_surface(WindowBuilder::new(), &event_loop, inst.clone())
      .map_err(|e| InitError::new(format!("error while creating window surface: {}", e)))?;

  if !surface
    .is_supported(queue.family())
    .map_err(|e| InitError::new(format!("failed to get surface support: {}", e)))?
  {
    return Err(InitError::new("swapchain surface does not support this queue family"));
  }

  let caps = surface
    .capabilities(physical)
    .map_err(|e| InitError::new(format!("failed to get surface capabilities: {}", e)))?;

  let dims = caps.current_extent.unwrap_or([1920, 1080]);
  // let dims = [800, 600];
  let alpha = caps.supported_composite_alpha.iter().next().unwrap();
  let format = caps.supported_formats[0].0;

  let (swapchain, images) = Swapchain::start(device.clone(), surface.clone())
    .num_images(caps.min_image_count)
    .format(format)
    .dimensions(dims)
    .usage(ImageUsage::color_attachment())
    .sharing_mode(&queue)
    .composite_alpha(alpha)
    .build()
    .map_err(|e| InitError::new(format!("failed to create swapchain: {}", e)))?;

  let render_pass = Arc::new(
    vulkano::single_pass_renderpass!(device.clone(),
      attachments: {
        color: {
          load: Clear,
          store: Store,
          format: format,
          samples: 1,
        }
      },
      pass: {
        color: [color],
        depth_stencil: {}
      }
    )
    .unwrap(),
  );

  // let framebuffer =
  //   Arc::new(Framebuffer::start(render_pass.clone()).add(image.clone()).
  // unwrap().build().unwrap());

  let dyn_state = DynamicState {
    line_width:   None,
    viewports:    None,
    scissors:     None,
    compare_mask: None,
    write_mask:   None,
    reference:    None,
  };

  let game_pipeline = Arc::new(
    GraphicsPipeline::start()
      .vertex_input_single_buffer::<Vert>()
      .vertex_shader(game_vs.main_entry_point(), ())
      .triangle_list()
      .viewports_dynamic_scissors_irrelevant(1)
      .fragment_shader(game_fs.main_entry_point(), ())
      .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
      .build(device.clone())
      .unwrap(),
  );
  let ui_pipeline = Arc::new(
    GraphicsPipeline::start()
      .vertex_input_single_buffer::<Vert>()
      .vertex_shader(ui_vs.main_entry_point(), ())
      .triangle_list()
      .viewports_dynamic_scissors_irrelevant(1)
      .fragment_shader(ui_fs.main_entry_point(), ())
      .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
      .build(device.clone())
      .unwrap(),
  );

  let initial_future = Some(sync::now(device.clone()).boxed());
  let mut data = WindowData {
    render_pass,
    buffers: vec![],
    device,
    queue,
    game_pipeline,
    ui_pipeline,
    swapchain,
    dyn_state,
    format,
  };
  data.resize(images);

  Ok(GameWindow { event_loop, data, initial_future })
}

impl GameWindow {
  pub fn run(self, ui: Arc<UI>) -> ! {
    let mut data = self.data;

    let vertex_buffer = {
      CpuAccessibleBuffer::from_iter(
        data.device.clone(),
        BufferUsage::all(),
        false,
        [Vert::new(-0.5, -0.25), Vert::new(0.0, 0.5), Vert::new(0.25, -0.1)].iter().cloned(),
      )
      .unwrap()
    };

    let mut pc = game_vs::ty::PushData { offset: [0.0, 0.0] };
    let start = Instant::now();

    let mut previous_frame_fut = self.initial_future;
    let mut resize = false;

    self.event_loop.run(move |event, _, control_flow| match event {
      Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
        resize = true;
      }
      Event::RedrawEventsCleared => {
        previous_frame_fut.as_mut().unwrap().cleanup_finished();

        if resize {
          if data.recreate_swapchain() {
            return;
          }
          resize = false;
        }

        let (img_num, suboptimal, acquire_fut) =
          match swapchain::acquire_next_image(data.swapchain.clone(), None) {
            Ok(v) => v,
            Err(AcquireError::OutOfDate) => {
              // We just want to re-try the render of this happens
              resize = true;
              return;
            }
            Err(e) => panic!("error acquiring frame: {}", e),
          };
        if suboptimal {
          info!("suboptimal");
          resize = true;
        }

        pc.offset[1] = Instant::now().duration_since(start).as_secs_f32() % 1.0 - 0.5;

        let mut builder = AutoCommandBufferBuilder::primary(
          data.device.clone(),
          data.queue.family(),
          CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
          .begin_render_pass(
            data.buffers[img_num].clone(),
            SubpassContents::Inline,
            vec![[0.0, 0.0, 0.0, 0.0].into()],
          )
          .unwrap();

        builder
          .draw(data.game_pipeline.clone(), &data.dyn_state, vertex_buffer.clone(), (), pc, [])
          .unwrap();
        ui.draw(&mut builder, data.ui_pipeline.clone(), &data.dyn_state);

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();

        let fut = previous_frame_fut
          .take()
          .unwrap()
          .join(acquire_fut)
          .then_execute(data.queue.clone(), command_buffer)
          .unwrap()
          .then_swapchain_present(data.queue.clone(), data.swapchain.clone(), img_num)
          .then_signal_fence_and_flush();

        match fut {
          Ok(fut) => {
            // Fixes dumb nvidia big mode
            fut.wait(None).unwrap();
            previous_frame_fut = Some(fut.boxed());
          }
          Err(FlushError::OutOfDate) => {
            resize = true;
            previous_frame_fut = Some(sync::now(data.device.clone()).boxed());
          }
          Err(e) => {
            error!("failed to flush future: {:?}", e);
            previous_frame_fut = Some(sync::now(data.device.clone()).boxed());
          }
        }
      }
      _ => (),
    });
  }

  pub fn add_initial_future<F>(&mut self, f: F)
  where
    F: GpuFuture + 'static,
  {
    match self.initial_future.take() {
      Some(fut) => self.initial_future = Some(fut.join(f).boxed()),
      None => self.initial_future = Some(f.boxed()),
    }
  }
}

impl WindowData {
  /// Recreates the swapchain. Returns true if this needs to try again.
  fn recreate_swapchain(&mut self) -> bool {
    let dims: [u32; 2] = self.swapchain.surface().window().inner_size().into();
    info!("resizing to {:?}", dims);

    let (new_swapchain, new_images) = match self.swapchain.recreate().dimensions(dims).build() {
      Ok(r) => r,
      // This error tends to happen when the user is manually resizing the window.
      // Simply restarting the loop is the easiest way to fix this issue.
      Err(SwapchainCreationError::UnsupportedDimensions) => return true,
      Err(e) => panic!("failed to recreate swapchain: {}", e),
    };
    self.swapchain = new_swapchain;
    self.resize(new_images);
    false
  }
  /// Updates the internal framebuffers and viewport with the given images.
  /// Should be called when the swapchain is created. Otherwise, this will be
  /// called by recreate_swapchain when needed.
  fn resize(&mut self, images: Vec<Arc<SwapchainImage<Window>>>) {
    let dims: [u32; 2] = images[0].dimensions();

    let viewport = Viewport {
      origin:      [0.0, 0.0],
      dimensions:  [dims[0] as f32, dims[1] as f32],
      depth_range: 0.0..1.0,
    };
    self.dyn_state.viewports = Some(vec![viewport]);

    self.buffers = images
      .into_iter()
      .map(|img| {
        Arc::new(
          Framebuffer::start(self.render_pass.clone())
            .add(ImageView::new(img).unwrap())
            .unwrap()
            .build()
            .unwrap(),
        )
      })
      .collect::<Vec<_>>()
  }

  pub fn ui_pipeline(
    &self,
  ) -> &Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  > {
    &self.ui_pipeline
  }
  pub fn swapchain(&self) -> &Arc<Swapchain<Window>> {
    &self.swapchain
  }
  pub fn queue(&self) -> &Arc<Queue> {
    &self.queue
  }
  pub fn device(&self) -> &Arc<Device> {
    &self.device
  }
  pub fn format(&self) -> Format {
    self.format
  }
}

impl Deref for GameWindow {
  type Target = WindowData;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}
