use anyhow::Result;
use github_net::{
  component_sizes_csv::save_component_sizes,
  contribution_dist_csv::{
    save_contribution_dist, save_contribution_dist_item,
  },
  dataset::Dataset,
  degree_dist_csv::{save_degrees_dataset, save_degrees_projected_graph},
  item_name_to_save_name,
  projected_graph::ProjectedGraph,
  save_subgraph::save_subgraph,
  ItemType, UserRepoPair,
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

  /// Compute the contribution distribution and save it to a csv.
  #[structopt(long)]
  contribution: bool,

  /// Compute the contribution distribution for a user and save it to a csv.
  #[structopt(long, use_delimiter = true)]
  contributions_for_user: Vec<String>,

  /// Compute the contribution distribution for a repo and save it to a csv.
  #[structopt(long, use_delimiter = true)]
  contributions_for_repo: Vec<String>,

  /// Compute degrees and save as csv files.
  #[structopt(short, long)]
  degrees: bool,

  /// Compute components and save as csv files.
  #[structopt(short, long)]
  components: bool,

  /// Save the projected subgraph close to this user.
  #[structopt(long, use_delimiter = true)]
  subgraph_user: Vec<String>,

  /// Save the subgraph close to this repo.
  #[structopt(long, use_delimiter = true)]
  subgraph_repo: Vec<String>,

  #[structopt(long, default_value = "3")]
  subgraph_limit: usize,

  /// How many common items (repos in this case) are needed to keep a edge in
  /// the user projected graph.
  #[structopt(long, use_delimiter = true)]
  projected_user_min_common: Vec<usize>,

  /// How many common items (users in this case) are needed to keep a edge in
  /// the repo projected graph.
  #[structopt(long, use_delimiter = true)]
  projected_repo_min_common: Vec<usize>,

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

pub fn main() -> Result<()> {
  let Opt {
    limit,
    max_user_degree,
    contribution,
    contributions_for_user,
    contributions_for_repo,
    degrees,
    components,
    subgraph_user,
    subgraph_repo,
    subgraph_limit,
    projected_user_min_common,
    projected_repo_min_common,
    mut min_contribution,
  } = Opt::from_args();

  let mut projected_min_common = UserRepoPair {
    user: projected_user_min_common,
    repo: projected_repo_min_common,
  };

  let subgraph_names = UserRepoPair {
    user: subgraph_user,
    repo: subgraph_repo,
  };

  let contribution_names = UserRepoPair {
    user: (contributions_for_user, "user"),
    repo: (contributions_for_repo, "repo"),
  };

  let mut dataset = Dataset::load_limited(limit, Some(max_user_degree))?;

  let output_dir: PathBuf = "output_data/".into();

  if contribution {
    println!("running contribution");
    save_contribution_dist(
      &output_dir.join("contributions_dist.csv"),
      &dataset,
    )?;
  }

  for (item_type, (names, item_name)) in contribution_names.iter_with_types() {
    for name in names {
      println!("running contribution dist for {} {}", item_name, name);
      let idx = dataset.find_item(item_type, &name).unwrap();
      save_contribution_dist_item(
        &output_dir.join(format!(
          "{}_{}_contributions_dist.csv",
          item_name,
          item_name_to_save_name(&name)
        )),
        item_type,
        idx,
        &dataset,
      )?;
    }
  }

  min_contribution.sort();

  for min_contribution in min_contribution {
    println!("running for min contributions {}", min_contribution);
    dataset.filter_contributions(min_contribution);

    let output_dir =
      output_dir.join(format!("min_contribution_{}", min_contribution));

    fs::create_dir_all(&output_dir)?;

    if degrees {
      println!("running degrees");
      run_degrees(&output_dir, &dataset)?
    }

    if components {
      println!("running components");
      save_component_sizes(&dataset, &output_dir.join("component_sizes.csv"))?;
    }

    for &(item_type, prefix) in
      &[(ItemType::User, "user"), (ItemType::Repo, "repo")]
    {
      let projected_min_common = &mut projected_min_common[item_type];
      projected_min_common.sort();
      let projected_min_common = &*projected_min_common;

      let lowest = if let Some(&lowest) = projected_min_common.get(0) {
        lowest
      } else {
        continue;
      };

      let mut projected_graph =
        ProjectedGraph::from_dataset(item_type, lowest, &dataset);

      for &min_common in projected_min_common {
        println!(
          "running projected graph for {} with min common {}",
          prefix, min_common
        );

        projected_graph =
          projected_graph.filter_edges(dataset.len(item_type), min_common);

        let output_dir: PathBuf = output_dir
          .join(&format!("projected_{}", prefix))
          .join(format!("min_common_{}", min_common));

        fs::create_dir_all(&output_dir)?;

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

        for name in &subgraph_names[item_type] {
          let idx = dataset.find_item(item_type, name).unwrap();

          println!("saving subgraph for {:?} {}", item_type, name);

          save_subgraph(
            &output_dir,
            idx,
            subgraph_limit,
            &projected_graph,
            item_type,
            &dataset,
          )?;
        }
      }
    }
  }

  Ok(())
}
