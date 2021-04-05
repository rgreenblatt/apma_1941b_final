use super::schema::{contributions, dependencies, repos, users};

#[derive(Identifiable, Queryable, PartialEq, Debug, Clone)]
#[table_name = "users"]
pub struct User {
  pub(super) id: i32,
  pub(super) login: String,
}

impl User {
  #[cfg(test)]
  pub(super) fn to_new(&self) -> NewUser<'_> {
    NewUser { login: &self.login }
  }
}

#[derive(Insertable, Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[table_name = "users"]
pub struct NewUser<'a> {
  pub login: &'a str,
}

struct OwnerName<'a> {
  owner: &'a str,
  name: &'a str,
}

impl<'a> OwnerName<'a> {
  fn new(owner_name: &'a str) -> Self {
    let mut iter = owner_name.splitn(2, '/');
    let owner = iter.next().unwrap();
    let name = iter.next().unwrap();

    Self { owner, name }
  }
}

#[derive(Identifiable, Queryable, PartialEq, Debug, Clone)]
#[table_name = "repos"]
pub struct Repo {
  pub(super) id: i32,
  pub(super) owner_name: String,
}

impl Repo {
  #[cfg(test)]
  pub(super) fn to_new(&self) -> NewRepo<'_> {
    NewRepo {
      owner_name: &self.owner_name,
    }
  }

  pub fn owner(&self) -> &str {
    OwnerName::new(&self.owner_name).owner
  }

  pub fn name(&self) -> &str {
    OwnerName::new(&self.owner_name).name
  }
}

#[derive(Insertable, Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[table_name = "repos"]
pub struct NewRepo<'a> {
  pub(super) owner_name: &'a str,
}

impl<'a> NewRepo<'a> {
  pub fn owner(self) -> &'a str {
    OwnerName::new(self.owner_name).owner
  }

  pub fn name(self) -> &'a str {
    OwnerName::new(self.owner_name).name
  }
}

#[derive(
  Identifiable,
  Queryable,
  Associations,
  PartialEq,
  Debug,
  Clone,
  Copy,
  Ord,
  PartialOrd,
  Eq,
)]
#[belongs_to(Repo)]
#[belongs_to(User)]
#[table_name = "contributions"]
pub(super) struct Contribution {
  pub(super) id: i32,
  pub(super) repo_id: i32,
  pub(super) user_id: i32,
  pub(super) num: i32,
}

#[derive(Insertable, Debug, Clone, Copy)]
#[table_name = "contributions"]
pub(super) struct NewContribution {
  pub(super) repo_id: i32,
  pub(super) user_id: i32,
  pub(super) num: i32,
}

#[derive(
  Identifiable, Queryable, Associations, PartialEq, Debug, Clone, Copy,
)]
#[belongs_to(Repo, foreign_key = "repo_from_id", foreign_key = "repo_to_id")]
#[table_name = "dependencies"]
pub(super) struct Dependency {
  pub(super) id: i32,
  pub(super) repo_from_id: i32,
  pub(super) repo_to_id: i32,
}

#[derive(Insertable, Debug, Clone, Copy)]
#[table_name = "dependencies"]
pub(super) struct NewDepencency {
  pub(super) repo_from_id: i32,
  pub(super) repo_to_id: i32,
}
