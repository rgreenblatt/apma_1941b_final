use crate::{
  dataset::Dataset,
  output_data::output_data_dir,
  progress_bar::get_bar,
  traversal::{self, default_visited, traverse},
};
use anyhow::Result;
use fnv::{FnvHashMap as Map, FnvHashSet as Set};
use std::{borrow::Cow, fs::File, io::BufWriter, iter};

pub fn save_subgraph(
  start: traversal::Node,
  limit: usize,
  repo_degree_thresh: usize,
  common_users_thresh: usize,
  dataset: &Dataset,
) -> Result<()> {
  let mut visited = default_visited(dataset);
  start.set_visited(&mut visited);
  let mut component = start.into();

  let bar = get_bar(None, 1000);

  traverse(&mut component, &mut visited, dataset, Some(limit), |_| {
    bar.inc(1)
  });

  let name =
    dataset.names()[start.item_type][start.idx].replace(&['/', '-'][..], "_");
  let graph_name = format!("sub_graph_for_{}", name);
  let graph_name = &graph_name;
  let save_name = format!("{}.dot", graph_name);

  let path = output_data_dir()?.join(save_name);
  let file = File::create(path)?;
  let mut writer = BufWriter::new(file);

  let user_set: Set<_> = component.user.iter().cloned().collect();

  let repo_set: Set<_> = component
    .repo
    .iter()
    .cloned()
    .filter(|&idx| dataset.repo_contributions()[idx].len() > repo_degree_thresh)
    .collect();

  let mut edge_counts = Map::default();

  let bar = get_bar(None, 10_000);

  for &repo_idx in repo_set.iter() {
    for user_idx in
      dataset.repo_contributions()[repo_idx]
        .iter()
        .filter_map(|&contrib_idx| {
          let user_idx = dataset.contributions()[contrib_idx].idx.user;
          if user_set.contains(&user_idx) {
            Some(user_idx)
          } else {
            None
          }
        })
    {
      for item in dataset.user_contributions()[user_idx].iter().filter_map(
        |&contrib_idx| {
          bar.inc(1);
          let end_repo_idx = dataset.contributions()[contrib_idx].idx.repo;
          if repo_set.contains(&end_repo_idx) && repo_idx < end_repo_idx {
            Some((repo_idx, end_repo_idx))
          } else {
            None
          }
        },
      ) {
        *edge_counts.entry(item).or_insert(0) += 1;
      }
    }
  }

  let edge_set: Set<_> = edge_counts
    .into_iter()
    .filter_map(|(item, count)| {
      if count > common_users_thresh {
        Some(item)
      } else {
        None
      }
    })
    .collect();

  println!(
    "saving {} with {} repos and {} edges",
    graph_name,
    repo_set.len(),
    edge_set.len()
  );

  let graph = Graph {
    repo_set,
    edge_set,
    dataset,
    graph_name,
  };

  dot::render(&graph, &mut writer)?;

  Ok(())
}

type Node = usize;
type Edge = (usize, usize);

struct Graph<'a> {
  repo_set: Set<Node>,
  edge_set: Set<Edge>,
  dataset: &'a Dataset,
  graph_name: &'a str,
}

impl<'a> dot::Labeller<'a, Node, Edge> for Graph<'a> {
  fn graph_id(&'a self) -> dot::Id<'a> {
    dot::Id::new(self.graph_name).unwrap()
  }

  fn node_id(&'a self, n: &Node) -> dot::Id<'a> {
    // remove 'special' characters for graphviz
    let name = self.dataset.repo_names()[*n]
      .replace(&['/', '-', '?', '(', ')', '[', ']', '{', '}', '.'][..], "_");
    let first = name.chars().next().unwrap();
    // also avoid the first char being a number
    let name = if '0' <= first && first <= '9' {
      iter::once('_').chain(name.chars()).collect()
    } else {
      name
    };
    let name = format!("{}", name);
    let out = dot::Id::new(name.clone());
    if let Ok(out) = out {
      out
    } else {
      panic!("name isn't valid for dot: \"{}\"", name);
    }
  }
}

impl<'a> dot::GraphWalk<'a, Node, Edge> for Graph<'a> {
  fn nodes(&self) -> dot::Nodes<'a, Node> {
    let nodes = self.repo_set.iter().cloned().collect();
    Cow::Owned(nodes)
  }

  fn edges(&'a self) -> dot::Edges<'a, Edge> {
    let edges = self.edge_set.iter().cloned().collect();
    Cow::Owned(edges)
  }

  fn source(&self, edge: &Edge) -> Node {
    edge.0
  }

  fn target(&self, edge: &Edge) -> Node {
    edge.1
  }
}
