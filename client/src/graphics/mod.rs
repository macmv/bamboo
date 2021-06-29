use crate::{World, UI};
use common::math::Pos;
use num_traits::FromPrimitive;
use rand::Rng;
use std::{
  convert::TryInto,
  ops::{Add, AddAssign, Deref},
  sync::{Arc, Mutex},
};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
  command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents},
  descriptor::pipeline_layout::PipelineLayoutAbstract,
  device::{Device, Queue},
  format::Format,
  image::{view::ImageView, SwapchainImage},
  pipeline::{vertex::SingleBufferDefinition, viewport::Viewport, GraphicsPipeline},
  render_pass::{Framebuffer, RenderPass},
  swapchain::{self, AcquireError, Swapchain, SwapchainCreationError},
  sync::{self, FlushError, GpuFuture},
};
use winit::{
  dpi::PhysicalPosition,
  event::{ElementState, Event, KeyboardInput, MouseButton, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::Window,
};

mod chunk;
mod init;
mod key;
mod text;

pub use chunk::MeshChunk;
pub use init::init;
pub use key::KeyCode;

pub struct GameWindow {
  event_loop: EventLoop<()>,
  data:       WindowData,

  initial_future: Option<Box<dyn GpuFuture>>,
}

struct KeyStates {
  // 256 bits, for the state of 256 scancodes.
  state: [u8; 32],
  // modifiers: something
}

impl KeyStates {
  pub fn new() -> Self {
    KeyStates { state: [0; 32] }
  }

  pub fn set(&mut self, code: u8, value: bool) {
    if value {
      self.state[(code / 8) as usize] |= 1 << (code % 8);
    } else {
      self.state[(code / 8) as usize] &= !(1 << (code % 8));
    }
  }

  pub fn get(&self, code: u8) -> bool {
    self.state[(code / 8) as usize] & 1 << (code % 8) != 0
  }
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
  key_states:   KeyStates,

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

impl Add<Pos> for Vert3 {
  type Output = Vert3;

  fn add(self, o: Pos) -> Vert3 {
    Vert3::new(
      self.x() + o.x() as f32,
      self.y() + o.y() as f32,
      self.z() + o.z() as f32,
      self.u(),
      self.v(),
    )
  }
}

impl AddAssign<Pos> for Vert3 {
  fn add_assign(&mut self, o: Pos) {
    self.pos[0] += o.x() as f32;
    self.pos[1] += o.y() as f32;
    self.pos[2] += o.z() as f32;
  }
}

impl Add for Vert3 {
  type Output = Vert3;

  fn add(self, o: Vert3) -> Vert3 {
    Vert3::new(
      self.x() + o.x(),
      self.y() + o.y(),
      self.z() + o.z(),
      self.u() + o.y(),
      self.v() + o.v(),
    )
  }
}

impl AddAssign for Vert3 {
  fn add_assign(&mut self, o: Vert3) {
    self.pos[0] += o.x();
    self.pos[1] += o.y();
    self.pos[2] += o.z();
    self.uv[0] += o.u();
    self.uv[1] += o.v();
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
      Event::WindowEvent { event: WindowEvent::KeyboardInput { input, .. }, .. } => {
        data.lock().unwrap().keyboard_input(input);
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

  /// Updates the internal keyboard state
  fn keyboard_input(&mut self, input: KeyboardInput) {
    // We probably got a weird value if this is above 256
    if let Ok(code) = input.scancode.try_into() {
      self.key_states.set(code, input.state == ElementState::Pressed);
      for c in 0..=255 {
        if self.key_states.get(c) {
          match KeyCode::from_u8(c) {
            Some(v) => print!("{:?}({}) ", v, c),
            None => print!("Unknown({}) ", c),
          }
        }
      }
      println!();
    }
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
    let win = self.swapchain.surface().window();
    let size = win.inner_size();
    // Set cursor to the middle of the screen. If this fails, it means we are on
    // wayland, web, ios, or android. We ignore these failures, as the cursor is
    // already grabbed.
    let x = size.width as f64 / 2.0;
    let y = size.height as f64 / 2.0;
    let _ = win.set_cursor_position(PhysicalPosition { x: x as f32, y: y as f32 });
    self.prev_mouse_x = x;
    self.prev_mouse_y = y;
    (dx, dy)
  }

  /// Returns the mouse position in pixels.
  pub fn key_down(&self, code: KeyCode) -> bool {
    self.key_states.get(code as u8)
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
    self.world = Some(world);
    let win = self.swapchain.surface().window();
    win.set_cursor_visible(false);
    win.set_cursor_grab(true).unwrap();
  }
}

impl Deref for GameWindow {
  type Target = WindowData;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}
