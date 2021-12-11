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
  pub fn new(builder: impl Fn() -> T + 'static) -> Self {
    Threaded { threads: RwLock::new(HashMap::new()), builder: Box::new(builder) }
  }

  pub fn get(&self, run: impl Fn(&mut T)) {
    let tid = thread::current().id();
    if !self.threads.read().contains_key(&tid) {
      self.threads.write().insert(tid, RefCell::new((self.builder)()));
    }

    let threads = self.threads.read();
    let item = &threads[&tid];
    run(&mut item.borrow_mut());
  }
}
