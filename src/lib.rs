#[cfg(test)]
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;

pub mod db;
pub mod github_api;

pub use db::models::{GithubIDWrapper, HasGithubID, Repo, User};

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
