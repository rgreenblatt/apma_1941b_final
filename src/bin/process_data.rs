use anyhow::Result;
use github_net::{
  component_sizes_csv::save_component_sizes,
  dataset::Dataset,
  degree_dist_csv::{save_degrees_dataset, save_degrees_projected_graph},
  projected_graph::ProjectedGraph,
  // save_subgraph::save_subgraph,
  traversal::Node,
  ItemType,
  UserRepoPair,
};
use std::{
  fs,
  path::{Path, PathBuf},
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
  #[structopt(long, use_delimiter = true)]
  subgraph_repo: Vec<String>,

  /// Save the subgraph close to this node.
  #[structopt(long, use_delimiter = true)]
  subgraph_user: Vec<String>,

  #[structopt(long, default_value = "3")]
  subgraph_limit: usize,

  #[structopt(long, use_delimiter = true)]
  projected_min_common_users: Vec<usize>,

  #[structopt(long, use_delimiter = true)]
  projected_min_common_repos: Vec<usize>,

  #[structopt(long, default_value = "0", use_delimiter = true)]
  min_contribution: Vec<u32>,
}

fn run_degrees(output_dir: &Path, dataset: &Dataset) -> Result<()> {
  for &(item_type, name) in &[
    (ItemType::User, "user_degrees.csv"),
    (ItemType::Repo, "repo_degrees.csv"),
  ] {
    save_degrees_dataset(
      &output_dir.join(name),
      item_type,
      dataset,
      |items: &[_]| items.len(),
    )?
  }

  for &(item_type, name) in &[
    (ItemType::User, "user_total_contributions.csv"),
    (ItemType::Repo, "repo_total_events.csv"),
  ] {
    save_degrees_dataset(
      &output_dir.join(name),
      item_type,
      dataset,
      |items: &[usize]| -> usize {
        items
          .iter()
          .map(|&i| dataset.contributions()[i].num as usize)
          .sum()
      },
    )?
  }

  Ok(())
}

fn run_components(output_dir: &Path, dataset: &Dataset) -> Result<()> {
  save_component_sizes(dataset, &output_dir.join("component_sizes.csv"))
}

pub fn main() -> Result<()> {
  let mut opt = Opt::from_args();

  let mut dataset =
    Dataset::load_limited(opt.limit, Some(opt.max_user_degree))?;

  opt.min_contribution.sort();

  for min_contribution in opt.min_contribution {
    println!("running for min contributions {}", min_contribution);
    dataset.filter_contributions(min_contribution);

    let output_dir: PathBuf =
      format!("output_data/min_contribution_{}", min_contribution).into();

    fs::create_dir_all(&output_dir)?;

    if opt.degrees {
      println!("running degrees");
      run_degrees(&output_dir, &dataset)?
    }

    if opt.components {
      println!("running components");
      run_components(&output_dir, &dataset)?
    }

    // for &(names, item_type) in &[
    //   (&opt.subgraph_user, ItemType::User),
    //   (&opt.subgraph_repo, ItemType::Repo),
    // ] {
    //   for name in names {
    //     let (idx, _) = dataset.names()[item_type]
    //       .iter()
    //       .enumerate()
    //       .find(|(_, other_name)| other_name == &name)
    //       .unwrap();
    //     let start = Node { item_type, idx };

    //     println!("saving subgraph for {:?} {}", item_type, name);

    //     save_subgraph(
    //       start,
    //       opt.subgraph_limit,
    //       opt.subgraph_min_repo_degree,
    //       opt.subgraph_min_common_users,
    //       &dataset,
    //     )?;
    //   }
    // }

    if min_contribution < 5 {
      // this is absurdly dense, the projected graph is unworkable
      continue;
    }

    let projected_min_common = UserRepoPair {
      user: opt.projected_min_common_repos.clone(),
      repo: opt.projected_min_common_users.clone(),
    };

    for &(item_type, prefix) in
      &[(ItemType::User, "user"), (ItemType::Repo, "repo")]
    {
      for &min_common in &projected_min_common[item_type] {
        println!(
          "running projected graph for {} with min common {}",
          prefix, min_common
        );
        let output_dir: PathBuf = output_dir
          .join(&format!("projected_{}", prefix))
          .join(format!("min_common_{}", min_common));

        fs::create_dir_all(&output_dir)?;

        let projected_graph =
          ProjectedGraph::from_dataset(item_type, min_common, &dataset);

        save_degrees_projected_graph(
          &output_dir.join("degrees.csv"),
          item_type,
          &projected_graph,
          &dataset,
          |items: &[_]| items.len(),
        )?;

        save_degrees_projected_graph(
          &output_dir.join("total_events.csv"),
          item_type,
          &projected_graph,
          &dataset,
          |items: &[usize]| -> usize {
            items
              .iter()
              .map(|&i| projected_graph.edges()[i].num as usize)
              .sum()
          },
        )?;
      }
    }
  }

  Ok(())
}
