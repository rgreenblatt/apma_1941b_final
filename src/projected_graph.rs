use crate::{
  dataset::{Contribution, Dataset},
  edge_vec::EdgeVec,
  github_types::ItemType,
  progress_bar::get_bar,
};
use fnv::FnvHashMap as Map;
use indicatif::ProgressIterator;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Edge {
  /// order is lowest to highest, but otherwise shouldn't matter
  pub node_idxs: [usize; 2],

  pub num: usize,
}

pub struct ProjectedGraph {
  edges_v: Vec<Edge>,
  edge_idxs_v: EdgeVec<usize>,
}

impl ProjectedGraph {
  pub fn edges(&self) -> &[Edge] {
    &self.edges_v
  }

  pub fn edge_idxs(&self) -> &EdgeVec<usize> {
    &self.edge_idxs_v
  }

  pub fn filter_edges(&self, num_items: usize, min_common: usize) -> Self {
    let edges = self
      .edges()
      .iter()
      .cloned()
      .filter(|e| e.num >= min_common)
      .collect();

    Self::from_edges(num_items, edges)
  }

  fn from_edges(num_items: usize, edges_v: Vec<Edge>) -> Self {
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
    min_common: usize,
    dataset: &Dataset,
  ) -> ProjectedGraph {
    let num_items = dataset.len(item_type);

    let bar = get_bar(Some(num_items as u64), 10_000);

    let contrib_idx_to_item_idx = |item_type: ItemType| {
      move |&contrib_idx| {
        let contrib: Contribution = dataset.contributions()[contrib_idx];
        contrib.idx[item_type]
      }
    };

    let mut edges = Vec::new();

    for (start_idx, contributions) in dataset.contribution_idxs()[item_type]
      .iter()
      .enumerate()
      .progress_with(bar)
    {
      // constructing a new map each time is faster because the average case
      // has a small number of edges
      let mut edge_map = Map::default();

      for middle_idx in contributions
        .iter()
        .map(contrib_idx_to_item_idx(item_type.other()))
      {
        for end_idx in dataset.contribution_idxs()[item_type.other()]
          [middle_idx]
          .iter()
          .map(contrib_idx_to_item_idx(item_type))
          .filter(|&end_idx| end_idx > start_idx)
        {
          *edge_map.entry(end_idx).or_insert(0) += 1;
        }
      }

      edges.extend(edge_map.drain().filter_map(|(end_idx, num)| {
        if num >= min_common {
          let edge = Edge {
            node_idxs: [start_idx, end_idx],
            num,
          };
          Some(edge)
        } else {
          None
        }
      }))
    }

    Self::from_edges(num_items, edges)
  }
}
