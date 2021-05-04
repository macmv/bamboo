use std::{cmp::Eq, collections::HashMap, fmt::Debug, hash::Hash, rc::Rc, slice};

#[derive(Clone)]
pub struct Registry<K: Eq + Hash + Debug + Clone + Copy, V> {
  // The list of registered items.
  items: Vec<(K, V)>,
  // The last position that an item got inserted at. Should be moved if remove() would shift that
  // element.
  index: usize,
  // A way to access the items by a unique id.
  ids: HashMap<K, usize>,
}

/// This is a registry with any number of children. It is used to build the tree
/// of registries used in the VersionedRegistry. In most situations, you
/// probably just want to use a VersionedRegistry.
pub struct CloningRegistry<K: Eq + Hash + Debug + Clone + Copy, V> {
  current: Rc<Registry<K, V>>,
  children: Vec<Rc<CloningRegistry<K, V>>>,
}

impl<K: Eq + Hash + Debug + Clone + Copy, V> CloningRegistry<K, V> {
  pub fn new(children: Vec<Rc<CloningRegistry<K, V>>>) -> Self {
    CloningRegistry { current: Rc::new(Registry::new()), children }
  }
}

/// This is a registry setup such that a single item/block can be added to an
/// older version, which will then propogate through all the newer versions.
/// This is primarily used to build the block table. It makes it very easy to
/// generate all block ids for all versions at the same time.
pub struct VersionedRegistry<Ver: Eq + Hash + Debug, K: Eq + Hash + Debug + Clone + Copy, V> {
  versions: HashMap<Ver, Rc<CloningRegistry<K, V>>>,
}

impl<Ver: Eq + Hash + Debug, K: Eq + Hash + Debug + Clone + Copy, V> VersionedRegistry<Ver, K, V> {
  pub fn new(initial: Ver) -> Self {
    let mut reg = VersionedRegistry { versions: HashMap::new() };
    reg.versions.insert(initial, Rc::new(CloningRegistry::new(vec![])));
    reg
  }

  pub fn insert(&mut self, ver: Ver, k: K, v: V) {
    if !self.versions.contains_key(&ver) {
      panic!("unknown version {:?}", ver);
    }
  }
}

impl<K: Eq + Hash + Debug + Clone + Copy, V> Registry<K, V> {
  pub fn new() -> Self {
    Registry { items: vec![], index: 0, ids: HashMap::new() }
  }
  /// Inserts an element to the registry. If insert_at() has not been called
  /// yet, this is the same as calling [`add()`]. Panics if the key already
  /// exists.
  pub fn insert(&mut self, k: K, v: V) {
    if self.ids.contains_key(&k) {
      panic!("registry already contains key {:?}", k);
    }
    self.ids.insert(k, self.index);
    self.items.insert(self.index, (k, v));
    self.index += 1;
  }
  /// This starts inserting items at the new index `i`. The new item created
  /// from k and v will have the index `i + 1`. All items with a larger index
  /// will be shifted downwards by one. Any calls to insert() after this will
  /// place new items right after this one.
  pub fn insert_at(&mut self, i: usize, k: K, v: V) {
    if i > self.items.len() {
      panic!("i cannot be greater than items.len()");
    }
    self.index = i;
    self.insert(k, v);
  }

  /// Appends the given item to the end of the registry. Panics if the key
  /// already exists.
  pub fn add(&mut self, k: K, v: V) {
    if self.ids.contains_key(&k) {
      panic!("registry already contains key {:?}", k);
    }
    self.index = self.items.len();
    self.ids.insert(k, self.index);
    self.items.push((k, v));
  }

  /// Gets an item within the registry. This could be used to retrieve
  /// items/blocks by name, or a packet from its id over the wire.
  pub fn get(&self, k: K) -> Option<(usize, &V)> {
    match self.ids.get(&k) {
      Some(index) => Some((*index, &self.items[self.index].1)),
      None => None,
    }
  }

  /// Gets an item within the registry, via it's index. This could be used to
  /// retrieve a block by its id, or a packet from its id internally.
  pub fn get_index(&self, i: usize) -> Option<&(K, V)> {
    self.items.get(i)
  }

  /// Iterates through all elements, in order of index.
  pub fn iter(&self) -> slice::Iter<'_, (K, V)> {
    self.items.iter()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn registry_add() {
    let mut reg = Registry::new();
    reg.add("stone", 5);
    reg.add("dirt", 10);
    for (i, (k, v)) in reg.iter().enumerate() {
      if i == 0 {
        assert_eq!(k, &"stone");
        assert_eq!(v, &5);
      } else if i == 1 {
        assert_eq!(k, &"dirt");
        assert_eq!(v, &10);
      } else {
        unreachable!("should not have added more than 2 items");
      }
    }
  }

  #[test]
  pub fn registry_insert() {
    let mut reg = Registry::new();
    reg.add("first", 5);
    reg.add("second", 10);
    reg.add("third", 20);
    reg.insert_at(1, "inserted at", 100);
    reg.insert_at(1, "inserted at again", 100);
    reg.insert("inserted", 100);
    for (i, (k, v)) in reg.iter().enumerate() {
      dbg!(i);
      if i == 0 {
        assert_eq!(k, &"first");
        assert_eq!(v, &5);
      } else if i == 1 {
        assert_eq!(k, &"inserted at again");
        assert_eq!(v, &100);
      } else if i == 2 {
        assert_eq!(k, &"inserted");
        assert_eq!(v, &100);
      } else if i == 3 {
        assert_eq!(k, &"inserted at");
        assert_eq!(v, &100);
      } else if i == 4 {
        assert_eq!(k, &"second");
        assert_eq!(v, &10);
      } else if i == 5 {
        assert_eq!(k, &"third");
        assert_eq!(v, &20);
      } else {
        unreachable!("should not have added more than 6 items");
      }
    }
  }
}
