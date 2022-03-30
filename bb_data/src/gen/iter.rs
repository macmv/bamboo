pub struct AppendStart<I, J>
where
  I: Iterator,
  J: Iterator<Item = I::Item>,
{
  start: J,
  inner: I,
}
pub struct AppendEnd<I, J>
where
  I: Iterator,
  J: Iterator<Item = I::Item>,
{
  end:   J,
  inner: I,
}

impl<I, J> Iterator for AppendStart<I, J>
where
  I: Iterator,
  J: Iterator<Item = I::Item>,
{
  type Item = I::Item;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(item) = self.start.next() {
      Some(item)
    } else {
      self.inner.next()
    }
  }
}
impl<I, J> Iterator for AppendEnd<I, J>
where
  I: Iterator,
  J: Iterator<Item = I::Item>,
{
  type Item = I::Item;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(item) = self.inner.next() {
      Some(item)
    } else {
      self.end.next()
    }
  }
}

pub trait AppendIters: Iterator {
  fn append_start<'a, J>(self, start: J) -> AppendStart<Self, J::IntoIter>
  where
    J: IntoIterator<Item = Self::Item>,
    Self: Sized,
  {
    AppendStart { start: start.into_iter(), inner: self }
  }
  fn append_end<'a, J>(self, end: J) -> AppendEnd<Self, J::IntoIter>
  where
    J: IntoIterator<Item = Self::Item>,
    Self: Sized,
  {
    AppendEnd { end: end.into_iter(), inner: self }
  }
}

impl<T: ?Sized> AppendIters for T where T: Iterator {}
