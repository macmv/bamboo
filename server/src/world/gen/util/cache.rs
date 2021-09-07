use std::{
  borrow::Borrow,
  collections::{HashMap, VecDeque},
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
/// ```
/// let cache = Cache::new(|key| key + 10);
///
/// assert_eq!(cache.get(5), 15);
/// assert_eq!(cache.get(5), 15); // This will not call builder
/// ```
pub struct Cache<K, V, F: ?Sized> {
  data:    HashMap<K, (V, usize)>,
  age:     VecDeque<K>,
  builder: Box<F>,
}

impl<K, V, F> Cache<K, V, F> {
  /// Creates an empty cache. This will allocate all the required elements in
  /// the cache, so that all future calls will never allocate anything.
  pub fn new(builder: F) -> Self {
    Cache {
      data:    HashMap::with_capacity(MAX_SIZE),
      age:     VecDeque::with_capacity(MAX_SIZE),
      builder: Box::new(builder),
    }
  }
}

impl<K, V, F> Cache<K, V, F>
where
  K: Eq + Hash + Debug + Copy,
  V: Debug,
  F: Fn(K) -> V,
{
  /// If the key is present within the map, then the value is returned.
  /// Otherwise, the internal builder is used to create a new value for this
  /// key. Either way, a reference into the map is returned.
  pub fn get<'a>(&mut self, key: K) -> &V {
    if let Some((_, index)) = self.data.get(&key) {
      // We just looked up the item at key, so it should be at the back of age.
      self.age.remove(*index);
      self.age.push_back(key);
      self.data.insert(key, ((self.builder)(key), self.age.len() - 1));
    } else {
      self.clean();
      self.age.push_back(key);
      self.data.insert(key, ((self.builder)(key), self.age.len() - 1));
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
    while self.data.len() >= MAX_SIZE {
      let key = self.age.pop_front().unwrap();
      self.data.remove(&key).unwrap();
    }
    self.validate();
    // self.age just got a bunch of items removed, so we need to fix all the indices
    // in self.data.
    for (idx, key) in self.age.iter().enumerate() {
      self.data.get_mut(key).unwrap().1 = idx;
    }
  }

  fn validate(&self) {
    for key in self.age.iter() {
      // dbg!(&self.age);
      self.data.get(key).expect(&format!("invalid key: {:?}", key));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cache_get() {
    let mut cache = Cache::new(|key| key + 10);
    assert_eq!(*cache.get(5), 15);
  }

  #[test]
  fn cache_clean() {
    let mut cache = Cache::new(|key| key + 10);
    assert_eq!(cache.age.capacity(), MAX_SIZE);
    assert!(cache.data.capacity() >= MAX_SIZE, "{}", cache.data.capacity());

    for i in 0..MAX_SIZE {
      assert_eq!(*cache.get(i + 30), i + 40);
    }
    assert_eq!(cache.data.len(), MAX_SIZE);
    assert_eq!(cache.age.len(), MAX_SIZE);

    assert_eq!(*cache.get(1000), 1010);
    // Cache will have removed the element (30, 40), and added the element (1000,
    // 10010).
    assert_eq!(cache.data.len(), MAX_SIZE);
    assert_eq!(cache.age.len(), MAX_SIZE);
    println!("data: {:?}, age: {:?}", cache.data, cache.age);

    for i in 0..(MAX_SIZE - 1) {
      let key = i + 31;
      let val = i + 41;
      assert_eq!(cache.age[i], key);
      assert_eq!(cache.data[&key], (val, i));
    }
    assert_eq!(cache.age[MAX_SIZE - 1], 1000);
    assert_eq!(cache.data[&1000], (1010, MAX_SIZE - 1));

    assert_eq!(cache.age.capacity(), MAX_SIZE);
    assert!(cache.data.capacity() >= MAX_SIZE, "{}", cache.data.capacity());
  }
}
