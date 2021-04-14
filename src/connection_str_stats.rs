use crate::{
  connection_strength::{
    bin_float, ConnectionStrength, ConnectionStrengthValue,
    ExpectationAccelerator,
  },
  dataset::Dataset,
  degree_dist_csv::save_sort_items,
  projected_graph::transitive_edge_compute,
  ItemType,
};
use anyhow::Result;
use fnv::FnvHashMap as Map;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry {
  pub degree: usize,
  pub count: usize,
}

#[derive(Serialize)]
pub struct ConnectionStrengthCsvEntry<S: Serialize> {
  pub strength: S,
  pub count: usize,
}

pub fn save_connection_str_stats<
  T: ConnectionStrength,
  V: ConnectionStrength,
>(
  output_dir: &Path,
  item_type: ItemType,
  connection_strength: &T,
  accelerator: &ExpectationAccelerator<V>,
  dataset: &Dataset,
) -> Result<()> {
  let mut degree_counts = Map::default();
  let mut strength_counts = Map::default();
  let mut strength_normalized_counts = Map::default();
  let mut expected_counts = Map::default();
  let mut total_strength = 0.;
  let mut total_sqr_strength = 0.;
  let mut count = 0;

  let f = |start_idx: usize,
           mut edge_map: Map<_, (Vec<usize>, Vec<[usize; 2]>)>| {
    *degree_counts.entry(edge_map.len()).or_insert(0) += 1;
    for (end_idx, (common_other_idxs, contrib_idxs)) in edge_map.drain() {
      let end_idx: usize = end_idx;

      let strength = connection_strength.strength(
        item_type,
        &contrib_idxs,
        &common_other_idxs,
        dataset,
      );
      let expected =
        accelerator.expectation([start_idx, end_idx], &common_other_idxs);

      *strength_counts.entry(strength.clone().bin()).or_insert(0) += 1;
      *strength_normalized_counts
        .entry(bin_float(strength.clone().to_float() / expected))
        .or_insert(0) += 1;
      *expected_counts.entry(bin_float(expected)).or_insert(0) += 1;

      let strength = strength.to_float();
      total_strength += strength;
      total_sqr_strength += strength.powi(2);
      count += 1;
    }
  };

  transitive_edge_compute(item_type, dataset, f);

  let mean_strength = total_strength / count as f64;
  let mean_sqr_strength = total_sqr_strength / count as f64;

  println!(
    "strength variance is {}",
    mean_sqr_strength - mean_strength.powi(2)
  );

  save_sort_items(
    &output_dir.join("degrees.csv"),
    degree_counts,
    |(degree, _)| degree.clone(),
    |(degree, count)| DegreeCsvEntry { degree, count },
  )?;

  save_sort_items(
    &output_dir.join("strengths.csv"),
    strength_counts,
    |(strength, _)| strength.clone(),
    |(strength, count)| ConnectionStrengthCsvEntry {
      strength: strength.to_serializable(),
      count,
    },
  )?;

  save_sort_items(
    &output_dir.join("strengths_normalized.csv"),
    strength_normalized_counts,
    |(strength, _)| strength.clone(),
    |(strength, count)| ConnectionStrengthCsvEntry {
      strength: strength.to_serializable(),
      count,
    },
  )?;

  save_sort_items(
    &output_dir.join("expected.csv"),
    expected_counts,
    |(expected, _)| expected.clone(),
    |(expected, count)| ConnectionStrengthCsvEntry {
      strength: expected.into_inner(),
      count,
    },
  )?;

  Ok(())
}
