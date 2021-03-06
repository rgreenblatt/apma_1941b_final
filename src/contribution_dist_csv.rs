use crate::{
  dataset::{Contribution, Dataset, DatasetNameID},
  output_data::csv_writer,
  ItemType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize)]
struct ContributionCsvEntry<ID> {
  num: usize,
  count: usize,
  example_user: ID,
  example_repo: ID,
}

fn save_contribution_dist_impl<'a>(
  csv_path: &'a Path,
  contributions: impl IntoIterator<Item = &'a Contribution>,
  dataset_info: &impl DatasetNameID,
) -> Result<()> {
  let mut contrib_count = HashMap::new();
  for &Contribution { num, idx } in contributions {
    contrib_count
      .entry(num)
      .or_insert_with(|| {
        (
          0,
          dataset_info.user_id(idx.user),
          dataset_info.repo_id(idx.repo),
        )
      })
      .0 += 1;
  }

  let mut writer = csv_writer(csv_path)?;

  let mut contrib_count: Vec<_> = contrib_count.into_iter().collect();
  contrib_count.sort_unstable_by_key(|item| item.0);

  for (num, (count, example_user, example_repo)) in contrib_count {
    writer.serialize(ContributionCsvEntry {
      num,
      count,
      example_user,
      example_repo,
    })?;
  }

  Ok(())
}

pub fn save_contribution_dist(
  csv_path: &Path,
  dataset: &Dataset,
  dataset_info: &impl DatasetNameID,
) -> Result<()> {
  save_contribution_dist_impl(csv_path, dataset.contributions(), dataset_info)
}

pub fn save_contribution_dist_item(
  csv_path: &Path,
  item_type: ItemType,
  idx: usize,
  dataset: &Dataset,
  dataset_info: &impl DatasetNameID,
) -> Result<()> {
  save_contribution_dist_impl(
    csv_path,
    dataset.contribution_idxs()[item_type][idx]
      .iter()
      .map(|&contrib_idx| &dataset.contributions()[contrib_idx]),
    dataset_info,
  )
}
