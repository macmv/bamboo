use super::ConstLock;
use std::{
  fmt,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

pub struct LazyLock<T> {
  lock: ConstLock<Option<T>>,
  init: fn() -> T,
}
#[must_use = "if unused the lock will immediately unlock"]
pub struct LazyGuard<'a, T> {
  lock:   &'a ConstLock<Option<T>>,
  marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for LazyLock<T> {}
unsafe impl<T: Send> Sync for LazyLock<T> {}

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

impl<'a, T: fmt::Debug + 'a> fmt::Debug for LazyGuard<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (**self).fmt(f) }
}
impl<'a, T: fmt::Display + 'a> fmt::Display for LazyGuard<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (**self).fmt(f) }
}
