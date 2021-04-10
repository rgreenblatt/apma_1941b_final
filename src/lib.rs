pub mod csv_items;
pub mod degree_dist_csv;
pub mod github_api;
pub mod loaded_dataset;

#[cfg(test)]
fn check_error<E: std::error::Error + Eq + Sync + Send + 'static>(
  err: anyhow::Error,
  expected: &E,
) -> anyhow::Result<()> {
  assert_eq!(
    match err.downcast_ref::<E>() {
      Some(err) => err,
      None => return Err(err).into(),
    },
    expected,
  );

  Ok(())
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub struct Repo {
  pub github_id: github_api::ID,
}

pub trait HasGithubID {
  fn get_github_id(&self) -> github_api::ID;
}

pub trait GithubIDWrapper: HasGithubID {
  fn from_github_id(github_id: github_api::ID) -> Self;
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
