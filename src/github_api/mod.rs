use anyhow::{anyhow, Result};
use graphql_client::GraphQLQuery;
use std::{error::Error, fmt};

mod dependencies;
mod info;

pub use dependencies::get_repo_dependencies;
pub use info::{get_repo, get_repo_names, get_user, get_user_logins};

/// Not really sure about this type (might not be big enough).
/// Note that this only has to be big enough for users and repos, not for
/// events.
pub type ID = i32;

#[derive(PartialEq, Eq, Debug)]
pub struct UnexpectedNullError(String);

impl fmt::Display for UnexpectedNullError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} was null", self.0)
  }
}

impl Error for UnexpectedNullError {}

#[derive(PartialEq, Eq, Debug)]
pub struct RepoNotFoundError;

impl fmt::Display for RepoNotFoundError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "repo not found")
  }
}

impl Error for RepoNotFoundError {}

#[derive(PartialEq, Eq, Debug)]
pub struct UserNotFoundError;

impl fmt::Display for UserNotFoundError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "user not found")
  }
}

impl Error for UserNotFoundError {}

use info::NodeIDWrapper;

const API_COUNT_LIMIT: i64 = 100;
const GITHUB_GRAPHQL_ENDPOINT: &'static str = "https://api.github.com/graphql";

fn make_request<Query: GraphQLQuery>(
  variables: Query::Variables,
) -> Result<Query::ResponseData> {
  let client = reqwest::blocking::Client::builder()
    .user_agent("github_net/0.1.0")
    .build()?;

  let q = Query::build_query(variables);

  let res = client
    .post(GITHUB_GRAPHQL_ENDPOINT)
    .bearer_auth(&get_token())
    .json(&q)
    .send()?;

  res.error_for_status_ref()?;

  let response_body: graphql_client::Response<_> = res.json()?;

  response_body.data.ok_or(anyhow!("missing response data"))
}

fn get_token() -> String {
  dotenv::dotenv().ok();

  std::env::var("GITHUB_API_TOKEN").expect("GITHUB_API_TOKEN must be set")
}
