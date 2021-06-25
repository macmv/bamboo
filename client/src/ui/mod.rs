use crate::{
  graphics,
  graphics::{GameWindow, Vert},
};
use png;
use std::{fs::File, sync::Arc};
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
  descriptor::{
    descriptor_set::{
      PersistentDescriptorSet, PersistentDescriptorSetImg, PersistentDescriptorSetSampler,
    },
    PipelineLayoutAbstract,
  },
  format::Format,
  image::{view::ImageView, ImageDimensions, ImmutableImage, MipmapsCount},
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
};

pub struct UI {
  // set: Arc<
  //   PersistentDescriptorSet<(
  //     ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
  //     PersistentDescriptorSetSampler,
  //   )>,
  // >,
  vbuf: Arc<CpuAccessibleBuffer<[Vert]>>,
}

impl UI {
  pub fn new(win: &mut GameWindow) -> Self {
    let (tex, tex_fut) = {
      let f = File::open("client/assets/textures/ui/button-down.png").unwrap();
      let decoder = png::Decoder::new(f);
      let (info, mut reader) = decoder.read_info().unwrap();
      let dimensions = ImageDimensions::Dim2d {
        width:        info.width,
        height:       info.height,
        array_layers: 1,
      };
      let mut image_data = Vec::new();
      image_data.resize((info.width * info.height * 4) as usize, 0);
      reader.next_frame(&mut image_data).unwrap();

      let (image, future) = ImmutableImage::from_iter(
        image_data.iter().cloned(),
        dimensions,
        MipmapsCount::One,
        Format::R8G8B8A8Srgb,
        win.queue().clone(),
      )
      .unwrap();
      (ImageView::new(image).unwrap(), future)
    };

    win.add_initial_future(tex_fut);

    let sampler = Sampler::new(
      win.device().clone(),
      Filter::Linear,
      Filter::Linear,
      MipmapMode::Linear,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      0.0,
      1.0,
      0.0,
      0.0,
    )
    .unwrap();

    // let layout = win.ui_pipeline().layout().descriptor_set_layout(0).unwrap();
    // let set = Arc::new(
    //   PersistentDescriptorSet::start(layout.clone())
    //     .add_sampled_image(tex, sampler)
    //     .unwrap()
    //     .build()
    //     .unwrap(),
    // );

    let vbuf = CpuAccessibleBuffer::from_iter(
      win.device().clone(),
      BufferUsage::all(),
      false,
      [Vert::new(-1.0, -1.0), Vert::new(0.0, 1.0), Vert::new(0.25, -1.0)].iter().cloned(),
    )
    .unwrap();

    // UI { set, vbuf }
    UI { vbuf }
  }

  pub fn draw(
    &self,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<
      GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
    >,
    dyn_state: &DynamicState,
  ) {
    let pc = graphics::ui_vs::ty::PushData { pos: [0.3, 0.4], size: [0.2, 0.5] };
    // builder.draw(pipeline, dyn_state, self.vbuf.clone(), self.set.clone(), (),
    // []).unwrap();
    builder.draw(pipeline, dyn_state, self.vbuf.clone(), (), pc, []).unwrap();
  }
}
