use anyhow::Result;
use github_net::{
  dataset::Dataset, degree_dist_csv::save_degree_item, ItemType,
};
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

  for &(item_type, name) in &[
    (ItemType::Repo, "repo_degrees.csv"),
    (ItemType::User, "user_degrees.csv"),
  ] {
    save_degree_item(item_type, &dataset, name, |items: &[_]| items.len())?
  }

  for &(item_type, name) in &[
    (ItemType::User, "user_total_contributions.csv"),
    (ItemType::Repo, "repo_total_events.csv"),
  ] {
    save_degree_item(item_type, &dataset, name, |items: &[usize]| -> usize {
      items
        .iter()
        .map(|&i| dataset.contributions()[i].num as usize)
        .sum()
    })?
  }

  Ok(())
}
