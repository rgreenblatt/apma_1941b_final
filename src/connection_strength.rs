use crate::{dataset::Dataset, ItemType};
use ordered_float::NotNan;
use std::{fmt, hash::Hash, iter, ops, str::FromStr};

pub trait ConnectionStrengthValue:
  PartialOrd
  + Default
  + Clone
  + ops::AddAssign
  + fmt::Display
  + Hash
  + Ord
  + iter::Sum
{
  type S: serde::Serialize;

  fn from_float(v: f64) -> anyhow::Result<Self>;

  // obtain a more discrete approprimation (for use in a hash map/saving to disk)
  fn bin(self) -> Self;

  fn to_serializable(self) -> Self::S;
}

impl ConnectionStrengthValue for NotNan<f64> {
  type S = f64;

  fn from_float(v: f64) -> anyhow::Result<Self> {
    Ok(NotNan::new(v)?)
  }

  fn bin(self) -> Self {
    let place = NotNan::new(2f64.powi(8)).unwrap();
    NotNan::new((self * place).round()).unwrap() / place
  }

  fn to_serializable(self) -> Self::S {
    self.into_inner()
  }
}

impl ConnectionStrengthValue for usize {
  type S = Self;

  fn from_float(v: f64) -> anyhow::Result<Self> {
    if v.fract() != 0. {
      return Err(anyhow::anyhow!(
        "fractional part of number is non-zero in convert to integer"
      ));
    }
    return Ok(v.round() as usize);
  }

  fn bin(self) -> Self {
    self
  }

  fn to_serializable(self) -> Self::S {
    self
  }
}

pub trait ConnectionStrength: Clone + Copy + fmt::Debug + Default {
  type Value: ConnectionStrengthValue;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value;

  fn normalize(
    strength: Self::Value,
    _item_type: ItemType,
    _start_idx: usize,
    _end_idx: usize,
    _dataset: &Dataset,
  ) -> Self::Value {
    strength
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NumCommonNodes;

impl ConnectionStrength for NumCommonNodes {
  type Value = usize;

  fn strength(
    _first_contrib_idx: usize,
    _second_contrib_idx: usize,
    _dataset: &Dataset,
  ) -> Self::Value {
    1
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MinNumEvents;

impl ConnectionStrength for MinNumEvents {
  type Value = usize;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    dataset.contributions()[first_contrib_idx]
      .num
      .min(dataset.contributions()[second_contrib_idx].num) as usize
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NormalizedMinNumEvents;

impl ConnectionStrength for NormalizedMinNumEvents {
  type Value = NotNan<f64>;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    let s =
      MinNumEvents::strength(first_contrib_idx, second_contrib_idx, dataset);
    NotNan::new(s as f64).unwrap()
  }

  fn normalize(
    strength: Self::Value,
    item_type: ItemType,
    start_idx: usize,
    end_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    let denom: f64 = [start_idx, end_idx]
      .iter()
      .map(|&idx| {
        dataset.contribution_idxs()[item_type][idx]
          .iter()
          .map(|&contrib_idx| dataset.contributions()[contrib_idx].num as f64)
          .sum::<f64>()
      })
      .fold(f64::INFINITY, |l, r| l.min(r));
    strength / denom
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TotalNumEvents;

impl ConnectionStrength for TotalNumEvents {
  type Value = usize;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    (dataset.contributions()[first_contrib_idx].num
      + dataset.contributions()[second_contrib_idx].num) as usize
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NormalizedTotalNumEvents;

impl ConnectionStrength for NormalizedTotalNumEvents {
  type Value = NotNan<f64>;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    let s =
      TotalNumEvents::strength(first_contrib_idx, second_contrib_idx, dataset);
    NotNan::new(s as f64).unwrap()
  }

  fn normalize(
    strength: Self::Value,
    item_type: ItemType,
    start_idx: usize,
    end_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    let denom: f64 = [start_idx, end_idx]
      .iter()
      .map(|&idx| {
        dataset.contribution_idxs()[item_type][idx]
          .iter()
          .map(|&contrib_idx| dataset.contributions()[contrib_idx].num as f64)
          .sum::<f64>()
      })
      .sum();
    strength / denom
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct GeometricMeanEvents;

impl ConnectionStrength for GeometricMeanEvents {
  type Value = NotNan<f64>;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    NotNan::new(
      (dataset.contributions()[first_contrib_idx].num as f64
        * dataset.contributions()[second_contrib_idx].num as f64)
        .sqrt(),
    )
    .unwrap()
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NormalizedGeometricMeanEvents;

impl ConnectionStrength for NormalizedGeometricMeanEvents {
  type Value = NotNan<f64>;

  fn strength(
    first_contrib_idx: usize,
    second_contrib_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    GeometricMeanEvents::strength(
      first_contrib_idx,
      second_contrib_idx,
      dataset,
    )
  }

  fn normalize(
    strength: Self::Value,
    item_type: ItemType,
    start_idx: usize,
    end_idx: usize,
    dataset: &Dataset,
  ) -> Self::Value {
    let denom: f64 = [start_idx, end_idx]
      .iter()
      .map(|&idx| {
        dataset.contribution_idxs()[item_type][idx]
          .iter()
          .map(|&contrib_idx| {
            let v = dataset.contributions()[contrib_idx].num as f64;
            NotNan::new(v).unwrap()
          })
          .sum::<NotNan<f64>>()
      })
      .product::<NotNan<f64>>()
      .sqrt();
    strength / denom
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStrengthTypes {
  NumCommonNodes,
  MinNumEvents,
  NormalizedMinNumEvents,
  TotalNumEvents,
  NormalizedTotalNumEvents,
  GeometricMeanEvents,
  NormalizedGeometricMeanEvents,
}

impl FromStr for ConnectionStrengthTypes {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let out = match s {
      "num-common-nodes" => Self::NumCommonNodes,
      "min-num-events" => Self::MinNumEvents,
      "normalized-min-num-events" => Self::NormalizedMinNumEvents,
      "total-num-events" => Self::TotalNumEvents,
      "normalized-total-num-events" => Self::NormalizedTotalNumEvents,
      "geometric-mean-events" => Self::GeometricMeanEvents,
      "normalized-geometric-mean-events" => Self::NormalizedGeometricMeanEvents,
      _ => {
        return Err(format!("Unrecognized connnection strength type: {}", s))
      }
    };

    Ok(out)
  }
}
