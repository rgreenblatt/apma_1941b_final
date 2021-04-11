use crate::{
  dataset::Dataset,
  github_types::{ItemType, UserRepoPair},
  output_data::output_data_dir,
  progress_bar::get_bar,
  traversal::{default_visited, traverse, Component, Node},
};
use anyhow::Result;
use std::{borrow::Cow, collections::HashSet, fs::File, io::BufWriter};

pub fn save_subgraph(
  start: Node,
  limit: usize,
  dataset: &Dataset,
) -> Result<()> {
  let mut visited = default_visited(dataset);
  start.set_visited(&mut visited);
  let mut component = start.into();

  let bar = get_bar(None, 1000);

  traverse(&mut component, &mut visited, dataset, Some(limit), |_| {
    bar.inc(1)
  });

  // let user_idx_to_new_id = component.
  //

  let name = dataset.names()[start.item_type][start.idx].replace('/', "_");
  let save_name = format!("sub_graph_for_{}.dot", name);

  let path = output_data_dir()?.join(save_name);
  let file = File::create(path)?;
  let mut writer = BufWriter::new(file);

  let sets = component
    .as_ref()
    .map(|idxs| idxs.iter().cloned().collect::<HashSet<usize>>());

  let graph = Graph {
    component,
    sets,
    dataset,
  };

  dot::render(&graph, &mut writer)?;

  Ok(())
}

struct Graph<'a> {
  component: Component,
  sets: UserRepoPair<HashSet<usize>>,
  dataset: &'a Dataset,
}
type Edge = usize;

impl<'a> dot::Labeller<'a, Node, Edge> for Graph<'a> {
  fn graph_id(&'a self) -> dot::Id<'a> {
    dot::Id::new("example1").unwrap()
  }

  fn node_id(&'a self, n: &Node) -> dot::Id<'a> {
    let name = self.dataset.names()[n.item_type][n.idx]
      .replace(&['/', '-', '?'][..], "_");
    let name = format!("{:?}_{}", n.item_type, name);
    let out = dot::Id::new(name.clone());
    if let Ok(out) = out {
      out
    } else {
      panic!("name isn't valued node id: \"{}\"", name);
    }
  }
}

impl<'a> dot::GraphWalk<'a, Node, Edge> for Graph<'a> {
  fn nodes(&self) -> dot::Nodes<'a, Node> {
    let nodes = self
      .component
      .as_ref()
      .iter_with_types()
      .flat_map(|(item_type, idxs)| {
        idxs.iter().map(move |&idx| Node { item_type, idx })
      })
      .collect();
    Cow::Owned(nodes)
  }

  fn edges(&'a self) -> dot::Edges<'a, Edge> {
    let mut edges: Vec<_> = self
      .component
      .as_ref()
      .iter_with_types()
      .flat_map(|(item_type, idxs)| {
        idxs.iter().flat_map(move |&idx| {
          self.dataset.contribution_idxs()[item_type][idx]
            .iter()
            .cloned()
        })
      })
      .filter(|&contrib_idx| {
        let contrib = &self.dataset.contributions()[contrib_idx];
        contrib
          .idx
          .as_ref()
          .iter_with_types()
          .all(|(item_type, idx)| self.sets[item_type].contains(idx))
      })
      .collect();

    edges.sort();
    edges.dedup();

    Cow::Owned(edges)
  }

  fn source(&self, i: &Edge) -> Node {
    Node {
      item_type: ItemType::User,
      idx: self.dataset.contributions()[*i].idx.user,
    }
  }

  fn target(&self, i: &Edge) -> Node {
    Node {
      item_type: ItemType::Repo,
      idx: self.dataset.contributions()[*i].idx.repo,
    }
  }
}
