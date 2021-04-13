use crate::{
  components::components_callback,
  dataset::Dataset,
  output_data::{csv_reader, csv_writer},
  progress_bar::get_bar,
  ItemType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize)]
pub struct ComponentSizeCsvEntry {
  pub user_size: usize,
  pub repo_size: usize,
  pub count: usize,
}

pub fn save_component_sizes(dataset: &Dataset, csv_path: &Path) -> Result<()> {
  let mut counts = HashMap::new();
  let bar = get_bar(
    Some(
      dataset
        .names()
        .as_ref()
        .into_iter()
        .map(|v| v.len() as u64)
        .sum(),
    ),
    1000,
  );
  for item in components_callback(dataset, |_| bar.inc(1))
    .map(|component| (component.user.len(), component.repo.len()))
  {
    *counts.entry(item).or_insert(0) += 1;
  }

  let mut writer = csv_writer(csv_path)?;

  let mut total_user_size = 0;
  let mut total_repo_size = 0;
  for ((user_size, repo_size), count) in counts {
    total_user_size += user_size * count;
    total_repo_size += repo_size * count;
    writer.serialize(ComponentSizeCsvEntry {
      user_size,
      repo_size,
      count,
    })?;
  }
  assert_eq!(total_user_size, dataset.len(ItemType::User));
  assert_eq!(total_repo_size, dataset.len(ItemType::Repo));

  Ok(())
}

pub fn load_component_sizes(
  csv_path: &Path,
) -> Result<impl Iterator<Item = csv::Result<ComponentSizeCsvEntry>>> {
  Ok(csv_reader(csv_path)?.into_deserialize())
}
