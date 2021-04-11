use crate::{dataset::Dataset, output_data::csv_writer, ItemType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry {
  pub degree: usize,
  pub count: usize,
  pub example_name: String,
}

pub fn save_degree_item(
  item_type: ItemType,
  dataset: &Dataset,
  csv_path: &str,
  get_degree: impl Fn(&[usize]) -> usize,
) -> Result<()> {
  let mut degree_count = HashMap::new();
  for (degree, name) in dataset.contribution_idxs()[item_type]
    .iter()
    .zip(&dataset.names()[item_type])
    .map(|(v, name)| (get_degree(v), name.clone()))
  {
    degree_count.entry(degree).or_insert((0, name)).0 += 1;
  }

  let mut writer = csv_writer(csv_path)?;

  let mut degree_count: Vec<_> = degree_count.into_iter().collect();
  degree_count.sort_unstable_by_key(|item| item.0);

  for (degree, (count, example_name)) in degree_count {
    if degree < 1 {
      continue;
    }
    writer.serialize(DegreeCsvEntry {
      degree,
      count,
      example_name,
    })?;
  }

  Ok(())
}
