use crate::{World, UI};
use rand::Rng;
use std::{
  error::Error,
  fmt,
  ops::Deref,
  sync::{Arc, Mutex},
};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
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
  event::{ElementState, Event, MouseButton, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::{Window, WindowBuilder},
};

mod chunk;
mod text;
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
    GraphicsPipeline<SingleBufferDefinition<Vert3>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  ui_pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert2>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  menu_pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert2>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  buffers:       Vec<Arc<Framebuffer<((), Arc<ImageView<Arc<SwapchainImage<Window>>>>)>>>,
  dyn_state:     DynamicState,
  swapchain:     Arc<Swapchain<Window>>,
  format:        Format,

  width:        u32,
  height:       u32,
  mouse_x:      f64,
  mouse_y:      f64,
  prev_mouse_x: f64,
  prev_mouse_y: f64,

  // If this is Some, then we are ingame, and should call render() on this.
  world: Option<Arc<World>>,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Vert2 {
  pos: [f32; 2],
}
vulkano::impl_vertex!(Vert2, pos);

#[derive(Debug, Default, Copy, Clone)]
pub struct Vert3 {
  pos: [f32; 3],
  uv:  [f32; 2],
}
vulkano::impl_vertex!(Vert3, pos, uv);

impl Vert2 {
  pub fn new(x: f32, y: f32) -> Self {
    Vert2 { pos: [x, y] }
  }

  #[inline(always)]
  pub fn x(&self) -> f32 {
    self.pos[0]
  }
  #[inline(always)]
  pub fn y(&self) -> f32 {
    self.pos[1]
  }

  #[inline(always)]
  pub fn set_x(&mut self, v: f32) {
    self.pos[0] = v;
  }
  #[inline(always)]
  pub fn set_y(&mut self, v: f32) {
    self.pos[1] = v;
  }
}

impl Vert3 {
  pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
    Vert3 { pos: [x, y, z], uv: [u, v] }
  }

  #[inline(always)]
  pub fn x(&self) -> f32 {
    self.pos[0]
  }
  #[inline(always)]
  pub fn y(&self) -> f32 {
    self.pos[1]
  }
  #[inline(always)]
  pub fn z(&self) -> f32 {
    self.pos[2]
  }
  #[inline(always)]
  pub fn u(&self) -> f32 {
    self.uv[0]
  }
  #[inline(always)]
  pub fn v(&self) -> f32 {
    self.uv[1]
  }
}

impl Into<[f32; 2]> for Vert2 {
  fn into(self) -> [f32; 2] {
    self.pos
  }
}
impl Into<[f32; 3]> for Vert3 {
  fn into(self) -> [f32; 3] {
    self.pos
  }
}

pub mod game_vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    path: "src/shader/game.vs"
  }
}
mod game_fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    path: "src/shader/game.fs"
  }
}

pub mod ui_vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    path: "src/shader/ui.vs"
  }
}
mod ui_fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    path: "src/shader/ui.fs"
  }
}

mod menu_vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    path: "src/shader/menu.vs"
  }
}
mod menu_fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    path: "src/shader/menu.fs"
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
    world: None,
  };
  data.resize(images);

  Ok(GameWindow { event_loop, data, initial_future })
}

