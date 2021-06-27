use crate::{graphics::GameWindow, util::load};
use std::sync::Arc;
use vulkano::image::{view::ImageView, ImmutableImage};

pub struct PNGRender {
  img: Arc<ImageView<Arc<ImmutableImage>>>,
}

impl PNGRender {
  /// Creates a new png text renderer. This function should be used during init,
  /// and a different function should be called if you need to reload a png
  /// render.
  pub fn new_init(path: &str, size: f32, win: &mut GameWindow) -> Self {
    let (img, fut) = load::png(path, win.queue().clone()).unwrap();
    win.add_initial_future(fut);
    PNGRender { img }
  }
}
