use crossbeam_channel::{Sender, TrySendError};
use std::thread;

type BoxFn<S> = Box<dyn FnOnce(&S) + Send>;

/// A pool of threads. Each thread will be created with some state. The
/// `new_state` function passed to the constructor will be called once for every
/// thread that is created. This state will then be passed to each thread
/// whenever they execute. This can be used for things such as cloning an arc on
/// initialization, instead of cloning it every time you call `execute`.
pub struct ThreadPool<S> {
  tx: Sender<BoxFn<S>>,
}

/// Default to 256 elements in the queue for a thread pool.
pub const DEFAULT_LIMIT: usize = 256;

impl<S: Send + 'static> ThreadPool<S> {
  /// Creates a thread pool with the same number of works as cores on the
  /// system. These are logical cores, so features like hyper threading will be
  /// accounted for.
  pub fn auto<F: Fn() -> S>(name: &str, new_state: F) -> Self {
    // I'm just going to use the number of cores here. Nothing more, nothing less.
    // Doubling this seems like way to many, and adding a small amount doesn't seem
    // necessary. There are always going to be at least 2 thread pools on the server
    // anyway, so adding more threads won't help that much.
    ThreadPool::new(name, num_cpus::get() as u32, new_state)
  }
  /// Creates a thread pool with the same number of works as cores on the
  /// system. These are logical cores, so features like hyper threading will be
  /// accounted for.
  ///
  /// The `limit` is the size of the message queue. This is the amount of
  /// messages that can be sent before `execute` blocks.
  ///
  /// # Panics
  ///
  /// Panics if the number of workers is 0.
  pub fn auto_with_limit<F: Fn() -> S>(name: &str, limit: usize, new_state: F) -> Self {
    ThreadPool::new_with_limit(name, num_cpus::get() as u32, limit, new_state)
  }
  /// Creates a thread pool with the given number of worker threads. A
  /// reasonable number should be chosen here. Anything too large will crash the
  /// program and/or host system.
  ///
  /// # Panics
  ///
  /// Panics if the number of workers is 0.
  pub fn new<F: Fn() -> S>(name: &str, workers: u32, new_state: F) -> Self {
    ThreadPool::new_with_limit(name, workers, DEFAULT_LIMIT, new_state)
  }

  /// Creates a thread pool with the given number of worker threads. A
  /// reasonable number should be chosen here. Anything too large will crash the
  /// program and/or host system.
  ///
  /// The `limit` is the size of the message queue. This is the amount of
  /// messages that can be sent before `execute` blocks.
  ///
  /// # Panics
  ///
  /// Panics if the number of workers is 0.
  pub fn new_with_limit<F: Fn() -> S>(
    name: &str,
    workers: u32,
    limit: usize,
    new_state: F,
  ) -> Self {
    if workers == 0 {
      panic!("cannot create a thread pool with no workers");
    }
    let (tx, rx): (Sender<BoxFn<S>>, _) = crossbeam_channel::bounded(limit);
    for _ in 0..workers {
      let s = new_state();
      let rx = rx.clone();
      thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
          while let Ok(f) = rx.recv() {
            f(&s)
          }
        })
        .unwrap_or_else(|e| panic!("could not spawn worker thread for pool {name}: {e}"));
    }
    ThreadPool { tx }
  }

  /// Executes the given task on a random worker thread. Blocks if the internal
  /// queue is full. If you don't want to block, use
  /// [`try_execute`](Self::try_execute).
  pub fn execute<F: FnOnce(&S) + Send + 'static>(&self, f: F) {
    self.tx.send(Box::new(f)).expect("thread unexpectedly closed");
  }

  /// Executes the given task on a random worker thread. Returns `Err(())` if
  /// the internal channel is full.
  pub fn try_execute<F: FnOnce(&S) + Send + 'static>(&self, f: F) -> Result<(), ()> {
    match self.tx.try_send(Box::new(f)) {
      Ok(()) => Ok(()),
      Err(TrySendError::Full(_)) => Err(()),
      Err(TrySendError::Disconnected(_)) => panic!("thread unexpectedly closed"),
    }
  }

  /// Runs the given closure for every item in the iterator, until the iterator
  /// returns None.
  ///
  /// Since each backing thread is just consuming from a channel, this will
  /// simply push a closure for every single element. This means that if you
  /// provide a large iterator, there is a good chance the channel used will
  /// fill up, and cause this function to block.
  pub fn execute_for_each<
    I: Iterator<Item = T>,
    T: Send + 'static,
    F: FnOnce(T, &S) + Copy + Send + Sync + 'static,
  >(
    &self,
    iter: I,
    f: F,
  ) {
    for it in iter {
      self.tx.send(Box::new(move |s| f(it, s))).expect("thread unexpectedly closed");
    }
  }

  /// Waits for all tasks to be completed.
  pub fn wait(&self) {
    loop {
      if self.tx.is_empty() {
        break;
      }
      std::thread::yield_now();
    }
  }
}
