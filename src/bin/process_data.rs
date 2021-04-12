use anyhow::Result;
use github_net::{
  component_sizes_csv::save_component_sizes, dataset::Dataset,
  degree_dist_csv::save_degree_item, save_subgraph::save_subgraph,
  traversal::Node, ItemType,
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

  /// Limit the maximum user degree to avoid bots and spammers.
  #[structopt(long, default_value = "10000")]
  max_user_degree: usize,

  /// Compute degrees and save as csv files.
  #[structopt(short, long)]
  degrees: bool,

  /// Compute components and save as csv files.
  #[structopt(short, long)]
  components: bool,

  /// Save the subgraph close to this node.
  #[structopt(long)]
  subgraph_repo: Vec<String>,

  /// Save the subgraph close to this node.
  #[structopt(long)]
  subgraph_user: Vec<String>,

  #[structopt(long, default_value = "3")]
  subgraph_limit: usize,

  #[structopt(long, default_value = "10000")]
  subgraph_min_repo_degree: usize,

  #[structopt(long, default_value = "100")]
  subgraph_min_common_users: usize,
}

fn run_degrees(dataset: &Dataset) -> Result<()> {
  for &(item_type, name) in &[
    (ItemType::User, "user_degrees.csv"),
    (ItemType::Repo, "repo_degrees.csv"),
  ] {
    save_degree_item(item_type, dataset, name, |items: &[_]| items.len())?
  }

  for &(item_type, name) in &[
    (ItemType::User, "user_total_contributions.csv"),
    (ItemType::Repo, "repo_total_events.csv"),
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

  let dataset = Dataset::load_limited(opt.limit, Some(opt.max_user_degree))?;

  if opt.degrees {
    println!("running degrees");
    run_degrees(&dataset)?
  }

  if opt.components {
    println!("running components");
    run_components(&dataset)?
  }

  for &(names, item_type) in &[
    (&opt.subgraph_user, ItemType::User),
    (&opt.subgraph_repo, ItemType::Repo),
  ] {
    for name in names {
      let (idx, _) = dataset.names()[item_type]
        .iter()
        .enumerate()
        .find(|(_, other_name)| other_name == &name)
        .unwrap();
      let start = Node { item_type, idx };

      println!("saving subgraph for {:?} {}", item_type, name);

      save_subgraph(
        start,
        opt.subgraph_limit,
        opt.subgraph_min_repo_degree,
        opt.subgraph_min_common_users,
        &dataset,
      )?;
    }
  }

  Ok(())
}
