use crate::{
  connection_strength::{ConnectionStrength, ConnectionStrengthValue},
  dataset::Dataset,
  output_data::csv_writer,
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
  pub example_name: String,
}

#[derive(Serialize)]
pub struct ConnectionStrengthCsvEntry<S: Serialize> {
  pub strength: S,
  pub count: usize,
  pub example_name_start: String,
  pub example_name_end: String,
}

pub fn save_connection_str_stats<T: ConnectionStrength>(
  output_dir: &Path,
  item_type: ItemType,
  dataset: &Dataset,
) -> Result<()> {
  let mut degree_counts = Map::default();
  let mut strength_counts = Map::default();

  let f = |start_idx: usize, mut edge_map: Map<_, _>| {
    let names: &[String] = &dataset.names()[item_type];
    let example_name_start = names[start_idx].clone();
    degree_counts
      .entry(edge_map.len())
      .or_insert((0, example_name_start.clone()))
      .0 += 1;
    for (end_idx, strength) in edge_map.drain() {
      let end_idx: usize = end_idx;
      let example_name_end = names[end_idx].clone();
      let strength =
        T::normalize(strength, item_type, start_idx, end_idx, dataset).bin();
      strength_counts
        .entry(strength)
        .or_insert((0, example_name_start.clone(), example_name_end))
        .0 += 1;
    }
  };

  ProjectedGraph::<T>::transitive_edge_compute(item_type, dataset, f);

  let mut degree_writer = csv_writer(&output_dir.join("degrees.csv"))?;

  let mut degree_count: Vec<_> = degree_counts.into_iter().collect();
  degree_count.sort_unstable_by_key(|item| item.0.clone());

  for (degree, (count, example_name)) in degree_count {
    degree_writer.serialize(DegreeCsvEntry {
      degree,
      count,
      example_name,
    })?;
  }

  let mut strength_writer = csv_writer(&output_dir.join("strengths.csv"))?;

  let mut strength_count: Vec<_> = strength_counts.into_iter().collect();
  strength_count.sort_unstable_by_key(|item| item.0.clone());

  for (strength, (count, example_name_start, example_name_end)) in
    strength_count
  {
    strength_writer.serialize(ConnectionStrengthCsvEntry {
      strength: strength.to_serializable(),
      count,
      example_name_start,
      example_name_end,
    })?;
  }

  Ok(())
}
