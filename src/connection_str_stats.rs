use crate::{
  connection_strength::{
    bin_float, bin_float_place, ConnectionStrength, ConnectionStrengthValue,
    ExpectationAccelerator,
  },
  dataset::Dataset,
  degree_dist_csv::save_sort_items,
  projected_graph::transitive_edge_compute,
  ItemType,
};
use anyhow::Result;
use fnv::FnvHashMap as Map;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};

#[derive(Deserialize, Serialize)]
pub struct DegreeCsvEntry {
  pub degree: usize,
  pub count: usize,
}

#[derive(Serialize)]
pub struct ConnectionStrengthCsvEntry<S: Serialize> {
  pub strength: S,
  pub count: usize,
}

#[derive(Serialize)]
pub struct ConnectionStrengthExpectedCsvEntry {
  pub strength: f64,
  pub expected: f64,
  pub count: usize,
}

struct State<T: ConnectionStrength> {
  degree_counts: Map<usize, usize>,
  strength_counts: Map<T::Value, usize>,
  strength_normalized_counts: Map<NotNan<f64>, usize>,
  expected_counts: Map<NotNan<f64>, usize>,
  strength_expected_counts: Map<(NotNan<f64>, NotNan<f64>), usize>,
  total_expected: f64,
  total_strength: f64,
  total_sqr_strength: f64,
  total_sqr_expected: f64,
  total_strength_expected: f64,
  total_strength_normalized: f64,
  total_sqr_strength_normalized: f64,
  count: usize,
}

pub fn save_connection_str_stats<
  T: ConnectionStrength,
  V: ConnectionStrength,
