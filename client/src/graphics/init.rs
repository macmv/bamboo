use super::{
  game_fs, game_vs, menu_fs, menu_vs, ui_fs, ui_vs, GameWindow, KeyStates, Vert2, Vert3, WindowData,
};
use std::{error::Error, fmt, sync::Arc};
use vulkano::{
  self,
  command_buffer::DynamicState,
  device::{Device, Features},
  image::ImageUsage,
  instance::{Instance, PhysicalDevice},
  pipeline::GraphicsPipeline,
  render_pass::Subpass,
  swapchain::Swapchain,
  sync::{self, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{event_loop::EventLoop, window::WindowBuilder};

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

  let game_vs = game_vs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create vertex shader: {}", e)))?;
  let game_fs = game_fs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create fragment shader: {}", e)))?;

  let ui_vs = ui_vs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create vertex shader: {}", e)))?;
  let ui_fs = ui_fs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create fragment shader: {}", e)))?;

  let menu_vs = menu_vs::Shader::load(device.clone())
    .map_err(|e| InitError::new(format!("failed to create vertex shader: {}", e)))?;
  let menu_fs = menu_fs::Shader::load(device.clone())
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
      .vertex_input_single_buffer::<Vert3>()
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
      .vertex_input_single_buffer::<Vert2>()
      .vertex_shader(ui_vs.main_entry_point(), ())
      .triangle_list()
      .viewports_dynamic_scissors_irrelevant(1)
      .fragment_shader(ui_fs.main_entry_point(), ())
      .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
      .build(device.clone())
      .unwrap(),
  );
  let menu_pipeline = Arc::new(
    GraphicsPipeline::start()
      .vertex_input_single_buffer::<Vert2>()
      .vertex_shader(menu_vs.main_entry_point(), ())
      .triangle_list()
      .viewports_dynamic_scissors_irrelevant(1)
      .fragment_shader(menu_fs.main_entry_point(), ())
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
    menu_pipeline,
    swapchain,
    dyn_state,
    format,
    width: 0,
    height: 0,
    mouse_x: 0.0,
    mouse_y: 0.0,
    prev_mouse_x: 0.0,
    prev_mouse_y: 0.0,
    key_states: KeyStates::new(),
    world: None,
  };
  data.resize(images);

  Ok(GameWindow { event_loop, data, initial_future })
}
