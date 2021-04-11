use std::{iter::FromIterator, ops};

#[derive(Clone, Debug)]
pub struct EdgeVec<T> {
  ends: Vec<usize>,
  values: Vec<T>,
}

impl<T> EdgeVec<T> {
  pub fn new() -> Self {
    EdgeVec {
      ends: Vec::new(),
      values: Vec::new(),
    }
  }

  pub fn push(&mut self, items: impl IntoIterator<Item = T>) {
    self.values.extend(items);
    self.ends.push(self.values.len());
  }

  pub fn start(&self, i: usize) -> usize {
    if i == 0 {
      0
    } else {
      self.ends[i - 1]
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = &[T]> {
    (0..self.ends.len()).map(move |i| &self[i])
  }

  pub fn reserve(&mut self, additional: usize) {
    self.ends.reserve(additional);
    self.values.reserve(additional);
  }
}

impl<T> Default for EdgeVec<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T, V: IntoIterator<Item = T>> Extend<V> for EdgeVec<T> {
  fn extend<U: IntoIterator<Item = V>>(&mut self, iter: U) {
    let iter = iter.into_iter();
    self.reserve(iter.size_hint().0);

    for v in iter {
      self.push(v);
    }
  }
}

impl<T, V: IntoIterator<Item = T>> FromIterator<V> for EdgeVec<T> {
  fn from_iter<U: IntoIterator<Item = V>>(iter: U) -> Self {
    let mut out = Self::default();
    out.extend(iter);
    out
  }
}

impl<T> ops::Index<usize> for EdgeVec<T> {
  type Output = [T];

  fn index(&self, i: usize) -> &Self::Output {
    let start = self.start(i);
    &self.values[start..self.ends[i]]
  }
}

impl<T> ops::IndexMut<usize> for EdgeVec<T> {
  fn index_mut(&mut self, i: usize) -> &mut Self::Output {
    let start = self.start(i);
    &mut self.values[start..self.ends[i]]
  }
}
