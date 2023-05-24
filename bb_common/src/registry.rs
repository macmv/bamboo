use std::{
  cell::{Ref, RefCell},
  cmp::Eq,
  collections::HashMap,
  fmt::Debug,
  hash::Hash,
  rc::Rc,
  slice,
};

/// This is a registry. It is essentially an ordered hashmap. It is intended to
/// be generated during load time, and then either used to generate compressed
/// data, or just stored and referenced at runtime.
///
/// Blocks/items will be registered with a [`VersionedRegistry`], so that any
/// changes made to older versions of the game will be propagated to newer
/// versions.
///
/// This is going to be used to generate packet specs. Because packet specs for
/// a version can be built entirely on their own, it is easier to clone a
/// registry and then edit the cloned one.
#[derive(Clone, Debug)]
pub struct Registry<K: Eq + Hash + Debug + Clone + Copy, V> {
  // The list of registered items.
  items: Vec<(K, V)>,
  // The last position that an item got inserted at. Should be moved if remove() would shift that
  // element.
  index: usize,
  // A way to access the items by a unique id.
  ids:   HashMap<K, usize>,
}

impl<K: Eq + Hash + Debug + Clone + Copy, V> Default for Registry<K, V> {
  /// Same as calling [`new`](Self::new).
  fn default() -> Self { Registry::new() }
}

impl<K: Eq + Hash + Debug + Clone + Copy, V> Registry<K, V> {
  /// Creates an empty registry. Any [`insert`](Self::insert) calls will do the
  /// same thing as [`add`](Self::add).
  pub fn new() -> Self { Registry { items: vec![], index: 0, ids: HashMap::new() } }
  /// Inserts an element to the registry. If [`insert_at`](Self::insert_at) has
  /// not been called yet, this is the same as calling [`add`](Self::add).
  /// Panics if the key already exists.
  pub fn insert(&mut self, k: K, v: V) {
    if self.ids.contains_key(&k) {
      panic!("registry already contains key {k:?}");
    }
    // Shifts all ids after index up by one, so that they correctly index into
    // self.items.
    for (k, _) in &self.items[self.index..] {
      *self.ids.get_mut(k).unwrap() += 1;
    }
    self.ids.insert(k, self.index);
    self.items.insert(self.index, (k, v));
    self.index += 1;
  }
  /// This starts inserting items at the new index `i`. The new item created
  /// from k and v will have the index `i`. All items with a larger index will
  /// be shifted downwards by one. Any calls to [`insert`](Self::insert) after
  /// this will place new items right after this one.
  pub fn insert_at(&mut self, i: usize, k: K, v: V) {
    if i > self.items.len() {
      panic!("i cannot be greater than items.len()");
    }
    self.index = i;
    self.insert(k, v);
  }

  /// Removes the entry with the given key. If the writing index is past that
  /// entry, then it will be moved backwards, so that inserts in the future work
  /// as expected.
  ///
  /// # Example
  ///
  /// ```rust
  /// # use bb_common::registry::Registry;
  /// # let some_value = 100;
  /// let mut reg = Registry::new();
  /// reg.add("a", some_value);
  /// reg.add("b", some_value);
  /// reg.add("c", some_value);
  /// reg.insert_at(1, "inserted", some_value);
  /// // We now have a registry which looks like:
  /// // - a
  /// // - inserted
  /// // - b
  /// // - c
  /// reg.remove("a");
  /// reg.insert("inserted again", some_value);
  /// // We now have a registry which looks like:
  /// // - inserted
  /// // - inserted again
  /// // - b
  /// // - c
  /// ```
  ///
  /// Without the decrement of the insert index, `"insert again"` would have
  /// been placed after `"b"`.
  pub fn remove(&mut self, k: K) { self.remove_index(self.ids[&k]); }
  /// Removes an entry via its index. See [`remove`](Self::remove) for more.
  pub fn remove_index(&mut self, i: usize) {
    // Shifts all ids after index down by one, so that they correctly index into
    // self.items.
    for (k, _) in &self.items[i + 1..] {
      *self.ids.get_mut(k).unwrap() -= 1;
    }
    let (k, _) = self.items[i];
    self.ids.remove(&k);
    self.items.remove(i);
    if self.index >= i {
      self.index -= 1;
    }
  }

