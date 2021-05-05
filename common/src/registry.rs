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
/// changes made to older versions of the game will be propogated to newer
/// versions.
///
/// This is going to be used to generate packet specs. Because packet specs for
/// a version can be built entireley on their own, it is easier to clone a
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

impl<K: Eq + Hash + Debug + Clone + Copy, V> Registry<K, V> {
  /// Creates an empty registry. Any [`insert`](Self::insert) calls will do the
  /// same thing as [`add`](Self::add).
  pub fn new() -> Self {
    Registry { items: vec![], index: 0, ids: HashMap::new() }
  }
  /// Inserts an element to the registry. If [`insert_at`](Self::insert_at) has
  /// not been called yet, this is the same as calling [`add`](Self::add).
  /// Panics if the key already exists.
  pub fn insert(&mut self, k: K, v: V) {
    if self.ids.contains_key(&k) {
      panic!("registry already contains key {:?}", k);
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

  /// Appends the given item to the end of the registry. Panics if the key
  /// already exists. This will also set the currently writing index to be at
  /// the end of the list, so any future calls to [`insert`](Self::insert) will
  /// append at the end of the registry.
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
  /// retrieve a block/item by its id, or a packet from its id internally.
  pub fn get_index(&self, i: usize) -> Option<&(K, V)> {
    self.items.get(i)
  }

  /// Iterates through all elements, in order of index.
  pub fn iter(&self) -> slice::Iter<'_, (K, V)> {
    self.items.iter()
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

impl<K: Eq + Hash + Debug + Clone + Copy, V: Clone> CloningRegistry<K, V> {
  pub fn new() -> Self {
    CloningRegistry { current: Registry::new(), children: vec![] }
  }

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
  pub fn get(&self, k: K) -> Option<(usize, &V)> {
    self.current.get(k)
  }

  /// Gets an item within the registry, via it's index. This could be used to
  /// retrieve a block by its id, or a packet from its id internally.
  pub fn get_index(&self, i: usize) -> Option<&(K, V)> {
    self.current.get_index(i)
  }

  /// Iterates through all elements, in order of index.
  pub fn iter(&self) -> slice::Iter<'_, (K, V)> {
    self.current.iter()
  }
}

/// This is a registry setup such that a single item/block can be added to an
/// older version, which will then propogate through all the newer versions.
/// This is primarily used to build the block table. It makes it very easy to
/// generate all block ids for all versions at the same time.
#[derive(Debug)]
pub struct VersionedRegistry<
  Ver: Eq + Hash + Debug + Clone + Copy,
  K: Eq + Hash + Debug + Clone + Copy,
  V: Clone,
> {
  current:  Ver,
  versions: HashMap<Ver, Rc<RefCell<CloningRegistry<K, Rc<V>>>>>,
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
      Some(_) => panic!("already contains version {:?}", ver),
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
      None => panic!("unknown version {:?}", ver),
    }
  }
  pub fn insert_at(&mut self, ver: Ver, i: usize, k: K, v: V) {
    match self.versions.get_mut(&ver) {
      Some(reg) => reg.borrow_mut().insert_at(i, k, Rc::new(v)),
      None => panic!("unknown version {:?}", ver),
    }
  }

  pub fn add(&mut self, ver: Ver, k: K, v: V) {
    match self.versions.get_mut(&ver) {
      Some(reg) => reg.borrow_mut().add(k, Rc::new(v)),
      None => panic!("unknown version {:?}", ver),
    }
  }

  /// Gets an item within the registry. This could be used to retrieve
  /// items/blocks by name, or a packet from its id over the wire.
  pub fn get(&self, ver: Ver) -> Option<Ref<CloningRegistry<K, Rc<V>>>> {
    match self.versions.get(&ver) {
      Some(reg) => Some(reg.borrow()),
      None => None,
    }
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
    for (i, (k, v)) in reg.get(2).unwrap().iter().enumerate() {
      if i == 0 {
        assert_eq!(k, &"first");
        assert_eq!(v.as_ref(), &5);
      } else if i == 1 {
        assert_eq!(k, &"second");
        assert_eq!(v.as_ref(), &10);
      } else {
        unreachable!("should not have added more than 2 items to v2");
      }
    }
    for (i, (k, v)) in reg.get(3).unwrap().iter().enumerate() {
      if i == 0 {
        assert_eq!(k, &"first");
        assert_eq!(v.as_ref(), &5);
      } else if i == 1 {
        assert_eq!(k, &"second");
        assert_eq!(v.as_ref(), &10);
      } else if i == 2 {
        assert_eq!(k, &"third");
        assert_eq!(v.as_ref(), &20);
      } else {
        unreachable!("should not have added more than 3 items to v3");
      }
    }
  }
}
