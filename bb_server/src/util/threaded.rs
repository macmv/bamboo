use parking_lot::RwLock;
use std::{cell::RefCell, collections::HashMap, fmt, thread, thread::ThreadId};

/// Creates a duplicate struct for each new thread this is used on. This allows
/// for a shared similar struct between threads.
pub struct Threaded<T> {
  // We don't use an UnsafeCell here, because the `Threaded` struct may be accessed when a caller
  // is inside the closure. With an unsafe cell, this would be a double mutable borrow. With
  // refcell, this will panic.
  threads: RwLock<HashMap<ThreadId, RefCell<T>>>,
  builder: Box<dyn Fn() -> T + Send>,
}

impl<T> Threaded<T> {
  /// Creates a new threaded type. Anytime [`get`](Self::get) is called on a new
  /// thread, the `builder` function will be called. This should always return
  /// the same object on every thread. It will still function if you don't
  /// return the same object, as calling `get` from the same thread will always
  /// return the stored object for that thread.
  pub fn new(builder: impl Fn() -> T + 'static + Send) -> Self {
    Threaded { threads: RwLock::new(HashMap::new()), builder: Box::new(builder) }
  }

  /// Calls `run`, and passes the item stored for the current thread.
  pub fn get<R>(&self, run: impl FnOnce(&mut T) -> R) -> R {
    let tid = thread::current().id();
    if !self.threads.read().contains_key(&tid) {
      self.threads.write().insert(tid, RefCell::new((self.builder)()));
    }

    let threads = self.threads.read();
    let item = &threads[&tid];
    let r = run(&mut item.borrow_mut());
    r
  }
}

// SAFETY: The only time we access the internal `RefCell` at all is within the
// `get` function. Because `ThreadId` is guaranteed to be unique for the current
// thread, we only access one `RefCell` per thread.
unsafe impl<T: Send> Send for Threaded<T> {}
unsafe impl<T: Send> Sync for Threaded<T> {}

impl<T> fmt::Debug for Threaded<T>
where
  T: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.get(|val| f.debug_tuple("Threaded").field(val).finish())
  }
}
