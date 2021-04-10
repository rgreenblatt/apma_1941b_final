use super::{make_request, RepoNotFoundError, UserNotFoundError};
use crate::{GithubIDWrapper, Repo, User};
use anyhow::{anyhow, Result};
use graphql_client::GraphQLQuery;
use std::str::from_utf8;

// see https://gist.github.com/natanlao/afb676b17aa724754ee77099e4291f3f
// for info about node ids

pub(super) trait NodeIDWrapper: GithubIDWrapper + Sized {
  const BASE_STRING: &'static str;

  fn as_node_id(&self) -> String {
    base64::encode(
      format!("{}{}", Self::BASE_STRING, self.get_github_id()).as_bytes(),
    )
  }

  fn from_node_id(node_id: &str) -> Result<Self> {
    let bytes = base64::decode(&node_id)?;
    let decoded = from_utf8(&bytes)?;
    let mut sp = decoded.split(Self::BASE_STRING);
    if sp.next() != Some("") {
      return Err(anyhow!("unexpected prefix in node id"));
    }
    let id = sp
      .next()
      .ok_or_else(|| anyhow!("unexpected base64 encoded id!"))?
      .parse()?;

    if sp.next().is_some() {
      return Err(anyhow!("unexpected suffix in node id"));
    }

    Ok(Self::from_github_id(id))
  }
}

impl NodeIDWrapper for Repo {
  const BASE_STRING: &'static str = "010:Repository";
}

impl NodeIDWrapper for User {
  const BASE_STRING: &'static str = "04:User";
}

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "graphql/github_schema.graphql",
  query_path = "graphql/query_repo_names.graphql",
  response_derives = "Debug"
)]
struct RepoNames;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct OwnerName {
  pub owner: String,
  pub name: String,
}

pub fn get_repo_names(repos: &[Repo]) -> Result<Vec<OwnerName>> {
  let ids = repos.iter().map(Repo::as_node_id).collect();

  let response_data = make_request::<RepoNames>(repo_names::Variables { ids })?;

  response_data
    .nodes
    .iter()
    .map(|node| {
      let node = node.as_ref().ok_or(RepoNotFoundError {})?;
      use repo_names::RepoNamesNodesOn::*;
      if let Repository(repo) = &node.on {
        let mut items = repo.name_with_owner.split('/');
        let get_err = || anyhow!("unexpected nameWithOwner format");
        let owner = items.next().ok_or_else(get_err)?.to_owned();
        let name = items.next().ok_or_else(get_err)?.to_owned();
        Ok(OwnerName { owner, name })
      } else {
        Err(RepoNotFoundError {}.into())
      }
    })
    .collect()
}

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "graphql/github_schema.graphql",
  query_path = "graphql/query_repo_id.graphql",
  response_derives = "Debug"
)]
struct RepoID;

pub fn get_repo(owner: String, name: String) -> Result<Repo> {
  let response_data =
    make_request::<RepoID>(repo_id::Variables { owner, name })?;

  let repo = response_data.repository.ok_or(RepoNotFoundError {})?;

  Repo::from_node_id(&repo.id)
}

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "graphql/github_schema.graphql",
  query_path = "graphql/query_user_logins.graphql",
  response_derives = "Debug"
)]
struct UserLogins;

pub fn get_user_logins(users: &[User]) -> Result<Vec<String>> {
  let ids = users.iter().map(User::as_node_id).collect();

  let response_data =
    make_request::<UserLogins>(user_logins::Variables { ids })?;

  response_data
    .nodes
    .iter()
    .map(|node| {
      let node = node.as_ref().ok_or(UserNotFoundError {})?;
      use user_logins::UserLoginsNodesOn::*;
      if let User(user) = &node.on {
        Ok(user.login.clone())
      } else {
        Err(UserNotFoundError {}.into())
      }
    })
    .collect()
}

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "graphql/github_schema.graphql",
  query_path = "graphql/query_user_id.graphql",
  response_derives = "Debug"
)]
struct UserID;

pub fn get_user(login: String) -> Result<User> {
  let response_data = make_request::<UserID>(user_id::Variables { login })?;

  let user = response_data.user.ok_or(UserNotFoundError {})?;

  User::from_node_id(&user.id)
}

#[test]
fn get_user_and_login() -> Result<()> {
  for login in &[
    "rgreenblatt".to_owned(),
    "BurntSushi".to_owned(),
    "torvalds".to_owned(),
  ] {
    assert_eq!(
      get_user_logins(&[get_user(login.clone())?])?[0],
      login.clone()
    );
  }

  Ok(())
}

#[test]
fn login_not_found() -> Result<()> {
  let err = get_user(
    "a user which certainly doesn't exist (this user would be absurd)"
      .to_owned(),
  )
  .unwrap_err();

  crate::check_error(err, &UserNotFoundError {})
}

#[test]
fn user_not_found() -> Result<()> {
  let err = get_user_logins(&[User { github_id: 0 }]).unwrap_err();

  crate::check_error(err, &UserNotFoundError {})
}

#[test]
fn get_repo_and_owner_name() -> Result<()> {
  for (owner, name) in &[
    ("rgreenblatt".to_owned(), "dotfiles".to_owned()),
    ("numpy".to_owned(), "numpy".to_owned()),
    ("torvalds".to_owned(), "linux".to_owned()),
  ] {
    assert_eq!(
      get_repo_names(&[get_repo(owner.clone(), name.clone())?])?[0],
      OwnerName {
        owner: owner.clone(),
        name: name.clone()
      }
    );
  }

  Ok(())
}

#[test]
fn repo_name_not_found() -> Result<()> {
  let err = get_repo(
    "rgreenblatt".to_owned(),
    "a_repo_which_certainly_does_not_exist".to_owned(),
  )
  .unwrap_err();

  crate::check_error(err, &RepoNotFoundError {})
}

#[test]
fn repo_not_found() -> Result<()> {
  let err = get_repo_names(&[Repo { github_id: 0 }]).unwrap_err();

  crate::check_error(err, &RepoNotFoundError {})
}