  /// Appends the given item to the end of the registry. This will also set the
  /// currently writing index to be at the end of the list, so any future
  /// calls to [`insert`](Self::insert) will append at the end of the
  /// registry.
  ///
  /// # Example
  ///
  /// ```rust
  /// # use bb_common::registry::Registry;
  /// # let some_value = 100;
  /// let mut reg = Registry::new();
  /// reg.add("a", some_value);
  /// reg.add("b", some_value);
  /// reg.add("c", some_value);
  /// // The registry now has 3 items, "a", "b", "c", in that order.
  ///
  /// reg.insert_at(1, "inserted", some_value);
  /// reg.add("added at end", some_value);
  /// // The registry now contains "a", "inserted", "b", c", "added at end".
  ///
  /// // This insert call will add at the end of the registry, not at index 2.
  /// reg.insert("another insert", some_value);
  ///
  /// // The registry now contains "a", "inserted", "b", c", "added at end", and "another insert".
  /// ```
  /// See also: [`insert`](Self::insert) and [`insert_at`](Self::insert_at).
  ///
  /// # Panics
  ///
  /// This function will panic if the key is already present in the registry. If
  /// [`get(k)`](Self::get) returns `None`, then this function will not panic.
  pub fn add(&mut self, k: K, v: V) {
    if self.ids.contains_key(&k) {
      panic!("registry already contains key {k:?}");
    }
    self.index = self.items.len();
    self.ids.insert(k, self.index);
    self.items.push((k, v));
  }

  /// Gets an item within the registry. This could be used to retrieve
  /// items/blocks by name, or a packet from its id over the wire.
  pub fn get(&self, k: K) -> Option<(usize, &V)> {
    self.ids.get(&k).map(|index| (*index, &self.items[self.index].1))
  }

  /// Gets an item within the registry, via it's index. This could be used to
  /// retrieve a block/item by its id, or a packet from its id internally.
  pub fn get_index(&self, i: usize) -> Option<&(K, V)> { self.items.get(i) }

  /// Iterates through all elements, in order of index.
  pub fn iter(&self) -> slice::Iter<'_, (K, V)> { self.items.iter() }

  #[cfg(test)]
  fn validate_index(&self, i: usize) {
    let (k, _) = self.items.get(i).unwrap();
    assert_eq!(self.ids[k], i);
  }
}

/// This is a registry with any number of children. It is used to build the tree
/// of registries used in the [`VersionedRegistry`]. In most situations, you
/// probably just want to use a [`VersionedRegistry`]. This registry also calls
/// clone on the values for every child registry that it is inserted into, so V
/// should probably be an [`Rc`] or [`Arc`](std::sync::Arc).
#[derive(Debug)]
pub struct CloningRegistry<K: Eq + Hash + Debug + Clone + Copy, V: Clone> {
  current:  Registry<K, V>,
  children: Vec<Rc<RefCell<CloningRegistry<K, V>>>>,
}

impl<K: Eq + Hash + Debug + Clone + Copy, V: Clone> Default for CloningRegistry<K, V> {
  /// Same as calling [`new`](Self::new).
  fn default() -> Self { CloningRegistry::new() }
}

impl<K: Eq + Hash + Debug + Clone + Copy, V: Clone> CloningRegistry<K, V> {
  /// Creates an empty cloning registry. Any children that are added to this
  /// registry will also be called every time a modifying function is called on
  /// this registry.
  pub fn new() -> Self { CloningRegistry { current: Registry::new(), children: vec![] } }

  /// Adds a new child to this registry. All future calls that modify this
  /// registry will also modify this child.
  pub fn add_child(&mut self, child: Rc<RefCell<CloningRegistry<K, V>>>) {
    self.children.push(child);
  }

