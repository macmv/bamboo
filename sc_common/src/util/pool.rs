use crossbeam_channel::Sender;
use std::{
  sync::atomic::{AtomicU32, Ordering},
  thread,
};

type BoxFn = Box<dyn Fn() + Send>;

pub struct ThreadPool {
  threads: Vec<Sender<BoxFn>>,
  // I don't want to use AtomicUsize here, as I want consistency between host systems. Also 4
  // billion threads is never going to happen, so we don't need to worry about overflows.
  id:      AtomicU32,
}

impl ThreadPool {
  /// Creates a thread pool with the same number of works as cores on the
  /// system. These are logical cores, so features like hyper threading will be
  /// accounted for.
  pub fn auto() -> Self {
    // I'm just going to use the number of cores here. Nothing more, nothing less.
    // Doubling this seems like way to many, and adding a small amount doesn't seem
    // necessary. There are always going to be at least 2 thread pools on the server
    // anyway, so adding more threads won't help that much.
    ThreadPool::new(num_cpus::get() as u32)
  }
  /// Creates a thread pool with the given number of worker threads. A
  /// reasonable number should be chosen here. Anything too large will crash the
  /// program and/or host system.
  ///
  /// # Panics
  ///
  /// Panics if the number of workers is 0.
  pub fn new(workers: u32) -> Self {
    if workers == 0 {
      panic!("cannot create a thread pool with no workers");
    }
    let mut threads: Vec<Sender<BoxFn>> = Vec::with_capacity(workers as usize);
    for _ in 0..workers {
      let (tx, rx) = crossbeam_channel::bounded(32);
      threads.push(tx);

      thread::spawn(move || loop {
        match rx.recv() {
          Ok(f) => f(),
          Err(_) => break,
        }
      });
    }
    ThreadPool { threads, id: 0.into() }
  }

  /// Executes the given task on the next worker thread.
  pub fn execute<F: Fn() + Send + 'static>(&self, f: F) {
    let id = self.id.fetch_add(1, Ordering::Relaxed) % self.threads.len() as u32;
    self.threads[id as usize].send(Box::new(f)).expect("thread unexpectedly closed");
  }
}