>(
  output_dir: &Path,
  item_type: ItemType,
  connection_strength: &T,
  accelerator: &ExpectationAccelerator<V>,
  dataset: &Dataset,
) -> Result<()> {
  let state = State::<T> {
    degree_counts: Default::default(),
    strength_counts: Default::default(),
    strength_normalized_counts: Default::default(),
    expected_counts: Default::default(),
    strength_expected_counts: Default::default(),
    total_expected: Default::default(),
    total_strength: Default::default(),
    total_sqr_strength: Default::default(),
    total_sqr_expected: Default::default(),
    total_strength_expected: Default::default(),
    total_strength_normalized: Default::default(),
    total_sqr_strength_normalized: Default::default(),
    count: Default::default(),
  };
  let state = Mutex::new(state);

  let f = |start_idx: usize,
           mut edge_map: Map<_, (Vec<usize>, Vec<[usize; 2]>)>| {
    let values: Vec<_> = edge_map
      .drain()
      .map(|(end_idx, (common_other_idxs, contrib_idxs))| {
        let end_idx: usize = end_idx;

        let strength = connection_strength.strength(
          item_type,
          &contrib_idxs,
          &common_other_idxs,
          dataset,
        );

        let expected =
          accelerator.expectation([start_idx, end_idx], &common_other_idxs);

        (strength, expected)
      })
      .collect();

    let mut state = state.lock().unwrap();

    *state.degree_counts.entry(values.len()).or_insert(0) += values.len();
    state.count += values.len();
    for (strength, expected) in values {
      *state
        .strength_counts
        .entry(strength.clone().bin())
        .or_insert(0) += 1;

      let strength = strength.clone().to_float();

      let strength_normalized = strength / expected;
      *state
        .strength_normalized_counts
        .entry(bin_float(strength_normalized))
        .or_insert(0) += 1;
      *state
        .expected_counts
        .entry(bin_float(expected))
        .or_insert(0) += 1;
      *state
        .strength_expected_counts
        .entry((bin_float_place(strength, 1), bin_float_place(expected, 1)))
        .or_insert(0) += 1;

      state.total_expected += expected;
      state.total_strength += strength;
      state.total_sqr_strength += strength.powi(2);
      state.total_sqr_expected += expected.powi(2);
      state.total_strength_expected += strength * expected;
      state.total_strength_normalized += strength_normalized;
      state.total_sqr_strength_normalized += strength_normalized.powi(2);
    }
  };

  transitive_edge_compute(item_type, dataset, f);

  let State {
    degree_counts,
    strength_counts,
    strength_normalized_counts,
    expected_counts,
    strength_expected_counts,
    total_expected,
    total_strength,
    total_sqr_strength,
    total_sqr_expected,
    total_strength_expected,
    total_strength_normalized,
    total_sqr_strength_normalized,
    count,
  } = state.into_inner().unwrap();

  let total_contributions = dataset
    .contributions()
    .iter()
    .map(|v| v.num as f64)
    .sum::<f64>();
  let total_degree = dataset.contributions().len() as f64;

  let mean_expected = total_expected / count as f64;
  let mean_strength = total_strength / count as f64;
  let mean_sqr_strength = total_sqr_strength / count as f64;
  let mean_sqr_expected = total_sqr_expected / count as f64;
  let mean_strength_expected = total_strength_expected / count as f64;
  let mean_strength_normalized = total_strength_normalized / count as f64;
  let mean_sqr_strength_normalized =
    total_sqr_strength_normalized / count as f64;

  println!("total contributions is {}", total_contributions);
  println!("total degree is {}", total_degree);
  println!(
    "total strength over total degree is {}",
    total_strength / total_degree
  );
  println!(
    "total strength over total contributions is {}",
    total_strength / total_contributions
  );
  println!("mean expected is {}", mean_expected);
  println!("mean strength is {}", mean_strength);
  println!("mean sqr strength is {}", mean_sqr_strength);
  println!("mean sqr expected is {}", mean_sqr_expected);
  println!("mean strength expected is {}", mean_strength_expected);
  println!(
    "correlation strength-expected is {}",
    (mean_strength_expected - mean_strength * mean_expected)
      / ((mean_sqr_strength - mean_strength.powi(2)).sqrt()
        * (mean_sqr_expected - mean_expected.powi(2)).sqrt())
  );
  println!("mean strength expected is {}", mean_strength_expected);
  println!(
    "strength variance is {}",
    mean_sqr_strength - mean_strength.powi(2)
  );
  println!("mean normalized strength is {}", mean_strength_normalized);
  println!(
    "normalized strength variance is {}",
    mean_sqr_strength_normalized - mean_strength_normalized.powi(2)
  );
  println!(
    "normalized strength mean sqr is {}",
    mean_sqr_strength_normalized
  );

  save_sort_items(
    &output_dir.join("degrees.csv"),
    degree_counts,
    |(degree, _)| degree.clone(),
    |(degree, count)| DegreeCsvEntry { degree, count },
  )?;

  save_sort_items(
    &output_dir.join("strengths.csv"),
    strength_counts,
    |(strength, _): &(T::Value, _)| strength.clone(),
    |(strength, count)| ConnectionStrengthCsvEntry {
      strength: strength.to_serializable(),
      count,
    },
  )?;

  save_sort_items(
    &output_dir.join("strengths_normalized.csv"),
    strength_normalized_counts,
    |(strength, _)| strength.clone(),
    |(strength, count)| ConnectionStrengthCsvEntry {
      strength: strength.to_serializable(),
      count,
    },
  )?;

  save_sort_items(
    &output_dir.join("expected.csv"),
    expected_counts,
    |(expected, _)| expected.clone(),
    |(expected, count)| ConnectionStrengthCsvEntry {
      strength: expected.into_inner(),
      count,
    },
  )?;

  save_sort_items(
    &output_dir.join("strength_expected.csv"),
    strength_expected_counts,
    |(expected, _)| expected.clone(),
    |((strength, expected), count)| ConnectionStrengthExpectedCsvEntry {
      strength: strength.into_inner(),
      expected: expected.into_inner(),
      count,
    },
  )?;

  Ok(())
}
