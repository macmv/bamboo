use super::Inventory;
use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

#[derive(Debug, Default, Clone)]
pub struct SharedInventory<const N: usize> {
  inv: Arc<Mutex<Inventory<N>>>,
}

impl<const N: usize> SharedInventory<N> {
  pub fn new() -> Self { SharedInventory { inv: Arc::new(Mutex::new(Inventory::new())) } }
  pub fn lock(&self) -> MutexGuard<Inventory<N>> { self.inv.lock() }
}
