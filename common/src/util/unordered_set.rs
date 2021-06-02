/// This is essentially a HashSet, where the items do not need to be Hash or Eq.
/// This is slow to check equality, and also slow to check if it contains an
/// item. This should only be used when a HashSet is not possible.
pub struct UnorderedSet<T> {
  inner: Vec<T>,
}

impl UnorderedSet {}
