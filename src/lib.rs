pub mod component_sizes_csv;
pub mod components;
pub mod csv_items;
pub mod csv_items_iter;
pub mod dataset;
pub mod degree_dist_csv;
mod edge_vec;
pub mod github_api;
mod github_types;
pub mod output_data;
pub mod traversal;

pub use edge_vec::EdgeVec;
pub use github_types::{
  GithubIDWrapper, HasGithubID, ItemType, Repo, User, UserRepoPair,
};

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
