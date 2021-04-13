use crate::{
  connection_strength::{
    bin_float, ConnectionStrength, ConnectionStrengthValue,
  },
  dataset::Dataset,
  degree_dist_csv::save_sort_items,
  projected_graph::ProjectedGraph,
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

pub fn save_connection_str_stats<T: ConnectionStrength>(
  output_dir: &Path,
  item_type: ItemType,
  dataset: &Dataset,
) -> Result<()> {
  let mut degree_counts = Map::default();
  let mut strength_counts = Map::default();
  // let mut strength_normalized_counts = Map::default();
  // let mut expected_counts = Map::default();
  // let mut total_strength = 0.;
  // let mut total_expected = 0.;

  let f = |start_idx: usize, mut edge_map: Map<_, T::Value>| {
    *degree_counts.entry(edge_map.len()).or_insert(0) += 1;
    for (end_idx, strength) in edge_map.drain() {
      let end_idx: usize = end_idx;

      // let expected = T::expected(item_type, [start_idx, end_idx], dataset);

      *strength_counts.entry(strength.clone().bin()).or_insert(0) += 1;
      // strength_normalized_counts
      //   .entry(bin_float(strength.clone().to_float() / expected))
      //   .or_insert((0, example_name_start.clone(), example_name_end.clone()))
      //   .0 += 1;
      // expected_counts
      //   .entry(bin_float(expected))
      //   .or_insert((0, example_name_start.clone(), example_name_end))
      //   .0 += 1;

      // total_strength += strength.to_float();
      // total_expected += expected;
    }
  };

  ProjectedGraph::<T>::transitive_edge_compute(item_type, dataset, f);

  // println!(
  //   "total strength divided by total expected is {}",
  //   total_strength / total_expected
  // );

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

  // save_sort_items(
  //   &output_dir.join("strengths_normalized.csv"),
  //   strength_normalized_counts,
  //   |(strength, _)| strength.clone(),
  //   |(strength, (count, example_name_start, example_name_end))| {
  //     ConnectionStrengthCsvEntry {
  //       strength: strength.to_serializable(),
  //       count,
  //       example_name_start,
  //       example_name_end,
  //     }
  //   },
  // )?;

  // save_sort_items(
  //   &output_dir.join("expected.csv"),
  //   expected_counts,
  //   |(expected, _)| expected.clone(),
  //   |(expected, (count, example_name_start, example_name_end))| {
  //     ConnectionStrengthCsvEntry {
  //       strength: expected.into_inner(),
  //       count,
  //       example_name_start,
  //       example_name_end,
  //     }
  //   },
  // )?;

  Ok(())
}
