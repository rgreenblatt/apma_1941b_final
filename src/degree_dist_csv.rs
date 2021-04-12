use crate::{
  dataset::Dataset, output_data::csv_writer, projected_graph::ProjectedGraph,
  ItemType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry {
  pub degree: usize,
  pub count: usize,
  pub example_name: String,
}

pub fn save_degrees_projected_graph(
  csv_path: &Path,
  item_type: ItemType,
  projected_graph: &ProjectedGraph,
  dataset: &Dataset,
  get_degree: impl Fn(&[usize]) -> usize,
) -> Result<()> {
  save_degrees(
    csv_path,
    projected_graph
      .edge_idxs()
      .iter()
      .zip(&dataset.names()[item_type])
      .map(|(v, name)| (get_degree(v), name.clone())),
  )
}

pub fn save_degrees_dataset(
  csv_path: &Path,
  item_type: ItemType,
  dataset: &Dataset,
  get_degree: impl Fn(&[usize]) -> usize,
) -> Result<()> {
  save_degrees(
    csv_path,
    dataset.contribution_idxs()[item_type]
      .iter()
      .zip(&dataset.names()[item_type])
      .map(|(v, name)| (get_degree(v), name.clone())),
  )
}

pub fn save_degrees(
  csv_path: &Path,
  degrees: impl IntoIterator<Item = (usize, String)>,
) -> Result<()> {
  let mut degree_count = HashMap::new();
  for (degree, name) in degrees {
    degree_count.entry(degree).or_insert((0, name)).0 += 1;
  }

  let mut writer = csv_writer(csv_path)?;

  let mut degree_count: Vec<_> = degree_count.into_iter().collect();
  degree_count.sort_unstable_by_key(|item| item.0);

  for (degree, (count, example_name)) in degree_count {
    writer.serialize(DegreeCsvEntry {
      degree,
      count,
      example_name,
    })?;
  }

  Ok(())
}
