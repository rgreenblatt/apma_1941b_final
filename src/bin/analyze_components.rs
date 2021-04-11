use anyhow::Result;
use github_net::component_sizes_csv::{
  load_component_sizes, ComponentSizeCsvEntry,
};

pub fn main() -> Result<()> {
  let component_sizes: Vec<_> = load_component_sizes("component_sizes.csv")?
    .collect::<csv::Result<Vec<_>>>()?;

  let (total_users, total_repos) = component_sizes.iter().fold(
    (0, 0),
    |(user_total, repo_total),
     ComponentSizeCsvEntry {
       user_size,
       repo_size,
       count,
     }| {
      (
        user_total + user_size * count,
        repo_total + repo_size * count,
      )
    },
  );

  let (&giant_users, &giant_repos) = component_sizes
    .iter()
    .map(
      |ComponentSizeCsvEntry {
         user_size,
         repo_size,
         ..
       }| (user_size, repo_size),
    )
    .max()
    .unwrap();

  println!(
    "prop users in giant {}",
    giant_users as f64 / total_users as f64
  );
  println!(
    "prop repos in giant {}",
    giant_repos as f64 / total_repos as f64
  );

  Ok(())
}
