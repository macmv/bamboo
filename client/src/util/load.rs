use png as png_mod;
use std::{error::Error, fmt, fs::File, io, sync::Arc};
use vulkano::{
  command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
  device::Queue,
  format::Format,
  image::{view::ImageView, ImageDimensions, ImmutableImage, MipmapsCount},
  sync::NowFuture,
};

#[derive(Debug)]
pub enum PngError {
  IO(io::Error),
  InvalidFormat(png::ColorType),
  Decoding(png::DecodingError),
}

impl fmt::Display for PngError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::IO(e) => write!(f, "{}", e),
      Self::InvalidFormat(e) => write!(f, "invalid format: {:?} (format must be RGBA)", e),
      Self::Decoding(e) => write!(f, "decoder error: {}", e),
    }
  }
}

impl Error for PngError {}

pub fn png(
  path: &str,
  queue: Arc<Queue>,
) -> Option<(
  Arc<ImageView<Arc<ImmutableImage>>>,
  CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>,
)> {
  match png_err(path, queue) {
    Ok(v) => Some(v),
    Err(e) => {
      error!("error while loading {}: {}", path, e);
      None
    }
  }
}

pub fn png_err(
  path: &str,
  queue: Arc<Queue>,
) -> Result<
  (
    Arc<ImageView<Arc<ImmutableImage>>>,
    CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>,
  ),
  PngError,
> {
  let (data, dimensions) = png_data(path)?;
  let (image, future) = ImmutableImage::from_iter(
    data.iter().cloned(),
    dimensions,
    MipmapsCount::One,
    Format::R8G8B8A8Srgb,
    queue,
  )
  .unwrap();
  Ok((ImageView::new(image).unwrap(), future))
}

pub fn png_data(path: &str) -> Result<(Vec<u8>, ImageDimensions), PngError> {
  let f = File::open(path).map_err(|e| PngError::IO(e))?;
  let decoder = png_mod::Decoder::new(f);
  let (info, mut reader) = decoder.read_info().map_err(|e| PngError::Decoding(e))?;
  let dimensions =
    ImageDimensions::Dim2d { width: info.width, height: info.height, array_layers: 1 };
  if info.color_type != png::ColorType::RGBA {
    return Err(PngError::InvalidFormat(info.color_type));
  }
  let mut image_data = vec![0; (info.width * info.height * 4) as usize];
  reader.next_frame(&mut image_data).map_err(|e| PngError::Decoding(e))?;
  Ok((image_data, dimensions))
}
