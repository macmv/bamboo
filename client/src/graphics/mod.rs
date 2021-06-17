use std::{error::Error, fmt, sync::Arc};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents},
  descriptor::pipeline_layout::PipelineLayoutAbstract,
  device::{Device, Features, Queue},
  image::{
    view::{ComponentMapping, ComponentSwizzle, ImageView},
    ImageAccess, ImageUsage,
  },
  instance::{Instance, PhysicalDevice},
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  render_pass::{Framebuffer, FramebufferAbstract, Subpass},
  swapchain::{self, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain},
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
  device:     Arc<Device>,
  queue:      Arc<Queue>,
  pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  buffers:    Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
  event_loop: EventLoop<()>,
  dyn_state:  DynamicState,
  swapchain:  Arc<Swapchain<Window>>,
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

void main() {
  gl_Position = vec4(position, 0.0, 1.0);
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

  let pipeline = Arc::new(
    GraphicsPipeline::start()
      // Defines what kind of vertex input is expected.
      .vertex_input_single_buffer::<Vertex>()
      // The vertex shader.
      .vertex_shader(vs.main_entry_point(), ())
      // Defines the viewport (explanations below).
      .viewports_dynamic_scissors_irrelevant(1)
      // The fragment shader.
      .fragment_shader(fs.main_entry_point(), ())
      // This graphics pipeline object concerns the first pass of the render pass.
      .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
      // Now that everything is specified, we call `build`.
      .build(device.clone())
      .unwrap(),
  );

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

  dbg!(images[0].format());

  let buffers = images
    .into_iter()
    .map(|img| {
      Arc::new(
        Framebuffer::start(render_pass.clone())
          .add(ImageView::new(img).unwrap())
          .unwrap()
          .build()
          .unwrap(),
      ) as Arc<dyn FramebufferAbstract + Send + Sync>
    })
    .collect::<Vec<_>>();

  Ok(GameWindow { buffers, device, queue, pipeline, swapchain, dyn_state, event_loop })
}

impl GameWindow {
  pub fn run(self) -> ! {
    let swapchain = self.swapchain;
    let device = self.device;
    let queue = self.queue;
    let pipeline = self.pipeline;
    let buffers = self.buffers;
    let dyn_state = self.dyn_state;

    let vertex_buffer = {
      CpuAccessibleBuffer::from_iter(
        device.clone(),
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

    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    self.event_loop.run(move |event, _, control_flow| match event {
      Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::RedrawRequested(_) => {
        previous_frame_end.as_mut().unwrap().cleanup_finished();

        let (img_num, ok, fut) = swapchain::acquire_next_image(swapchain.clone(), None).unwrap();
        info!("got ok: {}", ok);

        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

        let mut builder = AutoCommandBufferBuilder::primary(
          device.clone(),
          queue.family(),
          CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
          // Before we can draw, we have to *enter a render pass*. There aretwo methods to do this:
          // `draw_inline` and `draw_secondary`. The latter is a bit more advanced and is
          // not covered here.
          //
          // The third parameter builds the list of values to clear the attachments with. The API
          // is similar to the list of attachments when building the framebuffers, except that only
          // the attachments that use `load: Clear` appear in the list.
          .begin_render_pass(buffers[img_num].clone(), SubpassContents::Inline, clear_values)
          .unwrap()
          // We are now inside the first subpass of the render pass. We add a draw command. The last
          // two parameters contain the list of resources to pass to the shaders. Since we used an
          // `EmptyPipeline` object, the objects have to be `()`.
          .draw(pipeline.clone(), &dyn_state, vertex_buffer.clone(), (), (), [])
          .unwrap()
          // We leave the render pass by calling `draw_end`. Note that if we had multiple subpasses
          // we could have called `next_inline` (or `next_secondary`) to jump to the next subpass.
          .end_render_pass()
          .unwrap();

        let command_buffer = builder.build().unwrap();

        let future = previous_frame_end
          .take()
          .unwrap()
          .join(fut)
          .then_execute(queue.clone(), command_buffer)
          .unwrap()
          // The color output is now expected to contain our triangle. But in order to show it on
          // the screen, we have to *present* the image by calling `present`.
          //
          // This function does not actually present the image immediately. Instead it submits a
          // present command at the end of the queue. This means that it will only be presented once
          // the GPU has finished executing the command buffer that draws the triangle.
          .then_swapchain_present(queue.clone(), swapchain.clone(), img_num)
          .then_signal_fence_and_flush();

        match future {
          Ok(future) => {
            previous_frame_end = Some(future.boxed());
          }
          Err(FlushError::OutOfDate) => {
            // recreate_swapchain = true;
            previous_frame_end = Some(sync::now(device.clone()).boxed());
          }
          Err(e) => {
            error!("failed to flush future: {:?}", e);
            previous_frame_end = Some(sync::now(device.clone()).boxed());
          }
        }
      }
      _ => (),
    });
  }
}
