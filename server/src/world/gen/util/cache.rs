use std::{
  borrow::Borrow,
  collections::{HashMap, VecDeque},
  hash::Hash,
};

const MAX_SIZE: usize = 128;

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
pub struct Cache<K, V, F> {
  data:    HashMap<K, (V, usize)>,
  age:     VecDeque<K>,
  builder: F,
}

impl<K, V, F> Cache<K, V, F> {
  pub fn new(builder: F) -> Self {
    Cache {
      data: HashMap::with_capacity(MAX_SIZE),
      age: VecDeque::with_capacity(MAX_SIZE),
      builder,
    }
  }
}

impl<K, V, F> Cache<K, V, F>
where
  K: Eq + Hash + Copy,
  F: Fn(K) -> V,
{
  /// If the key is present within the map, then the value is returned.
  /// Otherwise, the internal builder is used to create a new value for this
  /// key. Either way, a reference into the map is returned.
  pub fn get<'a>(&mut self, key: K) -> &V {
    if let Some((_, index)) = self.data.get(&key) {
      self.age.remove(*index);
      self.age.push_back(key);
      self.data.insert(key, ((self.builder)(key), self.age.len()));
    } else {
      self.age.push_back(key);
      self.data.insert(key, ((self.builder)(key), self.age.len()));
      self.clean();
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
  /// MAX_SIZE items.
  fn clean(&mut self) {
    while self.data.len() > MAX_SIZE {
      let key = self.age.pop_front().expect("age is empty but map is longer than MAX_SIZE");
      self.data.remove(&key).expect("invalid map state");
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
}
