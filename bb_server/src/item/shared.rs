use super::{Inventory, WrappedInventory};
use crate::item::Stack;
use bb_transfer::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};
use parking_lot::{Mutex, MutexGuard};
use std::{io, sync::Arc};

#[derive(Debug, Clone)]
pub struct SharedInventory<const N: usize> {
  inv: Arc<Mutex<WrappedInventory<N>>>,
}

impl<const N: usize> From<Inventory<N>> for SharedInventory<N> {
  fn from(inv: Inventory<N>) -> Self { SharedInventory { inv: Arc::new(Mutex::new(inv.into())) } }
}

impl<const N: usize> Default for SharedInventory<N> {
  fn default() -> Self { Self::new() }
}

impl<const N: usize> MessageRead<'_> for SharedInventory<N> {
  fn read(r: &mut MessageReader) -> Result<Self, ReadError> {
    let items = r.read_list::<Stack>()?;
    let mut inv = WrappedInventory::new(1, 0);
    assert_eq!(items.len(), N);
    for (i, it) in items.enumerate() {
      inv.set(i as u32, it?);
    }
    Ok(SharedInventory { inv: Arc::new(Mutex::new(inv)) })
  }
}
impl<const N: usize> MessageWrite for SharedInventory<N> {
  fn write<W: io::Write>(&self, w: &mut MessageWriter<W>) -> Result<(), WriteError> {
    let inv = self.inv.lock();
    w.write_list(inv.inv.items().iter())
  }
}

impl<const N: usize> SharedInventory<N> {
  pub fn new() -> Self {
    // TODO: Offset should be passedin
    SharedInventory { inv: Arc::new(Mutex::new(WrappedInventory::new(1, 0))) }
  }
  pub fn lock(&self) -> MutexGuard<WrappedInventory<N>> { self.inv.lock() }
}
