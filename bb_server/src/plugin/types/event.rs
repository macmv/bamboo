use bb_server_macros::define_ty;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PEvent {
  pub cancelled: Arc<Mutex<bool>>,
}

impl PEvent {
  pub fn new() -> Self { PEvent { cancelled: Arc::new(Mutex::new(false)) } }
}

#[define_ty(panda_path = "bamboo::event::Event")]
impl PEvent {
  pub fn cancel(&self) { *self.cancelled.lock() = true; }
  pub fn is_cancelled(&self) -> bool { *self.cancelled.lock() }
}
