use crate::{dataset::Dataset, output_data::csv_writer, ItemType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry {
  pub degree: usize,
  pub count: usize,
  pub example_name: String,
}

pub fn save_degrees(
  csv_path: &Path,
  item_type: ItemType,
  dataset: &Dataset,
  get_degree: impl Fn(&[usize]) -> usize,
) -> Result<()> {
  let mut degree_count = HashMap::new();
  for (v, name) in dataset.contribution_idxs()[item_type]
    .iter()
    .zip(&dataset.names()[item_type])
  {
    let degree = get_degree(v);
    degree_count.entry(degree).or_insert((0, name.clone())).0 += 1;
  }

  save_sort_items(
    csv_path,
    degree_count,
    |(degree, _)| *degree,
    |(degree, (count, example_name))| DegreeCsvEntry {
      degree,
      count,
      example_name,
    },
  )
}

pub fn save_sort_items<T, K, E>(
  csv_path: &Path,
  items: impl IntoIterator<Item = T>,
  get_sort_key: impl Fn(&T) -> K,
  get_entry: impl Fn(T) -> E,
) -> Result<()>
where
  K: Ord,
  E: Serialize,
{
  let mut writer = csv_writer(csv_path)?;

  let mut to_sort: Vec<_> = items.into_iter().collect();
  to_sort.sort_unstable_by_key(get_sort_key);

  for item in to_sort {
    writer.serialize(get_entry(item))?;
  }

  Ok(())
}
