use std::{
  cell::UnsafeCell,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use super::WasmLock;

pub struct ConstLock<T> {
  pub(super) lock:  WasmLock,
  pub(super) value: UnsafeCell<T>,
}

#[must_use = "if unused the lock will immediately unlock"]
pub struct ConstGuard<'a, T> {
  lock:   &'a ConstLock<T>,
  marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for ConstLock<T> {}
unsafe impl<T: Send> Sync for ConstLock<T> {}

impl<T> ConstLock<T> {
  pub const fn new(value: T) -> Self {
    ConstLock { lock: WasmLock::new(), value: UnsafeCell::new(value) }
  }

  /// # Safety
  ///
  /// Lock must be held in order for a guard to be created.
  unsafe fn guard(&self) -> ConstGuard<T> { ConstGuard { lock: self, marker: PhantomData } }

  /// Locks the mutex.
  pub fn lock<'a>(&'a self) -> ConstGuard<'a, T> {
    self.lock.lock();
    // SAFETY: We know the lock is held, so this is safe.
    unsafe { self.guard() }
  }
  /// If the mutex is unlocked, this locks and returns a locked mutex. If
  /// locked, this returns `None`.
  pub fn try_lock(&self) -> Option<ConstGuard<T>> {
    if self.lock.try_lock() {
      // SAFETY: We know the lock is held, so this is safe.
      Some(unsafe { self.guard() })
    } else {
      None
    }
  }

  /// Unlocks the mutex.
  ///
  /// # Safety
  ///
  /// The mutex must be locked, and there must be no guards present.
  pub unsafe fn force_unlock(&self) { self.lock.unlock(); }
}

impl<'a, T: 'a> Deref for ConstGuard<'a, T> {
  type Target = T;
  fn deref(&self) -> &T { unsafe { &*self.lock.value.get() } }
}
impl<'a, T: 'a> DerefMut for ConstGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.lock.value.get() } }
}
impl<'a, T: 'a> Drop for ConstGuard<'a, T> {
  fn drop(&mut self) {
    // SAFETY: The mutex must be locked in order to create a guard.
    unsafe { self.lock.lock.unlock() };
  }
}
