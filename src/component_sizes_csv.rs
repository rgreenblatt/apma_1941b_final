use crate::{
  components::components, dataset::Dataset, output_data::csv_writer,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct ComponentSizeCsvEntry {
  pub user_size: usize,
  pub repo_size: usize,
  pub count: usize,
}

pub fn save_component_sizes(dataset: &Dataset, csv_path: &str) -> Result<()> {
  let mut counts = HashMap::new();
  for item in components(dataset)
    .map(|component| (component.user.len(), component.repo.len()))
  {
    *counts.entry(item).or_insert(0) += 1;
  }

  let mut writer = csv_writer(csv_path)?;

  for ((user_size, repo_size), count) in counts {
    writer.serialize(ComponentSizeCsvEntry {
      user_size,
      repo_size,
      count,
    })?;
  }

  Ok(())
}
