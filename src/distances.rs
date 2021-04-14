use crate::{
  dataset::Dataset,
  progress_bar::get_bar,
  traversal::{default_visited, traverse, Component, Node},
  ItemType,
};
use indicatif::ParallelProgressIterator;
use rand::{distributions::Uniform, prelude::*};
use rayon::prelude::*;

pub fn compute_pseudo_diameter(
  giant_component_node: Node,
  dataset: &Dataset,
) -> usize {
  let mut next = giant_component_node;
  let mut last_max_dist = 0;
  let mut last_min_degree = 0;
  loop {
    let mut component = next.into();
    let mut visited = default_visited(dataset);
    next.set_visited(&mut visited);
    let mut max_dist = 0;
    let mut min_degree = 0;
    println!(
      "starting pseudo diameter traversal with dist {} and degree {}",
      last_max_dist, last_min_degree
    );

    let bar = get_bar(None, 10000);

    traverse(&mut component, &mut visited, dataset, None, |node, dist| {
      bar.inc(1);
      let degree = dataset.contribution_idxs()[node.item_type][node.idx].len();
      if dist > max_dist || (dist == max_dist && degree < min_degree) {
        next = node;
        max_dist = dist;
        min_degree = degree;
      }
    });

    println!(
      "finished pseudo diameter traversal with new dist {} and new degree {}",
      max_dist, min_degree
    );

    assert!(max_dist >= last_max_dist);

    if max_dist == last_max_dist {
      return max_dist;
    }
    last_max_dist = max_dist;
    last_min_degree = min_degree;
  }
}

pub fn average_distance(
  giant_component: &Component,
  num_samples: usize,
  dataset: &Dataset,
) -> Vec<(Node, f64)> {
  let bar = get_bar(Some(num_samples as u64), 10000);
  rayon::iter::repeatn((), num_samples)
    .progress_with(bar)
    .map(|_| {
      let mut rng = rand::thread_rng();

      let item_type =
        [ItemType::User, ItemType::Repo][Uniform::from(0..2).sample(&mut rng)];
      let comp_idx =
        Uniform::from(0..giant_component[item_type].len()).sample(&mut rng);
      let idx = giant_component[item_type][comp_idx];

      let node = Node { item_type, idx };

      let mut component = node.into();
      let mut visited = default_visited(dataset);
      node.set_visited(&mut visited);

      let mut total_dist = 0;
      let mut count = 0;

      traverse(&mut component, &mut visited, dataset, None, |_, dist| {
        total_dist += dist;
        count += 1;
      });

      (node, total_dist as f64 / count as f64)
    })
    .collect()
}
