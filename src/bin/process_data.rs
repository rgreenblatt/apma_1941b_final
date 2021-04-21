use anyhow::Result;
use github_net::{
  component_sizes_csv::save_component_sizes,
  configuration_model,
  connection_str_stats::save_connection_str_stats,
  connection_strength::*,
  contribution_dist_csv::{
    save_contribution_dist, save_contribution_dist_item,
  },
  dataset::{Dataset, DatasetInfo, DatasetNameID, Lens},
  degree_dist_csv::save_degrees,
  distances::{average_distance, compute_pseudo_diameter},
  item_name_to_save_name,
  projected_graph::ProjectedGraph,
  save_subgraph::save_subgraph,
  traversal::Node,
  ItemType, UserRepoPair,
};
use rand::prelude::*;
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

  /// Don't run analysis on the original network.
  #[structopt(long)]
  no_original_network: bool,

  /// Also run analysis on the configuration model with the same degrees.
  #[structopt(long)]
  use_configuration_model: bool,

  /// Eliminate users with very large contribution to remove (some) bots and
  /// spammers.
  #[structopt(long, default_value = "500000")]
  max_user_contributions: usize,

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

  /// Compute pseudo diameter of the giant component.
  #[structopt(long, requires("components"))]
  pseudo_diameter: bool,

  /// Compute average distance in the giant component using some number of
  /// samples.
  #[structopt(long, requires("components"))]
  average_distance_samples: Option<usize>,

  /// Save the projected subgraph close to this user.
  #[structopt(long, use_delimiter = true)]
  subgraph_user: Vec<String>,

  /// Save the subgraph close to this repo.
  #[structopt(long, use_delimiter = true)]
  subgraph_repo: Vec<String>,

  #[structopt(long, default_value = "3")]
  subgraph_limit: usize,

  /// How strong the connection must be to keep a edge in the user projected
  /// graph.
  #[structopt(long, use_delimiter = true)]
  user_min_connection_str: Vec<f64>,

  /// How strong the connection must be to keep a edge in the repo projected
  /// graph.
  #[structopt(long, use_delimiter = true)]
  repo_min_connection_str: Vec<f64>,

  /// What type of connection strength metrics to use - typically just 1 should
  /// be specified.
  #[structopt(long, use_delimiter = true)]
  connection_str_types: Vec<ConnectionStrengthTypes>,

  /// Compute and save statistics about connection strengths.
  #[structopt(long)]
  connection_str_stats: bool,

  #[structopt(long, default_value = "0", use_delimiter = true)]
  min_contribution: Vec<usize>,
}

