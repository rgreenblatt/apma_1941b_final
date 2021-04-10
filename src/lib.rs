#[cfg(test)]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;

pub mod add_db_items;
pub mod csv_items;
pub mod db;
pub mod degree_dist_csv;
pub mod github_api;
pub mod loaded_dataset;

pub use add_db_items::add_items;
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
