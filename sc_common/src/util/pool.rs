use crossbeam_channel::Sender;
use std::thread;

type BoxFn = Box<dyn Fn() + Send>;

pub struct ThreadPool {
  threads: Vec<Sender<BoxFn>>,
}

impl ThreadPool {
  pub fn new(workers: usize) -> Self {
    let mut threads: Vec<Sender<BoxFn>> = Vec::with_capacity(workers);
    for _ in 0..workers {
      let (tx, rx) = crossbeam_channel::bounded(32);
      threads.push(tx);

      thread::spawn(move || loop {
        let f = rx.recv().unwrap();
        f();
      });
    }
    ThreadPool { threads }
  }
}
