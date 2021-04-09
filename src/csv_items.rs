use super::github_api;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct UserCsvEntry {
  pub github_id: github_api::ID,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RepoCsvEntry {
  pub github_id: github_api::ID,
  pub name: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ContributionCsvEntry {
  pub user_github_id: github_api::ID,
  pub repo_github_id: github_api::ID,
  pub num: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyCsvEntry {
  pub from_repo_github_id: github_api::ID,
  pub to_repo_github_id: github_api::ID,
  pub package_manager: Option<String>,
}
