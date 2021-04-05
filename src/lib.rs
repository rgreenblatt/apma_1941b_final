#[cfg(test)]
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate graphql_client;

pub mod db;
pub mod github_api;

use anyhow::{bail, Error, Result};
use std::convert::TryFrom;

#[derive(Hash, Ord, PartialOrd, PartialEq, Eq, Debug, Clone)]
pub struct Repo(pub(crate) String);

pub fn check_owner_name(owner_name: &str) -> bool {
  return owner_name.matches('/').count() == 1;
}

impl Repo {
  pub fn try_new(owner_name: String) -> Result<Self> {
    if !check_owner_name(&owner_name) {
      bail!("invalid owner name!");
    }

    Ok(Self(owner_name))
  }

  pub fn owner(&self) -> &str {
    RepoOwnerName::new(&self.0).owner
  }

  pub fn name(&self) -> &str {
    RepoOwnerName::new(&self.0).name
  }

  pub fn owner_name(&self) -> &str {
    &self.0
  }
}

impl TryFrom<String> for Repo {
  type Error = Error;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    Repo::try_new(value)
  }
}

impl<'a> TryFrom<(&'a str, &'a str)> for Repo {
  type Error = Error;

  fn try_from(value: (&'a str, &'a str)) -> Result<Self, Self::Error> {
    Repo::try_new(format!("{}/{}", value.0, value.1))
  }
}

struct RepoOwnerName<'a> {
  owner: &'a str,
  name: &'a str,
}

impl<'a> RepoOwnerName<'a> {
  fn new(owner_name: &'a str) -> Self {
    let mut iter = owner_name.splitn(2, '/');
    let owner = iter.next().unwrap();
    let name = iter.next().unwrap();

    Self { owner, name }
  }
}
