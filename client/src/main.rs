use std::sync::Arc;
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  device::{Device, DeviceExtensions, Features},
  format::Format,
  instance::{Instance, InstanceExtensions, PhysicalDevice},
  pipeline::GraphicsPipeline,
  render_pass::{Framebuffer, Subpass},
};
use vulkano_win::VkSurfaceBuild;
use winit::{event_loop::EventLoop, window::WindowBuilder};

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

fn main() {
  let inst = {
    let extensions = vulkano_win::required_extensions();
    Instance::new(None, &extensions, None).expect("failed to create vulkan instance")
  };

  let physical = PhysicalDevice::enumerate(&inst).next().expect("no vulkan devices available");
  let queue_family = physical
    .queue_families()
    .find(|q| q.supports_graphics())
    .expect("no vulkan queue families support graphics");

  let (device, mut queues) = {
    Device::new(
      physical,
      &Features::none(),
      &DeviceExtensions::none(),
      [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("failed to create a vulkan device")
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

  let vs = vs::Shader::load(device.clone()).expect("failed to create vertex shader");
  let fs = fs::Shader::load(device.clone()).expect("failed to create fragment shader");

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
  let surface = VkSurfaceBuild::build_vk_surface(WindowBuilder::new(), &event_loop, inst.clone());

  loop {}
}
