use std::sync::atomic::{AtomicU8, Ordering};

pub struct WasmLock {
  // 0 for unlocked, 1 for locked.
  state: AtomicU8,
}

impl WasmLock {
  pub const fn new() -> Self { WasmLock { state: AtomicU8::new(0) } }
  pub fn lock(&self) {
    // TODO: Spin locking, or something smarter than this?
    while self.state.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {}
  }
  pub fn try_lock(&self) -> bool {
    self.state.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst).is_ok()
  }
  pub unsafe fn unlock(&self) { self.state.store(0, Ordering::SeqCst); }
}