  pub fn insert(&mut self, k: K, v: V) {
    self.current.insert(k, v.clone());
    for c in &mut self.children {
      c.borrow_mut().insert(k, v.clone());
    }
  }
  pub fn insert_at(&mut self, i: usize, k: K, v: V) {
    self.current.insert_at(i, k, v.clone());
    for c in &mut self.children {
      c.borrow_mut().insert_at(i, k, v.clone());
    }
  }

  pub fn add(&mut self, k: K, v: V) {
    self.current.add(k, v.clone());
    for c in &mut self.children {
      c.borrow_mut().add(k, v.clone());
    }
  }

  /// Gets an item within the registry. This could be used to retrieve
  /// items/blocks by name, or a packet from its id over the wire.
  pub fn get(&self, k: K) -> Option<(usize, &V)> { self.current.get(k) }

  /// Gets an item within the registry, via it's index. This could be used to
  /// retrieve a block by its id, or a packet from its id internally.
  pub fn get_index(&self, i: usize) -> Option<&(K, V)> { self.current.get_index(i) }

  /// Iterates through all elements, in order of index.
  pub fn iter(&self) -> slice::Iter<'_, (K, V)> { self.current.iter() }

  #[cfg(test)]
  fn validate_index(&self, i: usize) { self.current.validate_index(i); }
}

type CloningRegRef<K, V> = Rc<RefCell<CloningRegistry<K, Rc<V>>>>;

/// This is a registry setup such that a single item/block can be added to an
/// older version, which will then propagate through all the newer versions.
/// This is primarily used to build the block table. It makes it very easy to
/// generate all block ids for all versions at the same time.
#[derive(Debug)]
pub struct VersionedRegistry<
  Ver: Eq + Hash + Debug + Clone + Copy,
  K: Eq + Hash + Debug + Clone + Copy,
  V: Clone,
> {
  current:  Ver,
  versions: HashMap<Ver, CloningRegRef<K, V>>,
}

