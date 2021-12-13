use parking_lot::RwLock;
use std::{cell::RefCell, collections::HashMap, thread, thread::ThreadId};

/// Creates a duplicate struct for each new thread this is used on. This allows
/// for a shared similar struct between threads.
pub struct Threaded<T> {
  // We don't use an UnsafeCell here, because the `Threaded` struct may be accessed when a caller
  // is inside the closure. With an unsafe cell, this would be a double mutable borrow. With
  // refcell, this will panic.
  threads: RwLock<HashMap<ThreadId, RefCell<T>>>,
  builder: Box<dyn Fn() -> T>,
}

impl<T> Threaded<T> {
  /// Creates a new threaded type. Anytime [`get`](Self::get) is called on a new
  /// thread, the `builder` function will be called. This should always return
  /// the same object on every thread. It will still function if you don't
  /// return the same object, as calling `get` from the same thread will always
  /// return the stored object for that thread.
  pub fn new(builder: impl Fn() -> T + 'static) -> Self {
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
