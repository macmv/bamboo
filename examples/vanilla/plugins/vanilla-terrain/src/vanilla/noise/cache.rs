use super::WyHashBuilder;
use std::{
  borrow::Borrow,
  collections::{HashMap, VecDeque},
  fmt,
  fmt::Debug,
  hash::Hash,
};

// VecDeque allocates one more than needed. So we end up allocating 128 elements
// in `age`, and around 200 elements in `data` (because the hash map
// overallocates, to avoid overloading).
const MAX_SIZE: usize = 127;

/// This is an item cache. It acts like a hash map, with some extra features.
/// When an item is looked up, and it does not exist, it will be created with a
/// builder function. If the size of this map gets too large, then old entries
/// will be removed.
///
/// It is a logic error for the builder function to return a different value
/// given the same key. So long as the builder function is consistent, the `get`
/// function will always return the same value for the same key.
///
/// The key is copy because we need to insert that in a list to track element
/// age. It could be clone instead, but we end up copying a lot in `get`. This
/// map is meant to be used with small keys, so I think the Clone restriction is
/// worthwhile.
///
/// Example:
/// ```ignore
/// use bb_server::world::gen::util::Cache;
/// use std::sync::{Arc, Mutex};
///
/// // An atomic would be less clear here, so lets just use a Mutex<i32>.
/// let num_calls = Arc::new(Mutex::new(0));
/// let num_calls_clone = num_calls.clone();
/// // The closure passed here must be `Send`, so we need to use a mutex.
/// let builder = move |key| {
///   *num_calls_clone.lock().unwrap() += 1;
///   key + 10
/// };
/// let mut cache = Cache::new();
///
/// assert_eq!(*cache.get(5, builder), 15);
/// assert_eq!(*cache.get(5, builder), 15); // This will not call builder.
/// assert_eq!(*num_calls.lock().unwrap(), 1);
///
/// assert_eq!(*cache.get(10, builder), 20); // This calls the builder again.
/// assert_eq!(*num_calls.lock().unwrap(), 2);
/// assert_eq!(*cache.get(10, builder), 20); // This won't call the builder.
/// assert_eq!(*num_calls.lock().unwrap(), 2);
/// ```
pub struct Cache<K, V> {
  data: HashMap<K, (V, usize), WyHashBuilder>,
  age:  VecDeque<K>,
}

impl<K, V> Cache<K, V> {
  /// Creates an empty cache. This will allocate all the required elements in
  /// the cache, so that all future calls will never allocate anything.
  pub fn new() -> Self {
    Cache {
      data: HashMap::with_capacity_and_hasher(MAX_SIZE, WyHashBuilder),
      age:  VecDeque::with_capacity(MAX_SIZE),
    }
  }
}

impl<K, V> Cache<K, V>
where
  K: Eq + Hash + Debug + Copy,
  V: Debug,
{
  /// If the key is present within the map, then the value is returned.
  /// Otherwise, the internal builder is used to create a new value for this
  /// key. Either way, a reference into the map is returned.
  pub fn get<F: Fn(K) -> V>(&mut self, key: K, builder: F) -> &V {
    // This makes sure the order for most recently modified item stays
    // correct. However, its slow, and removing it gave a 15% improvement in
    // the chunk test.
    /*
    if let Some((_, index)) = self.data.get_mut(&key) {
      let idx = *index;
      // We just looked up the item at key, so it should be at the back of age.
      *index = self.age.len() - 1;
      self.age.remove(idx);
      self.age.push_back(key);
      // All indices between index and self.age.len() - 1 need to be decreased by one,
      // in order to match the values in age that were just moved by the remove()
      // call.
      for i in idx..(self.age.len() - 1) {
        self.data.get_mut(&self.age[i]).unwrap().1 -= 1;
      }
    } else {
      */
    if !self.data.contains_key(&key) {
      self.clean();
      self.age.push_back(key);
      self.data.insert(key, ((builder)(key), self.age.len() - 1));
    }
    &self.data.get(&key).unwrap().0
  }

  /// This will lookup a value within the cache without the possibilty to insert
  /// it. If you are using this often, it might be best to just use a
  /// [`HashMap`].
  pub fn get_no_insert<Q: ?Sized>(&self, key: &Q) -> Option<&V>
  where
    K: Borrow<Q>,
    Q: Eq + Hash,
  {
    self.data.get(key).map(|(v, _index)| v)
  }

  /// Cleans up the map. This will remove any entries if there are more than
  /// MAX_SIZE - 1 items. This should be called right before inserting an item.
  fn clean(&mut self) {
    if self.data.len() >= MAX_SIZE {
      while self.data.len() >= MAX_SIZE {
        let key = self.age.pop_front().unwrap();
        self.data.remove(&key).unwrap();
      }
      #[cfg(debug_assertions)]
      self.validate();
      // self.age just got a bunch of items removed, so we need to fix all the
      // indices in self.data.
      self.fix_indices();
    }
  }

  fn fix_indices(&mut self) {
    // TODO: Make this faster
    for (idx, key) in self.age.iter().enumerate() {
      self.data.get_mut(key).unwrap().1 = idx;
    }
  }

  #[cfg(debug_assertions)]
  fn validate(&self) {
    for key in self.age.iter() {
      // dbg!(&self.age);
      if !self.data.contains_key(key) {
        dbg!(&self.age);
        panic!("invalid key: {:?}", key);
      }
    }
  }
}

impl<K, V> fmt::Debug for Cache<K, V>
where
  K: fmt::Debug,
  V: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("Cache").field("size", &self.data.len()).finish()
  }
}

// TODO: The cache is not reordered on get, so these tests need to be rewritten.
/*
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cache_get() {
    let mut cache = Cache::new();
    assert_eq!(*cache.get(5, |key| key + 10), 15);
  }

  #[test]
  fn cache_clean() {
    let mut cache = Cache::new();
    let builder = |key| key + 10;
    assert_eq!(cache.age.capacity(), MAX_SIZE);
    assert!(cache.data.capacity() >= MAX_SIZE, "{}", cache.data.capacity());

    for i in 0..MAX_SIZE {
      assert_eq!(*cache.get(i + 30, builder), i + 40);
    }
    assert_eq!(cache.data.len(), MAX_SIZE);
    assert_eq!(cache.age.len(), MAX_SIZE);

    assert_eq!(*cache.get(1000, builder), 1010);
    // Cache will have removed the element (30, 40), and added the element (1000,
    // 10010).
    assert_eq!(cache.data.len(), MAX_SIZE);
    assert_eq!(cache.age.len(), MAX_SIZE);

    // Re-order the age list
    assert_eq!(*cache.get(31, builder), 41);
    assert_eq!(cache.data.len(), MAX_SIZE);
    assert_eq!(cache.age.len(), MAX_SIZE);

    println!("data: {:?}, age: {:?}", cache.data, cache.age);

    for i in 0..(MAX_SIZE - 2) {
      let key = i + 32;
      let val = i + 42;
      assert_eq!(cache.age[i], key);
      assert_eq!(cache.data[&key], (val, i));
    }
    assert_eq!(cache.age[MAX_SIZE - 2], 1000);
    assert_eq!(cache.data[&1000], (1010, MAX_SIZE - 2));
    assert_eq!(cache.age[MAX_SIZE - 1], 31);
    assert_eq!(cache.data[&31], (41, MAX_SIZE - 1));

    assert_eq!(cache.age.capacity(), MAX_SIZE);
    assert!(cache.data.capacity() >= MAX_SIZE, "{}", cache.data.capacity());
  }
}
*/
