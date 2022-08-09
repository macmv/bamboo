use crate::event::EventFlow;
use bb_server_macros::define_ty;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PEventFlow {
  pub cancelled: Arc<Mutex<Option<EventFlow>>>,
}

impl PEventFlow {
  pub fn new() -> Self { PEventFlow { cancelled: Arc::new(Mutex::new(None)) } }
}

#[define_ty(panda_path = "bamboo::event::Flow")]
impl PEventFlow {
  /// Cancells this event. This will stop the caller from contining to process
  /// this event.
  ///
  /// You cannot cancel an event twice, so calling this a second time will throw
  /// an error.
  pub fn cancel(&self) { *self.cancelled.lock() = Some(EventFlow::Continue); }
  /// Allows this event to continue, freeing up the event caller to continue
  /// processing, while allowing the event handler to also process in parallel.
  ///
  /// You cannot allow an event twice, so calling this a second time will throw
  /// an error.
  pub fn allow(&self) { *self.cancelled.lock() = Some(EventFlow::Continue); }
  /// Checks if this event has either been allowed or cancelled yet. If true,
  /// then calling `cancel` or `allow` will throw an error.
  pub fn is_set(&self) -> bool { self.cancelled.lock().is_some() }
  /// Checks if this event has been cancelled yet. If `true`, then calling
  /// `cancel` or `allow` will return an error.
  pub fn is_cancelled(&self) -> bool { self.cancelled.lock().is_some() }
  /// Checks if this event has either been allowed or cancelled yet. If true,
  /// then calling `cancel` or `allow` will throw an error.
  pub fn is_allowed(&self) -> bool { self.cancelled.lock().is_some() }
}
