use anyhow::Result;
use github_net::{
  component_sizes_csv::save_component_sizes, dataset::Dataset,
  degree_dist_csv::save_degree_item, ItemType,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "process_data",
  about = "load and process data, runs different computations depending on arguments"
)]
struct Opt {
  /// Maximum number of samples (mostly useful for testing).
  #[structopt(short, long)]
  limit: Option<usize>,

  /// Compute degrees and save as csv files.
  #[structopt(short, long)]
  degrees: bool,

  /// Compute components and save as csv files.
  #[structopt(short, long)]
  components: bool,
}

fn run_degrees(dataset: &Dataset) -> Result<()> {
  for &(item_type, name) in &[
    (ItemType::User, "user_degrees.csv"),
    (ItemType::User, "user_degrees.csv"),
  ] {
    save_degree_item(item_type, dataset, name, |items: &[_]| items.len())?
  }

  for &(item_type, name) in &[
    (ItemType::User, "user_total_contributions.csv"),
    (ItemType::User, "user_total_events.csv"),
  ] {
    save_degree_item(item_type, dataset, name, |items: &[usize]| -> usize {
      items
        .iter()
        .map(|&i| dataset.contributions()[i].num as usize)
        .sum()
    })?
  }

  Ok(())
}

fn run_components(dataset: &Dataset) -> Result<()> {
  save_component_sizes(dataset, "component_sizes.csv")
}

pub fn main() -> Result<()> {
  let opt = Opt::from_args();

  let dataset = Dataset::load_limited(opt.limit)?;

  // dbg!(&dataset);

  if opt.degrees {
    run_degrees(&dataset)?
  }

  if opt.components {
    run_components(&dataset)?
  }

  Ok(())
}
