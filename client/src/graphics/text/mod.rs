mod png;
mod ttf;

pub use self::png::PNGRender;
pub use ttf::TTFRender;

use super::{Vert, WindowData};
use std::sync::Arc;
use vulkano::{
  command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
  descriptor::PipelineLayoutAbstract,
  device::Device,
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  render_pass::{RenderPass, Subpass},
  swapchain::Swapchain,
};

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

fn create_pipeline<W>(
  device: Arc<Device>,
  swapchain: Arc<Swapchain<W>>,
) -> Arc<
  GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
>
where
  W: Send + Sync + 'static,
{
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

  Arc::new(
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
  )
}

pub trait TextRender {
  /// This queues the given text to be rendered on the next draw call. This is
  /// required for the ttf renderer, as it needs to update a cache image. This
  /// cannot be done during a render pass, so this needs to be a seperate
  /// operation from draw().
  fn queue_text(
    &mut self,
    text: &str,
    pos: (f32, f32),
    scale: f32,
    buf: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
  );
  /// This renders all queued text onto the screen.
  fn draw(
    &mut self,
    command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    win: &WindowData,
  );
}
