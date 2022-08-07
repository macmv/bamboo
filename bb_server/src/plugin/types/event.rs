use super::{add_from, wrap, wrap_eq};
use bb_common::{
  math::{ChunkPos, FPos, Pos},
  util::UUID,
};
use bb_server_macros::define_ty;

pub struct PEvent {
  data:      HashMap<String, Var>,
  cancelled: bool,
}

#[define_ty(panda_path = "bamboo::event::Event")]
impl PEvent {
  pub fn data(&self) -> HashMap<String, Var> { self.data.clone() }

  pub fn cancel(&mut self) { self.cancelled = true; }
}
