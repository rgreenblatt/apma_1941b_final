use super::github_api;
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};

pub struct ItemsPaths {
  pub user_csv_list: PathBuf,
  pub repo_csv_list: PathBuf,
  pub user_login_csv_list: PathBuf,
  pub repo_name_csv_list: PathBuf,
  pub contribution_csv_list: PathBuf,
}

pub fn get_csv_list_paths() -> ItemsPaths {
  dotenv::dotenv().ok();

  ItemsPaths {
    user_csv_list: env::var("USER_CSV_LIST").unwrap().into(),
    repo_csv_list: env::var("REPO_CSV_LIST").unwrap().into(),
    user_login_csv_list: env::var("USER_LOGIN_CSV_LIST").unwrap().into(),
    repo_name_csv_list: env::var("REPO_NAME_CSV_LIST").unwrap().into(),
    contribution_csv_list: env::var("CONTRIBUTION_CSV_LIST").unwrap().into(),
  }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct UserCsvEntry {
  pub github_id: github_api::ID,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserLoginCsvEntry {
  pub github_id: github_api::ID,
  pub login: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct RepoCsvEntry {
  pub repo_github_id: github_api::ID,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RepoNameCsvEntry {
  pub github_id: github_api::ID,
  pub name: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ContributionCsvEntry {
  pub user_github_id: github_api::ID,
  pub repo_github_id: github_api::ID,
  pub num: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyCsvEntry {
  pub from_repo_github_id: github_api::ID,
  pub to_repo_github_id: github_api::ID,
  pub package_manager: Option<String>,
}