impl GameWindow {
  pub fn run(self, ui: Arc<UI>) -> ! {
    // let mut text: Box<dyn TextRender> = Box::new(PNGRender::new(
    //   "/home/macmv/.minecraft/resourcepacks/ocd/assets/minecraft/textures/font/
    // ascii.png",
    //   32.0,
    //   &mut self,
    // ));

    let vbuf = {
      CpuAccessibleBuffer::from_iter(
        self.data.device.clone(),
        BufferUsage::all(),
        false,
        [Vert2::new(-0.5, -0.25), Vert2::new(0.0, 0.5), Vert2::new(0.25, -0.1)].iter().cloned(),
      )
      .unwrap()
    };
    let mut vels = [Vert2::new(0.0, 0.0); 3];
    let mut rng = rand::thread_rng();

    let data = Arc::new(Mutex::new(self.data));

    let mut previous_frame_fut = self.initial_future;
    let mut resize = false;

    self.event_loop.run(move |event, _, control_flow| match event {
      Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
        resize = true;
      }
      Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
        data.lock().unwrap().mouse_moved(position.x, position.y);
      }
      Event::WindowEvent { event: WindowEvent::MouseInput { state, button, .. }, .. } => {
        if state == ElementState::Pressed && button == MouseButton::Left {
          ui.on_click(&data);
        }
      }
      Event::RedrawEventsCleared => {
        previous_frame_fut.as_mut().unwrap().cleanup_finished();
        let mut data = data.lock().unwrap();

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

        let mut builder = AutoCommandBufferBuilder::primary(
          data.device.clone(),
          data.queue.family(),
          CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // text.queue_text("big", (0.0, 0.0), 1.0, &mut builder);

        builder
          .begin_render_pass(
            data.buffers[img_num].clone(),
            SubpassContents::Inline,
            vec![[0.0, 0.0, 0.0, 0.0].into()],
          )
          .unwrap();

        if let Some(ref world) = data.world {
          world.clone().render(&mut data, &mut builder);
        } else {
          // In a menu
          {
            let mut buf = vbuf.write().unwrap();
            // Center of all the points
            let mut avg = Vert2::new(0.0, 0.0);
            for p in buf.iter() {
              avg.set_x(avg.x() + p.x());
              avg.set_y(avg.y() + p.y());
            }
            avg.set_x(avg.x() / vbuf.len() as f32);
            avg.set_x(avg.x() / vbuf.len() as f32);

            for (i, p) in buf.iter_mut().enumerate() {
              let v = vels.get_mut(i).unwrap();
              // Push in a random direction
              let mut accel_x = (rng.gen::<f32>() - 0.5) / 1000.0;
              let mut accel_y = (rng.gen::<f32>() - 0.5) / 1000.0;
              // Push away from the edges of the screen
              accel_x -= p.x() / 2000.0;
              accel_y -= p.y() / 2000.0;
              // Push towards the average of the points
              accel_x += (p.x() - avg.x()) / 10000.0;
              accel_y += (p.y() - avg.y()) / 10000.0;

              // Apply the acceleration
              v.set_x(v.x() + accel_x);
              v.set_y(v.y() + accel_y);
              // Add a bit of damping
              v.set_x(v.x() * 0.9995);
              v.set_y(v.y() * 0.9995);
              // Apply the velocity
              p.set_x(p.x() + v.x());
              p.set_y(p.y() + v.y());
            }
          }

          builder
            .draw(data.menu_pipeline.clone(), &data.dyn_state, vbuf.clone(), (), (), [])
            .unwrap();
        }
        ui.draw(&mut builder, data.ui_pipeline.clone(), &data.dyn_state, &data);
        // text.draw(&mut builder, &data);

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
    self.width = dims[0];
    self.height = dims[1];

    let viewport = Viewport {
      origin:      [0.0, 0.0],
      dimensions:  [self.width as f32, self.height as f32],
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

  /// Updates the internal mouse position
  fn mouse_moved(&mut self, x: f64, y: f64) {
    self.mouse_x = x;
    self.mouse_y = y;
  }

  /// Returns the mouse delta since the last time this function was called. This
  /// will update an internal value, so calling it multiple times will only
  /// return a useful value the first time.
  pub fn mouse_delta(&mut self) -> (f64, f64) {
    let dx = self.mouse_x - self.prev_mouse_x;
    let dy = self.mouse_y - self.prev_mouse_y;
    self.prev_mouse_x = self.mouse_x;
    self.prev_mouse_y = self.mouse_y;
    (dx, dy)
  }

  /// Returns the mouse position in pixels.
  pub fn mouse_pos(&self) -> (f64, f64) {
    (self.mouse_x, self.mouse_y)
  }
  /// Returns the mouse position in screen space (each coordinate is within -1.0
  /// to 1.0).
  pub fn mouse_screen_pos(&self) -> (f64, f64) {
    let (x, y) = self.mouse_pos();
    (x / (self.width as f64) * 2.0 - 1.0, y / (self.height as f64) * 2.0 - 1.0)
  }

  #[inline(always)]
  pub fn game_pipeline(
    &self,
  ) -> &Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert3>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  > {
    &self.game_pipeline
  }
  #[inline(always)]
  pub fn ui_pipeline(
    &self,
  ) -> &Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert2>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  > {
    &self.ui_pipeline
  }
  #[inline(always)]
  pub fn swapchain(&self) -> &Arc<Swapchain<Window>> {
    &self.swapchain
  }
  #[inline(always)]
  pub fn dyn_state(&self) -> &DynamicState {
    &self.dyn_state
  }
  #[inline(always)]
  pub fn queue(&self) -> &Arc<Queue> {
    &self.queue
  }
  #[inline(always)]
  pub fn device(&self) -> &Arc<Device> {
    &self.device
  }
  #[inline(always)]
  pub fn format(&self) -> Format {
    self.format
  }
  #[inline(always)]
  pub fn width(&self) -> u32 {
    self.width
  }
  #[inline(always)]
  pub fn height(&self) -> u32 {
    self.height
  }

  /// Starts rendering a game from first person. This is how the application
  /// moves from the main menu into a game.
  pub fn start_ingame(&mut self, world: Arc<World>) {
    self.world = Some(world)
  }
}

impl Deref for GameWindow {
  type Target = WindowData;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}
