use png as png_mod;
use std::{fs::File, sync::Arc};
use vulkano::{
  command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
  device::Queue,
  format::Format,
  image::{view::ImageView, ImageDimensions, ImmutableImage, MipmapsCount},
  sync::NowFuture,
};

pub fn png(
  path: &str,
  queue: Arc<Queue>,
) -> Option<(
  Arc<ImageView<Arc<ImmutableImage>>>,
  CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>,
)> {
  let f = match File::open(path) {
    Err(e) => {
      error!("error while loading {}: {}", path, e);
      return None;
    }
    Ok(f) => f,
  };
  let decoder = png_mod::Decoder::new(f);
  let (info, mut reader) = match decoder.read_info() {
    Err(e) => {
      error!("error while loading {}: {}", path, e);
      return None;
    }
    Ok(f) => f,
  };
  let dimensions =
    ImageDimensions::Dim2d { width: info.width, height: info.height, array_layers: 1 };
  if info.color_type != png::ColorType::RGBA {
    error!(
      "error while loading {}: invalid color format {:?} (format must be RGBA)",
      path, info.color_type
    );
    return None;
  }
  let mut image_data = vec![0; (info.width * info.height * 4) as usize];
  if let Err(e) = reader.next_frame(&mut image_data) {
    error!("error while loading {}: {}", path, e);
    return None;
  }

  let (image, future) = ImmutableImage::from_iter(
    image_data.iter().cloned(),
    dimensions,
    MipmapsCount::One,
    Format::R8G8B8A8Srgb,
    queue,
  )
  .unwrap();
  Some((ImageView::new(image).unwrap(), future))
}
