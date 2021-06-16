use std::{error::Error, fmt, sync::Arc};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  device::{Device, DeviceExtensions, Features},
  format::Format,
  instance::{Instance, InstanceExtensions, PhysicalDevice},
  pipeline::GraphicsPipeline,
  render_pass::{Framebuffer, Subpass},
  swapchain::Surface,
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
  surface:    Arc<Surface<Window>>,
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

  let physical = PhysicalDevice::enumerate(&inst).next().expect("no vulkan devices available");
  let queue_family = physical
    .queue_families()
    .find(|q| q.supports_graphics())
    .ok_or(InitError::new("no vulkan queue families support graphics"))?;

  let (device, mut queues) = Device::new(
    physical,
    &Features::none(),
    &DeviceExtensions::none(),
    [(queue_family, 0.5)].iter().cloned(),
  )
  .map_err(|e| InitError::new(format!("failed to create a vulkan device: {}", e)))?;

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

  let render_pass = Arc::new(
    vulkano::single_pass_renderpass!(device.clone(),
    attachments: {
      color: {
        load: Clear,
        store: Store,
        format: Format::R8G8B8A8Unorm,
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

  let event_loop = EventLoop::new();
  let surface =
    VkSurfaceBuild::build_vk_surface(WindowBuilder::new(), &event_loop, inst.clone())
      .map_err(|e| InitError::new(format!("error while creating window surface: {}", e)))?;

  Ok(GameWindow { event_loop, surface })
}

impl GameWindow {
  pub fn run(self) -> ! {
    self.event_loop.run(|event, _, control_flow| match event {
      Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
        *control_flow = ControlFlow::Exit;
      }
      _ => (),
    });
  }
}