impl<Ver: Eq + Hash + Debug + Clone + Copy, K: Eq + Hash + Debug + Clone + Copy, V: Clone>
  VersionedRegistry<Ver, K, V>
{
  pub fn new(initial: Ver) -> Self {
    let mut reg = VersionedRegistry { current: initial, versions: HashMap::new() };
    reg.versions.insert(initial, Rc::new(RefCell::new(CloningRegistry::new())));
    reg
  }

  pub fn add_version(&mut self, ver: Ver) {
    match self.versions.get_mut(&ver) {
      Some(_) => panic!("already contains version {ver:?}"),
      None => {
        let new = Rc::new(RefCell::new(CloningRegistry::new()));
        self.versions[&self.current].borrow_mut().add_child(new.clone());
        self.versions.insert(ver, new);
        self.current = ver;
      }
    }
  }

  pub fn insert(&mut self, ver: Ver, k: K, v: V) {
    match self.versions.get_mut(&ver) {
      Some(reg) => reg.borrow_mut().insert(k, Rc::new(v)),
      None => panic!("unknown version {ver:?}"),
    }
  }
  pub fn insert_at(&mut self, ver: Ver, i: usize, k: K, v: V) {
    match self.versions.get_mut(&ver) {
      Some(reg) => reg.borrow_mut().insert_at(i, k, Rc::new(v)),
      None => panic!("unknown version {ver:?}"),
    }
  }

  pub fn add(&mut self, ver: Ver, k: K, v: V) {
    match self.versions.get_mut(&ver) {
      Some(reg) => reg.borrow_mut().add(k, Rc::new(v)),
      None => panic!("unknown version {ver:?}"),
    }
  }

  pub fn get(&self, ver: Ver) -> Option<Ref<CloningRegistry<K, Rc<V>>>> {
    self.versions.get(&ver).map(|reg| reg.borrow())
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
    for (i, v) in reg.iter().enumerate() {
      let e;
      match i {
        0 => e = ("stone", 5),
        1 => e = ("dirt", 10),
        _ => unreachable!("should not have added more than 2 items"),
      }
      assert_eq!(v, &e);
      assert_eq!(reg.get_index(i).unwrap(), &e);
      reg.validate_index(i);
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
    for (i, v) in reg.iter().enumerate() {
      dbg!(i);
      // Expected value
      let e;
      match i {
        0 => e = ("first", 5),
        1 => e = ("inserted at again", 100),
        2 => e = ("inserted", 100),
        3 => e = ("inserted at", 100),
        4 => e = ("second", 10),
        5 => e = ("third", 20),
        _ => unreachable!("should not have added more than 6 items"),
      }
      assert_eq!(v, &e);
      assert_eq!(reg.get_index(i).unwrap(), &e);
      reg.validate_index(i);
    }
  }

  #[test]
  pub fn registry_remove() {
    let mut reg = Registry::new();
    reg.add("first", 5);
    reg.add("second", 10);
    reg.add("third", 20);
    reg.add("fourth", 20);
    reg.insert_at(1, "funny", 420);
    for (i, v) in reg.iter().enumerate() {
      // Expected value
      let e;
      match i {
        0 => e = ("first", 5),
        1 => e = ("funny", 420),
        2 => e = ("second", 10),
        3 => e = ("third", 20),
        4 => e = ("fourth", 20),
        _ => unreachable!("too many items"),
      }
      assert_eq!(v, &e);
      assert_eq!(reg.get_index(i).unwrap(), &e);
      reg.validate_index(i);
    }
    // This should decrement all elements, including reg.index. We validate that
    // reg.index has been decreased by checking the output of reg.insert(). See
    // below.
    reg.remove("funny");
    for (i, v) in reg.iter().enumerate() {
      // Expected value
      let e;
      match i {
        0 => e = ("first", 5),
        1 => e = ("second", 10),
        2 => e = ("third", 20),
        3 => e = ("fourth", 20),
        _ => unreachable!("too many items"),
      }
      assert_eq!(v, &e);
      assert_eq!(reg.get_index(i).unwrap(), &e);
      reg.validate_index(i);
    }
    // If reg.index was not decremented, then this would insert at index 2.
    reg.insert("funny (but new)", 420);
    for (i, v) in reg.iter().enumerate() {
      // Expected value
      let e;
      match i {
        0 => e = ("first", 5),
        1 => e = ("funny (but new)", 420),
        2 => e = ("second", 10),
        3 => e = ("third", 20),
        4 => e = ("fourth", 20),
        _ => unreachable!("too many items"),
      }
      assert_eq!(v, &e);
      assert_eq!(reg.get_index(i).unwrap(), &e);
      reg.validate_index(i);
    }
  }

  #[test]
  pub fn versioned_registry_insert() {
    let mut reg = VersionedRegistry::new(2);
    // Version 3 extends from version 2.
    reg.add_version(3);
    // This should be added to both.
    reg.add(2, "first", 5);
    reg.add(2, "second", 10);
    // This should only be added to v3.
    reg.add(3, "third", 20);
    dbg!(&reg);
    let current = reg.get(2).unwrap();
    for (i, v) in current.iter().enumerate() {
      let e = match i {
        0 => ("first", Rc::new(5)),
        1 => ("second", Rc::new(10)),
        _ => unreachable!("should not have added more than 2 items to v2"),
      };
      assert_eq!(v, &e);
      assert_eq!(current.get_index(i).unwrap(), &e);
      current.validate_index(i);
    }
    let current = reg.get(3).unwrap();
    for (i, v) in current.iter().enumerate() {
      let e = match i {
        0 => ("first", Rc::new(5)),
        1 => ("second", Rc::new(10)),
        2 => ("third", Rc::new(20)),
        _ => unreachable!("should not have added more than 3 items to v3"),
      };
      assert_eq!(v, &e);
      assert_eq!(current.get_index(i).unwrap(), &e);
      current.validate_index(i);
    }
  }
}
