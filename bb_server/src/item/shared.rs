use super::WrappedInventory;
use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SharedInventory<const N: usize> {
  inv: Arc<Mutex<WrappedInventory<N>>>,
}

impl<const N: usize> SharedInventory<N> {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    // TODO: Offset should be passedin
    SharedInventory { inv: Arc::new(Mutex::new(WrappedInventory::new(1, 0))) }
  }
  pub fn lock(&self) -> MutexGuard<WrappedInventory<N>> { self.inv.lock() }
}
