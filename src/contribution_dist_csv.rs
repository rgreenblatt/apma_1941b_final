use crate::{
  dataset::{Contribution, Dataset},
  output_data::csv_writer,
  ItemType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize)]
struct ContributionCsvEntry {
  num: u32,
  count: usize,
  example_user: String,
  example_repo: String,
}

fn save_contribution_dist_impl<'a>(
  csv_path: &'a Path,
  contributions: impl IntoIterator<Item = &'a Contribution>,
  dataset: &Dataset,
) -> Result<()> {
  let mut contrib_count = HashMap::new();
  for &Contribution { num, idx } in contributions {
    contrib_count
      .entry(num)
      .or_insert((
        0,
        dataset.user_logins()[idx.user].clone(),
        dataset.user_logins()[idx.user].clone(),
      ))
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
) -> Result<()> {
  save_contribution_dist_impl(csv_path, dataset.contributions(), dataset)
}

pub fn save_contribution_dist_item(
  csv_path: &Path,
  item_type: ItemType,
  idx: usize,
  dataset: &Dataset,
) -> Result<()> {
  save_contribution_dist_impl(
    csv_path,
    dataset.contribution_idxs()[item_type][idx]
      .iter()
      .map(|&contrib_idx| &dataset.contributions()[contrib_idx]),
    dataset,
  )
}
