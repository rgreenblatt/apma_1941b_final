use anyhow::{anyhow, Result};
use std::{error::Error, fmt, str::from_utf8};

// mod dependencies;
mod repo_name;

// pub use dependencies::get_repo_dependencies;
pub use repo_name::{get_repo_id, get_repo_names};

/// Not really sure about this type (might not be big enough).
/// Note that this only has to be big enough for users and repos, not for
/// events.
pub type ID = i32;

const GITHUB_GRAPHQL_ENDPOINT: &'static str = "https://api.github.com/graphql";
const API_COUNT_LIMIT: i64 = 100;

pub fn get_token() -> String {
  dotenv::dotenv().ok();

  std::env::var("GITHUB_API_TOKEN").expect("GITHUB_API_TOKEN must be set")
}

/// see https://gist.github.com/natanlao/afb676b17aa724754ee77099e4291f3f
fn as_node_id(id: ID) -> String {
  base64::encode(format!("010:Repository{}", id).as_bytes())
}

/// see https://gist.github.com/natanlao/afb676b17aa724754ee77099e4291f3f
fn from_node_id(node_id: &str) -> Result<ID> {
  let bytes = base64::decode(&node_id)?;
  let decoded = from_utf8(&bytes)?;
  let out = decoded
    .split("Repository")
    .skip(1)
    .next()
    .ok_or(anyhow!("unexpected base64 encoded id!"))?
    .parse()?;
  Ok(out)
}

#[derive(PartialEq, Eq, Debug)]
pub enum ResponseError {
  RepoNotFound,
  UnexpectedNull(String),
}

impl fmt::Display for ResponseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::RepoNotFound => {
        write!(f, "repo not found")
      }
      Self::UnexpectedNull(s) => {
        write!(f, "{} was null", s)
      }
    }
  }
}

impl Error for ResponseError {}
