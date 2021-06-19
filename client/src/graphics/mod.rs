use std::{error::Error, fmt, sync::Arc, time::Instant};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents},
  descriptor::pipeline_layout::PipelineLayoutAbstract,
  device::{Device, Features, Queue},
  image::{view::ImageView, ImageUsage, SwapchainImage},
  instance::{Instance, PhysicalDevice},
  pipeline::{vertex::SingleBufferDefinition, viewport::Viewport, GraphicsPipeline},
  render_pass::{Framebuffer, FramebufferAbstract, RenderPass, Subpass},
  swapchain::{
    self, AcquireError, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
    SwapchainCreationError,
  },
  sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::{Window, WindowBuilder},
};

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
}

struct WindowData {
  render_pass: Arc<RenderPass>,
  device:      Arc<Device>,
  queue:       Arc<Queue>,
  pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  buffers:     Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
  dyn_state:   DynamicState,
  swapchain:   Arc<Swapchain<Window>>,
}

#[derive(Default, Copy, Clone)]
struct Vertex {
  position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

mod vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    src: "
#version 450

layout(location = 0) in vec2 position;

layout(push_constant) uniform PushData {
  vec2 offset;
} pc;

void main() {
  gl_Position = vec4(position + pc.offset, 0.0, 1.0);
}"
  }
}
mod fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    src: "
#version 450

layout(location = 0) out vec4 f_color;

void main() {
  f_color = vec4(1.0, 0.0, 0.0, 1.0);
}"
  }
}

pub fn init() -> Result<GameWindow, InitError> {
  let inst = {
    let extensions = vulkano_win::required_extensions();
    Instance::new(None, &extensions, None)
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

  let v1 = Vertex { position: [-0.5, -0.5] };
  let v2 = Vertex { position: [0.0, 0.5] };
  let v3 = Vertex { position: [0.5, -0.25] };

  CpuAccessibleBuffer::from_iter(
    device.clone(),
    BufferUsage::all(),
    false,
    vec![v1, v2, v3].into_iter(),
  )
  .unwrap();

  let vs = vs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create vertex shader: {}", e)))?;
  let fs = fs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create fragment shader: {}", e)))?;

  let event_loop = EventLoop::new();
  let surface =
    VkSurfaceBuild::build_vk_surface(WindowBuilder::new(), &event_loop, inst.clone())
      .map_err(|e| InitError::new(format!("error while creating window surface: {}", e)))?;

  let caps = surface
    .capabilities(physical)
    .map_err(|e| InitError::new(format!("failed to get surface capabilities: {}", e)))?;

  let dims = caps.current_extent.unwrap_or([1920, 1080]);
  // let dims = [800, 600];
  let alpha = caps.supported_composite_alpha.iter().next().unwrap();
  let format = caps.supported_formats[0].0;

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

  let (swapchain, images) = Swapchain::start(device.clone(), surface.clone())
    .num_images(caps.min_image_count)
    .format(format)
    .dimensions(dims)
    .layers(1)
    .usage(ImageUsage::color_attachment())
    .sharing_mode(&queue)
    .transform(SurfaceTransform::Identity)
    .composite_alpha(alpha)
    .present_mode(PresentMode::Fifo)
    .fullscreen_exclusive(FullscreenExclusive::Default)
    .build()
    .map_err(|e| InitError::new(format!("failed to create swapchain: {}", e)))?;

  let pipeline = Arc::new(
    GraphicsPipeline::start()
      .vertex_input_single_buffer::<Vertex>()
      .vertex_shader(vs.main_entry_point(), ())
      .viewports_dynamic_scissors_irrelevant(1)
      .fragment_shader(fs.main_entry_point(), ())
      .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
      .build(device.clone())
      .unwrap(),
  );

  let mut data =
    WindowData { render_pass, buffers: vec![], device, queue, pipeline, swapchain, dyn_state };
  data.resize(images);

  Ok(GameWindow { event_loop, data })
}

impl GameWindow {
  pub fn run(self) -> ! {
    let mut data = self.data;

    let vertex_buffer = {
      CpuAccessibleBuffer::from_iter(
        data.device.clone(),
        BufferUsage::all(),
        false,
        [
          Vertex { position: [-0.5, -0.25] },
          Vertex { position: [0.0, 0.5] },
          Vertex { position: [0.25, -0.1] },
        ]
        .iter()
        .cloned(),
      )
      .unwrap()
    };

    let mut push_constants = vs::ty::PushData { offset: [0.0, 0.0] };
    let start = Instant::now();

    let mut previous_frame_end = Some(sync::now(data.device.clone()).boxed());
    let mut resize = false;

    self.event_loop.run(move |event, _, control_flow| match event {
      Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::RedrawEventsCleared => {
        previous_frame_end.as_mut().unwrap().cleanup_finished();

        if resize {
          // TODO: Recreate swapchain here, without freezing my computer
          resize = false;
        }
        data.recreate_swapchain();

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

        push_constants.offset[1] = Instant::now().duration_since(start).as_secs_f32() % 1.0 - 0.5;

        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

        let mut builder = AutoCommandBufferBuilder::primary(
          data.device.clone(),
          data.queue.family(),
          CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
          .begin_render_pass(data.buffers[img_num].clone(), SubpassContents::Inline, clear_values)
          .unwrap()
          .draw(
            data.pipeline.clone(),
            &data.dyn_state,
            vertex_buffer.clone(),
            (),
            push_constants,
            [],
          )
          .unwrap()
          .end_render_pass()
          .unwrap();

        let command_buffer = builder.build().unwrap();

        let fut = previous_frame_end
          .take()
          .unwrap()
          .join(acquire_fut)
          .then_execute(data.queue.clone(), command_buffer)
          .unwrap()
          .then_swapchain_present(data.queue.clone(), data.swapchain.clone(), img_num)
          .then_signal_fence_and_flush();

        match fut {
          Ok(fut) => {
            previous_frame_end = Some(fut.boxed());
          }
          Err(FlushError::OutOfDate) => {
            resize = true;
            previous_frame_end = Some(sync::now(data.device.clone()).boxed());
          }
          Err(e) => {
            error!("failed to flush future: {:?}", e);
            previous_frame_end = Some(sync::now(data.device.clone()).boxed());
          }
        }
      }
      _ => (),
    });
  }
}

impl WindowData {
  fn recreate_swapchain(&mut self) {
    let dims: [u32; 2] = self.swapchain.surface().window().inner_size().into();

    let (new_swapchain, new_images) = match self.swapchain.recreate().dimensions(dims).build() {
      Ok(r) => r,
      // This error tends to happen when the user is manually resizing the window.
      // Simply restarting the loop is the easiest way to fix this issue.
      Err(SwapchainCreationError::UnsupportedDimensions) => return,
      Err(e) => panic!("failed to recreate swapchain: {}", e),
    };
    self.swapchain = new_swapchain;
    self.resize(new_images);
  }
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
        ) as Arc<dyn FramebufferAbstract + Send + Sync>
      })
      .collect::<Vec<_>>()
  }
}
