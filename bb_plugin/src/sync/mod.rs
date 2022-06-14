mod const_lock;
mod lazy_lock;
mod lock;

pub use const_lock::{ConstGuard, ConstLock};
pub use lazy_lock::{LazyGuard, LazyLock};

use lock::WasmLock;

#[cfg(test)]
mod tests {
  use super::*;

  static LAZY: LazyLock<Vec<u8>> = LazyLock::new(|| vec![3, 4, 5]);
  static CONST: ConstLock<[u8; 4]> = ConstLock::new([3, 4, 5, 6]);

  use std::{sync::Arc, thread};

  #[test]
  fn add_numbers() {
    for num_threads in 2..10 {
      let mutex = Arc::new(ConstLock::new(0));

      let mut handles = vec![];
      for _ in 0..num_threads {
        let m = mutex.clone();
        handles.push(thread::spawn(|| {
          for _ in 0..500 {
            let lock = m.lock();
            lock += 1;
          }
        }));
      }

      let expected = num_threads * 500;
      for handle in handles {
        handle.join();
      }
      let value = mutex.lock();
      if value != expected {
        panic!("expected {expected}, got {value}");
      }
    }
  }
}
