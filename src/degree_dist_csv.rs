use crate::{dataset::Dataset, ItemType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
  collections::HashMap,
  fs::{self, File},
  path::Path,
};

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry {
  pub degree: usize,
  pub count: usize,
  pub example_name: String,
}

pub fn save_degree_item<F: Fn(&[usize]) -> usize>(
  item_type: ItemType,
  dataset: &Dataset,
  csv_path: &str,
  get_degree: F,
) -> Result<()> {
  let output_data_dir = Path::new("output_data/");

  fs::create_dir_all(output_data_dir)?;

  let mut degree_count = HashMap::new();
  for (degree, name) in dataset.contribution_idxs()[item_type]
    .iter()
    .zip(&dataset.names()[item_type])
    .map(|(v, name)| (get_degree(v), name.clone()))
  {
    degree_count.entry(degree).or_insert((0, name)).0 += 1;
  }

  let mut csv_writer =
    csv::Writer::from_writer(File::create(output_data_dir.join(csv_path))?);

  let mut degree_count: Vec<_> = degree_count.into_iter().collect();
  degree_count.sort_unstable_by_key(|item| item.0);

  for (degree, (count, example_name)) in degree_count {
    if degree < 1 {
      continue;
    }
    csv_writer.serialize(DegreeCsvEntry {
      degree,
      count,
      example_name,
    })?;
  }

  Ok(())
}
