use crate::{
  connection_strength::ConnectionStrength,
  dataset::DatasetNameID,
  item_name_to_save_name,
  progress_bar::get_bar,
  projected_graph::ProjectedGraph,
  traversal::{projected_make_component_dists, projected_traverse_dist},
  ItemType,
};
use anyhow::Result;
use fnv::FnvHashMap as Map;
use std::{borrow::Cow, fs::File, io::BufWriter, path::Path};

pub fn save_subgraph<T: ConnectionStrength>(
  output_dir: &Path,
  start: usize,
  limit: usize,
  projected_graph: &ProjectedGraph<T>,
  item_type: ItemType,
  dataset_info: &impl DatasetNameID,
) -> Result<()> {
  let mut visited = vec![false; dataset_info.lens()[item_type]];
  visited[start] = true;
  let mut component = projected_make_component_dists(start);

  let bar = get_bar(None, 1000);

  projected_traverse_dist(
    &mut component,
    &mut visited,
    projected_graph,
    Some(limit),
    |_, _| bar.inc(1),
  );

  let name = dataset_info.get_name(item_type, start);

  let map: Map<_, _> = component
    .idxs()
    .iter()
    .cloned()
    .zip(component.dists().iter().cloned())
    .collect();

  println!("saving {} with {} items", name, map.len(),);

  let mut use_point_vals = vec![true];
  if map.len() < 10_000 {
    use_point_vals.push(false);
  }

  for use_point in use_point_vals {
    let graph = Graph {
      use_point,
      map: &map,
      projected_graph,
      item_type,
      dataset_info,
    };

    let end = if use_point { "" } else { "_with_names" };

    let save_name =
      format!("sub_graph_for_{}{}.dot", item_name_to_save_name(&name), end);
    let path = output_dir.join(save_name);
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    dot::render(&graph, &mut writer)?;
  }

  Ok(())
}

type Node = usize;
type Edge = [usize; 2];

struct Graph<'a, T: ConnectionStrength, D: DatasetNameID> {
  map: &'a Map<usize, usize>,
  projected_graph: &'a ProjectedGraph<T>,
  item_type: ItemType,
  dataset_info: &'a D,
  use_point: bool,
}

impl<'a, T: ConnectionStrength, D: DatasetNameID> dot::Labeller<'a, Node, Edge>
  for Graph<'a, T, D>
{
  fn graph_id(&'a self) -> dot::Id<'a> {
    dot::Id::new("G").unwrap()
  }

  fn node_id(&'a self, n: &Node) -> dot::Id<'a> {
    dot::Id::new(format!("_{}", n)).unwrap()
  }

  fn node_label(&'a self, n: &Node) -> dot::LabelText<'a> {
    dot::LabelText::LabelStr(Cow::Owned(
      self.dataset_info.get_name(self.item_type, *n),
    ))
  }

  fn node_shape(&'a self, _node: &Node) -> Option<dot::LabelText<'a>> {
    if self.use_point {
      Some(dot::LabelText::LabelStr(Cow::Borrowed("point")))
    } else {
      None
    }
  }

  fn edge_color(&'a self, e: &Edge) -> Option<dot::LabelText<'a>> {
    let dist = e.iter().map(|i| *self.map.get(i).unwrap()).min().unwrap();
    Some(dot::LabelText::LabelStr(
      format!("/dark28/{}", dist + 1).into(),
    ))
  }

  fn kind(&self) -> dot::Kind {
    dot::Kind::Graph
  }
}

impl<'a, T: ConnectionStrength, D: DatasetNameID> dot::GraphWalk<'a, Node, Edge>
  for Graph<'a, T, D>
{
  fn nodes(&self) -> dot::Nodes<'a, Node> {
    let nodes = self.map.keys().cloned().collect();
    Cow::Owned(nodes)
  }

  fn edges(&'a self) -> dot::Edges<'a, Edge> {
    let edges = self
      .projected_graph
      .edges()
      .iter()
      .cloned()
      .filter_map(|edge| {
        if edge.node_idxs.iter().all(|idx| self.map.get(idx).is_some()) {
          Some(edge.node_idxs)
        } else {
          None
        }
      })
      .collect();
    Cow::Owned(edges)
  }

  fn source(&self, edge: &Edge) -> Node {
    edge[0]
  }

  fn target(&self, edge: &Edge) -> Node {
    edge[1]
  }
}
