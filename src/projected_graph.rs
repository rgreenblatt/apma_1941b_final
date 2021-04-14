//! TODO: allow for using normalized strength values.
use crate::{
  connection_strength::ConnectionStrength,
  dataset::{Contribution, Dataset},
  edge_vec::EdgeVec,
  progress_bar::get_bar,
  ItemType,
};
use fnv::FnvHashMap as Map;
use indicatif::ProgressIterator;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Edge<T: ConnectionStrength> {
  /// order is lowest to highest, but otherwise shouldn't matter
  pub node_idxs: [usize; 2],

  pub strength: T::Value,
}

pub struct ProjectedGraph<T: ConnectionStrength> {
  edges_v: Vec<Edge<T>>,
  edge_idxs_v: EdgeVec<usize>,
}

// We use a "for each" type construct for efficiency - external iterators are
// very slow if used naively in this context.
pub fn transitive_edge_compute(
  item_type: ItemType,
  dataset: &Dataset,
  mut f: impl FnMut(usize, Map<usize, (Vec<usize>, Vec<[usize; 2]>)>),
) {
  let num_items = dataset.len(item_type);

  let bar = get_bar(Some(num_items as u64), 10_000);

  let contrib_idx_to_item_idx = |item_type: ItemType, contrib_idx| {
    let contrib: Contribution = dataset.contributions()[contrib_idx];
    contrib.idx[item_type]
  };

  // constructing a new map each time is faster because the average case
  // has a small number of edges
  for (start_idx, contributions) in dataset.contribution_idxs()[item_type]
    .iter()
    .enumerate()
    .progress_with(bar)
  {
    let mut edge_map: Map<_, (Vec<usize>, Vec<[usize; 2]>)> = Map::default();

    for &first_contrib_idx in contributions {
      let middle_idx =
        contrib_idx_to_item_idx(item_type.other(), first_contrib_idx);
      for (end_idx, second_contrib_idx) in dataset.contribution_idxs()
        [item_type.other()][middle_idx]
        .iter()
        .map(|&contrib_idx| {
          (contrib_idx_to_item_idx(item_type, contrib_idx), contrib_idx)
        })
        .filter(|&(end_idx, _)| end_idx > start_idx)
      {
        let entry = edge_map.entry(end_idx).or_insert_with(Default::default);
        entry.0.push(middle_idx);
        entry.1.push([first_contrib_idx, second_contrib_idx]);
      }
    }

    f(start_idx, edge_map);
  }
}

impl<T> ProjectedGraph<T>
where
  T: ConnectionStrength,
{
  pub fn edges(&self) -> &[Edge<T>] {
    &self.edges_v
  }

  pub fn edge_idxs(&self) -> &EdgeVec<usize> {
    &self.edge_idxs_v
  }

  pub fn filter_edges(
    &self,
    num_items: usize,
    min_strength: &T::Value,
  ) -> Self {
    let edges = self
      .edges()
      .iter()
      .cloned()
      .filter(|e| &e.strength >= min_strength)
      .collect();

    Self::from_edges(num_items, edges)
  }

  fn from_edges(num_items: usize, edges_v: Vec<Edge<T>>) -> Self {
    let mut edge_idxs = vec![Vec::new(); num_items];

    let bar = get_bar(Some(edges_v.len() as u64), 100_000);

    for (i, &Edge { node_idxs, .. }) in
      edges_v.iter().enumerate().progress_with(bar)
    {
      for &idx in &node_idxs {
        edge_idxs[idx].push(i);
        edge_idxs[idx].push(i);
      }
    }

    let edge_idxs_v: EdgeVec<_> = edge_idxs.into_iter().collect();

    Self {
      edges_v,
      edge_idxs_v,
    }
  }

  pub fn from_dataset(
    item_type: ItemType,
    connection_strength: &T,
    min_strength: &T::Value,
    dataset: &Dataset,
  ) -> Self {
    let mut edges = Vec::new();

    let f = |start_idx, mut edge_map: Map<_, (Vec<usize>, Vec<[usize; 2]>)>| {
      edges.extend(edge_map.drain().filter_map(
        |(end_idx, (common_other_idxs, contrib_idxs))| {
          let strength = connection_strength.strength(
            item_type,
            &contrib_idxs,
            &common_other_idxs,
            dataset,
          );
          if strength >= *min_strength {
            let edge = Edge {
              node_idxs: [start_idx, end_idx],
              strength,
            };
            Some(edge)
          } else {
            None
          }
        },
      ))
    };

    transitive_edge_compute(item_type, dataset, f);

    Self::from_edges(dataset.len(item_type), edges)
  }
}
