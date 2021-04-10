use anyhow::Result;
use github_net::{degree_dist_csv::save_degree_item, loaded_dataset::Dataset};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "load_dataset",
  about = "load the dataset directly into memory from .csv.gz files"
)]
struct Opt {
  // no options right now
}

pub fn main() -> Result<()> {
  let _ = Opt::from_args();

  let dataset = Dataset::load()?;

  let get_count = |items: &[_]| items.len();

  save_degree_item(
    &dataset.repo_contributions,
    &dataset.repo_names,
    "repo_degrees.csv",
    get_count,
  )?;
  save_degree_item(
    &dataset.user_contributions,
    &dataset.user_logins,
    "user_degrees.csv",
    get_count,
  )?;

  let get_total = |items: &[usize]| -> usize {
    items
      .iter()
      .map(|&i| dataset.contributions[i].num as usize)
      .sum()
  };

  save_degree_item(
    &dataset.repo_contributions,
    &dataset.repo_names,
    "repo_total_events.csv",
    get_total,
  )?;
  save_degree_item(
    &dataset.user_contributions,
    &dataset.user_logins,
    "user_total_contributions.csv",
    get_total,
  )?;

  Ok(())
}
