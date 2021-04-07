use super::{
  schema::{contributions, dependencies, repos, users},
  GithubID,
};
use std::convert::Into;

pub trait HasGithubID {
  fn get_github_id(&self) -> GithubID;
}

#[derive(
  Identifiable,
  Queryable,
  Hash,
  Ord,
  PartialOrd,
  Eq,
  PartialEq,
  Debug,
  Copy,
  Clone,
  Default,
)]
#[table_name = "users"]
pub struct UserEntry {
  pub(super) id: i32,
  pub github_id: GithubID,
}

impl HasGithubID for UserEntry {
  fn get_github_id(&self) -> GithubID {
    self.github_id
  }
}

#[derive(
  Insertable, Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone,
)]
#[table_name = "users"]
pub struct User {
  pub github_id: GithubID,
}

impl From<UserEntry> for User {
  fn from(user: UserEntry) -> Self {
    Self {
      github_id: user.github_id,
    }
  }
}

#[derive(
  Identifiable,
  Queryable,
  Hash,
  Ord,
  PartialOrd,
  Eq,
  PartialEq,
  Debug,
  Copy,
  Clone,
  Default,
)]
#[table_name = "repos"]
pub struct RepoEntry {
  pub(super) id: i32,
  pub github_id: GithubID,
}

impl HasGithubID for RepoEntry {
  fn get_github_id(&self) -> GithubID {
    self.github_id
  }
}

#[derive(
  Insertable, Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone,
)]
#[table_name = "repos"]
pub struct Repo {
  pub github_id: GithubID,
}

impl From<RepoEntry> for Repo {
  fn from(repo: RepoEntry) -> Self {
    Self {
      github_id: repo.github_id,
    }
  }
}

#[derive(
  Identifiable,
  Queryable,
  Associations,
  Hash,
  Ord,
  PartialOrd,
  Eq,
  PartialEq,
  Debug,
  Copy,
  Clone,
)]
#[belongs_to(RepoEntry, foreign_key = "repo_id")]
#[belongs_to(UserEntry, foreign_key = "user_id")]
#[table_name = "contributions"]
pub(super) struct ContributionEntry {
  pub(super) id: i32,
  pub(super) repo_id: i32,
  pub(super) user_id: i32,
  pub(super) num: i32,
}

#[derive(
  Insertable, Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone,
)]
#[table_name = "contributions"]
pub(super) struct NewContribution {
  pub(super) repo_id: i32,
  pub(super) user_id: i32,
  pub(super) num: i32,
}

#[derive(
  Identifiable,
  Queryable,
  Associations,
  Hash,
  Ord,
  PartialOrd,
  Eq,
  PartialEq,
  Debug,
  Copy,
  Clone,
)]
#[belongs_to(Repo, foreign_key = "repo_from_id", foreign_key = "repo_to_id")]
#[table_name = "dependencies"]
pub(super) struct DependencyEntry {
  pub(super) id: i32,
  pub(super) repo_from_id: i32,
  pub(super) repo_to_id: i32,
}

#[derive(
  Insertable, Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone,
)]
#[table_name = "dependencies"]
pub(super) struct NewDepencency {
  pub(super) repo_from_id: i32,
  pub(super) repo_to_id: i32,
}