fn run_degrees(
  output_dir: &Path,
  dataset: &Dataset,
  dataset_info: &impl DatasetNameID,
) -> Result<()> {
  let deg_names = UserRepoPair {
    user: "user_degrees.csv",
    repo: "repo_degrees.csv",
  };

  for (item_type, name) in deg_names.iter_with() {
    save_degrees(
      &output_dir.join(name),
      item_type,
      dataset,
      dataset_info,
      |items: &[_]| items.len(),
    )?
  }

  let total_names = UserRepoPair {
    user: "user_total_contributions.csv",
    repo: "repo_total_events.csv",
  };

  for (item_type, name) in total_names.iter_with() {
    save_degrees(
      &output_dir.join(name),
      item_type,
      dataset,
      dataset_info,
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

struct RunConnectionStrArgs<'a, D: DatasetNameID> {
  output_dir: &'a Path,
  min_connection_str: &'a mut UserRepoPair<Vec<f64>>,
  subgraph_names: UserRepoPair<&'a Vec<String>>,
  subgraph_limit: usize,
  dataset: &'a Dataset,
  dataset_info: &'a D,
  connection_str_stats: bool,
}

fn run_connection_outer<T: ConnectionStrength, D: DatasetNameID>(
  args: RunConnectionStrArgs<'_, D>,
  inner: T,
  norm: bool,
) -> Result<()> {
  let accelerators = &UserRepoPair::<()>::default().map_with(|_, item_type| {
    ExpectationAccelerator::new(item_type, args.dataset)
  });
  if norm {
    run_connection_str(
      args,
      Normalized {
        inner,
        accelerators,
      },
      accelerators,
    )
  } else {
    run_connection_str(args, inner, accelerators)
  }
}

fn run_connection_str<
  'a,
  T: ConnectionStrength,
  V: ConnectionStrength,
  D: DatasetNameID,
>(
  args: RunConnectionStrArgs<'a, D>,
  connection_strength: T,
  accelerators: &UserRepoPair<ExpectationAccelerator<V>>,
) -> Result<()> {
  let RunConnectionStrArgs {
    output_dir,
    min_connection_str,
    subgraph_names,
    subgraph_limit,
    dataset,
    dataset_info,
    connection_str_stats,
  } = args;

  let prefixs = UserRepoPair {
    user: "user",
    repo: "repo",
  };

  for (item_type, prefix) in prefixs.iter_with() {
    let output_dir: PathBuf = output_dir
      .join(&format!("projected_{}", prefix))
      .join(format!("{:?}", connection_strength));

    fs::create_dir_all(&output_dir)?;

    println!(
      "running projected graph for {} with connection strength type {:?}",
      prefix, connection_strength,
    );

    if connection_str_stats {
      save_connection_str_stats(
        &output_dir,
        item_type,
        &connection_strength,
        &accelerators[item_type],
        dataset,
        dataset_info,
      )?;
    }

    let min_connection_str = &mut min_connection_str[item_type];

    min_connection_str.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min_connection_str: Vec<T::Value> = min_connection_str
      .iter()
      .map(|v| ConnectionStrengthValue::from_float(*v))
      .collect::<anyhow::Result<_>>()?;

    let lowest = if let Some(lowest) = min_connection_str.get(0) {
      lowest
    } else {
      continue;
    };

    let mut projected_graph = ProjectedGraph::from_dataset(
      item_type,
      &connection_strength,
      lowest,
      dataset,
    );

    for ref min_connection_str in min_connection_str {
      println!("running for min connection strength {}", min_connection_str);

      projected_graph = projected_graph
        .filter_edges(dataset.lens()[item_type], min_connection_str);

      let output_dir: PathBuf =
        output_dir.join(format!("min_connection_str_{}", &min_connection_str));

      fs::create_dir_all(&output_dir)?;

      for name in subgraph_names[item_type] {
        let idx = dataset_info.find_item(item_type, name).unwrap();

        println!("saving subgraph for {:?} {}", item_type, name);

        save_subgraph(
          &output_dir,
          idx,
          subgraph_limit,
          &projected_graph,
          item_type,
          dataset_info,
        )?;
      }
    }
  }

  Ok(())
}

fn run(
  opts: &Opt,
  dataset: &mut Dataset,
  dataset_info: &impl DatasetNameID,
  output_dir: &Path,
) -> Result<()> {
  let Opt {
    contribution,
    contributions_for_user,
    contributions_for_repo,
    degrees,
    components,
    pseudo_diameter,
    average_distance_samples,
    subgraph_user,
    subgraph_repo,
    subgraph_limit,
    user_min_connection_str,
    repo_min_connection_str,
    connection_str_types,
    connection_str_stats,
    min_contribution,
    ..
  } = opts;

  let contribution_names = UserRepoPair {
    user: (contributions_for_user, "user"),
    repo: (contributions_for_repo, "repo"),
  };

  let subgraph_names = UserRepoPair {
    user: subgraph_user,
    repo: subgraph_repo,
  };

  let min_connection_str = &mut UserRepoPair {
    user: user_min_connection_str.clone(),
    repo: repo_min_connection_str.clone(),
  };

  let contributions_dir = output_dir.join("contributions");

  fs::create_dir_all(&contributions_dir)?;

  if *contribution {
    println!("running contribution");
    save_contribution_dist(
      &contributions_dir.join("overall.csv"),
      &dataset,
      dataset_info,
    )?;
  }

  for (item_type, (names, item_name)) in contribution_names.iter_with() {
    for name in names {
      println!("running contribution dist for {} {}", item_name, name);
      let idx = dataset_info.find_item(item_type, &name).unwrap();
      save_contribution_dist_item(
        &contributions_dir.join(format!(
          "{}_{}.csv",
          item_name,
          item_name_to_save_name(&name)
        )),
        item_type,
        idx,
        &dataset,
        dataset_info,
      )?;
    }
  }

  let mut min_contribution = min_contribution.clone();
  min_contribution.sort_unstable();

  for min_contribution in min_contribution {
    println!("running for min contributions {}", min_contribution);
    dataset.filter_contributions(min_contribution);

    let output_dir =
      output_dir.join(format!("min_contribution_{}", min_contribution));

    fs::create_dir_all(&output_dir)?;

    if *degrees {
      println!("running degrees");
      run_degrees(&output_dir, dataset, dataset_info)?
    }

    if *components {
      println!("running components");
      let giant_component = save_component_sizes(
        &dataset,
        &output_dir.join("component_sizes.csv"),
      )?;

      if let Some(giant_component) = giant_component {
        let giant_n_repos = giant_component.repo.len();
        let total_n_repos = dataset.lens().repo;
        if giant_n_repos < total_n_repos / 4 {
          println!(
            "WARN! giant component is quite small ({} / {} repos)",
            giant_n_repos, total_n_repos
          );
        }
        if *pseudo_diameter {
          println!("running pseudo diameter");

          let pseudo_diameter = compute_pseudo_diameter(
            Node {
              item_type: ItemType::Repo,
              idx: giant_component[ItemType::Repo][0],
            },
            &dataset,
          );

          println!("found pseudo diameter {}", pseudo_diameter);
        }

        if let Some(num_samples) = average_distance_samples {
          println!("running average distances");

          let distances =
            average_distance(&giant_component, *num_samples, &dataset);

          let total = distances.iter().map(|&(_, d)| d).sum::<f64>();
          let total_sqr =
            distances.iter().map(|&(_, d)| d.powi(2)).sum::<f64>();

          let avg = total / distances.len() as f64;
          let avg_sqr = total_sqr / distances.len() as f64;
          let var = avg_sqr - avg.powi(2);

          println!(
            "average distance is {} while variance of samples is {}",
            avg, var
          );
        }
      } else {
        println!(
          "Giant component wasn't found, so computations will be skipped!"
        );
      }
    }

    for &t in connection_str_types {
      let args = RunConnectionStrArgs {
        output_dir: &output_dir,
        min_connection_str,
        subgraph_limit: *subgraph_limit,
        subgraph_names,
        connection_str_stats: *connection_str_stats,
        dataset,
        dataset_info,
      };

      type CST = ConnectionStrengthTypes;
      match t {
        CST::NumCommonNodes(norm) => {
          run_connection_outer(args, NumCommonNodes::default(), norm)
        }
        CST::MinNumEvents(norm) => {
          run_connection_outer(args, MinNumEvents::default(), norm)
        }

        CST::TotalNumEvents(norm) => {
          run_connection_outer(args, TotalNumEvents::default(), norm)
        }
        CST::GeometricMeanEvents(norm) => {
          run_connection_outer(args, GeometricMeanEvents::default(), norm)
        }
      }?;
    }
  }

  Ok(())
}

pub fn main() -> Result<()> {
  let opt = Opt::from_args();

  let output_dir = PathBuf::from("output_data");

  if opt.use_configuration_model || !opt.no_original_network {
    let (dataset_info, dataset) =
      DatasetInfo::load_limited(opt.limit, Some(opt.max_user_contributions))?;

    println!("users: {}", dataset.lens().user);
    println!("repos: {}", dataset.lens().repo);
    println!("connections: {}", dataset.contributions().len());

    if opt.use_configuration_model {
      println!("=== running for configuration model ===\n");

      let mut rng = StdRng::seed_from_u64(812388383);

      run(
        &opt,
        &mut configuration_model::gen_graph(&dataset, &mut rng),
        &dataset_info,
        &output_dir.join("configuration_model"),
      )?;
    }
    if !opt.no_original_network {
      let mut dataset = dataset;
      println!("=== running for actual network ===\n");
      run(
        &opt,
        &mut dataset,
        &dataset_info,
        &output_dir.join("actual_graph"),
      )?;
    }
  }

  Ok(())
}
