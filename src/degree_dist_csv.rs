use crate::{
  dataset::{Dataset, DatasetNameID},
  output_data::csv_writer,
  ItemType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry<ID> {
  pub degree: usize,
  pub count: usize,
  pub example_id: ID,
}

pub fn save_degrees<D: DatasetNameID>(
  csv_path: &Path,
  item_type: ItemType,
  dataset: &Dataset,
  dataset_info: &D,
  get_degree: impl Fn(&[usize]) -> usize,
) -> Result<()> {
  let mut degree_count = HashMap::new();
  for (i, idxs) in dataset.contribution_idxs()[item_type].iter().enumerate() {
    let degree = get_degree(idxs);
    degree_count
      .entry(degree)
      .or_insert((0, dataset_info.get_id(item_type, i)))
      .0 += 1;
  }

  save_sort_items(
    csv_path,
    degree_count,
    |(degree, _)| *degree,
    |(degree, (count, example_id))| DegreeCsvEntry {
      degree,
      count,
      example_id,
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
