use std::{
  cell::UnsafeCell,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

mod lock;

use lock::WasmLock;

pub struct ConstLock<T> {
  lock:  WasmLock,
  value: UnsafeCell<T>,
}

pub struct LazyLock<T> {
  lock: ConstLock<Option<T>>,
  init: fn() -> T,
}

#[must_use = "if unused the lock will immediately unlock"]
pub struct ConstGuard<'a, T> {
  lock:   &'a ConstLock<T>,
  marker: PhantomData<&'a mut T>,
}
#[must_use = "if unused the lock will immediately unlock"]
pub struct LazyGuard<'a, T> {
  lock:   &'a ConstLock<Option<T>>,
  marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for ConstLock<T> {}
unsafe impl<T: Send> Sync for ConstLock<T> {}

unsafe impl<T: Send> Send for LazyLock<T> {}
unsafe impl<T: Send> Sync for LazyLock<T> {}

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

impl<T> LazyLock<T> {
  /// Creates a lazy lock. This will call `init` once, the first time the lock
  /// is held.
  pub const fn new(init: fn() -> T) -> Self { LazyLock { lock: ConstLock::new(None), init } }

  pub fn lock(&self) -> LazyGuard<T> {
    let mut lock = self.lock.lock();
    if lock.is_none() {
      *lock = Some((self.init)());
    }
    LazyGuard { lock: &self.lock, marker: PhantomData }
  }
  pub fn try_lock(&self) -> Option<LazyGuard<T>> {
    if let Some(lock) = self.lock.try_lock().as_mut() {
      if lock.is_none() {
        *lock.deref_mut() = Some((self.init)());
      }
      Some(LazyGuard { lock: &self.lock, marker: PhantomData })
    } else {
      None
    }
  }
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

impl<'a, T: 'a> Deref for LazyGuard<'a, T> {
  type Target = T;
  fn deref(&self) -> &T {
    // SAFETY: We know the mutex is locked, so the `Option<T>` must be `Some`.
    unsafe { (&*self.lock.value.get()).as_ref().unwrap_unchecked() }
  }
}
impl<'a, T: 'a> DerefMut for LazyGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut T {
    // SAFETY: We know the mutex is locked, so the `Option<T>` must be `Some`.
    unsafe { (&mut *self.lock.value.get()).as_mut().unwrap_unchecked() }
  }
}
impl<'a, T: 'a> Drop for LazyGuard<'a, T> {
  fn drop(&mut self) {
    // SAFETY: The mutex must be locked in order to create a guard.
    unsafe { self.lock.lock.unlock() };
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  static LAZY: LazyLock<Vec<u8>> = LazyLock::new(|| vec![3, 4, 5]);
  static CONST: ConstLock<[u8; 4]> = ConstLock::new([3, 4, 5, 6]);
}
