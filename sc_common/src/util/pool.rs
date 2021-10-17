use crossbeam_channel::Sender;
use std::{
  sync::atomic::{AtomicU32, Ordering},
  thread,
};

type BoxFn<S> = Box<dyn FnOnce(&S) + Send>;

/// A pool of threads. Each thread will be created with some state. The
/// `new_state` function passed to the constructor will be called once for every
/// thread that is created. This state will then be passed to each thread
/// whenever they execute. This can be used for things such as cloning an arc on
/// initialization, instead of cloning it every time you call `execute`.
pub struct ThreadPool<S> {
  threads: Vec<Sender<BoxFn<S>>>,
  // I don't want to use AtomicUsize here, as I want consistency between host systems. Also 4
  // billion threads is never going to happen, so we don't need to worry about overflows.
  id:      AtomicU32,
}

impl<S: Send + 'static> ThreadPool<S> {
  /// Creates a thread pool with the same number of works as cores on the
  /// system. These are logical cores, so features like hyper threading will be
  /// accounted for.
  pub fn auto<F: Fn() -> S>(new_state: F) -> Self {
    // I'm just going to use the number of cores here. Nothing more, nothing less.
    // Doubling this seems like way to many, and adding a small amount doesn't seem
    // necessary. There are always going to be at least 2 thread pools on the server
    // anyway, so adding more threads won't help that much.
    ThreadPool::new(num_cpus::get() as u32, new_state)
  }
  /// Creates a thread pool with the given number of worker threads. A
  /// reasonable number should be chosen here. Anything too large will crash the
  /// program and/or host system.
  ///
  /// # Panics
  ///
  /// Panics if the number of workers is 0.
  pub fn new<F: Fn() -> S>(workers: u32, new_state: F) -> Self {
    if workers == 0 {
      panic!("cannot create a thread pool with no workers");
    }
    let mut threads: Vec<Sender<BoxFn<S>>> = Vec::with_capacity(workers as usize);
    for _ in 0..workers {
      let (tx, rx) = crossbeam_channel::bounded(32);
      threads.push(tx);

      let s = new_state();
      thread::spawn(move || loop {
        match rx.recv() {
          Ok(f) => f(&s),
          Err(_) => break,
        }
      });
    }
    ThreadPool { threads, id: 0.into() }
  }

  /// Executes the given task on the next worker thread.
  pub fn execute<F: FnOnce(&S) + Send + 'static>(&self, f: F) {
    let id = self.id.fetch_add(1, Ordering::Relaxed) % self.threads.len() as u32;
    self.threads[id as usize].send(Box::new(f)).expect("thread unexpectedly closed");
  }
}
