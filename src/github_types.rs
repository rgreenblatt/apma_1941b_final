use crate::github_api;
use std::{iter, ops};

pub trait HasGithubID {
  fn get_github_id(&self) -> github_api::ID;
}

pub trait GithubIDWrapper: HasGithubID {
  fn from_github_id(github_id: github_api::ID) -> Self;
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub struct User {
  pub github_id: github_api::ID,
}

impl HasGithubID for User {
  fn get_github_id(&self) -> github_api::ID {
    self.github_id
  }
}

impl GithubIDWrapper for User {
  fn from_github_id(github_id: github_api::ID) -> Self {
    Self { github_id }
  }
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub struct Repo {
  pub github_id: github_api::ID,
}

impl HasGithubID for Repo {
  fn get_github_id(&self) -> github_api::ID {
    self.github_id
  }
}

impl GithubIDWrapper for Repo {
  fn from_github_id(github_id: github_api::ID) -> Self {
    Self { github_id }
  }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum ItemType {
  User,
  Repo,
}

impl ItemType {
  pub fn other(self) -> Self {
    match self {
      Self::User => Self::Repo,
      Self::Repo => Self::User,
    }
  }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Default, Hash)]
pub struct UserRepoPair<T> {
  pub user: T,
  pub repo: T,
}

impl<T> UserRepoPair<T> {
  pub fn same(value: T) -> Self
  where
    T: Clone,
  {
    Self {
      user: value.clone(),
      repo: value,
    }
  }

  pub fn as_ref(&self) -> UserRepoPair<&T> {
    UserRepoPair {
      user: &self.user,
      repo: &self.repo,
    }
  }

  pub fn as_mut(&mut self) -> UserRepoPair<&mut T> {
    UserRepoPair {
      user: &mut self.user,
      repo: &mut self.repo,
    }
  }

  pub fn map<U>(self, f: impl Fn(T) -> U) -> UserRepoPair<U> {
    UserRepoPair {
      user: f(self.user),
      repo: f(self.repo),
    }
  }
}

impl<T> IntoIterator for UserRepoPair<T> {
  type Item = T;
  type IntoIter = iter::Chain<iter::Once<T>, iter::Once<T>>;

  fn into_iter(self) -> Self::IntoIter {
    iter::once(self.user).chain(iter::once(self.repo))
  }
}

impl<T> ops::Index<ItemType> for UserRepoPair<T> {
  type Output = T;

  fn index(&self, item_type: ItemType) -> &Self::Output {
    match item_type {
      ItemType::User => &self.user,
      ItemType::Repo => &self.repo,
    }
  }
}

impl<T> ops::IndexMut<ItemType> for UserRepoPair<T> {
  fn index_mut(&mut self, item_type: ItemType) -> &mut Self::Output {
    match item_type {
      ItemType::User => &mut self.user,
      ItemType::Repo => &mut self.repo,
    }
  }
}
