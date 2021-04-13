use crate::{dataset::Dataset, ItemType};
use fnv::FnvHashSet as Set;
use itertools::Itertools;
use ordered_float::NotNan;
use std::{fmt, hash::Hash, iter, marker::PhantomData, ops, str::FromStr};

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

  fn to_float(self) -> f64;

  // obtain a more discrete approprimation (for use in a hash map/saving to disk)
  fn bin(self) -> Self;

  fn to_serializable(self) -> Self::S;
}

pub fn bin_float(v: f64) -> NotNan<f64> {
  let place = NotNan::new(2f64.powi(8)).unwrap();
  NotNan::new((NotNan::new(v).unwrap() * place).round()).unwrap() / place
}

impl ConnectionStrengthValue for NotNan<f64> {
  type S = f64;

  fn from_float(v: f64) -> anyhow::Result<Self> {
    Ok(NotNan::new(v)?)
  }

  fn to_float(self) -> f64 {
    self.into_inner()
  }

  fn bin(self) -> Self {
    bin_float(self.into_inner())
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

  fn to_float(self) -> f64 {
    self as f64
  }

  fn bin(self) -> Self {
    self
  }

  fn to_serializable(self) -> Self::S {
    self
  }
}

fn other_contribution_idxs_iter(
  item_type: ItemType,
  idx: usize,
  dataset: &Dataset,
) -> impl Iterator<Item = (usize, &[usize])> + '_ {
  let contribs = &dataset.contribution_idxs()[item_type][idx];

  contribs.iter().map(move |&contrib_idx| {
    let &other_idx =
      &dataset.contributions()[contrib_idx].idx[item_type.other()];
    (
      contrib_idx,
      &dataset.contribution_idxs()[item_type.other()][other_idx],
    )
  })
}

fn probs(item_type: ItemType, idx: usize, dataset: &Dataset) -> Vec<f64> {
  let total_degree = dataset.contributions().len() as f64;

  let contribs = &dataset.contribution_idxs()[item_type][idx];
  let degree = contribs.len() as f64;

  let p_connect_per = degree / total_degree;

  let mut probs: Vec<_> = other_contribution_idxs_iter(item_type, idx, dataset)
    .map(|(_, other_contrib_idxs)| {
      p_connect_per * (other_contrib_idxs.len() - 1) as f64
    })
    .collect();

  let denom = 1. - probs.iter().map(|&p| 1. - p).product::<f64>();

  for p in &mut probs {
    *p /= denom;
  }

  probs
}

pub trait ConnectionStrength: Clone + Copy + fmt::Debug + Default {
  type Value: ConnectionStrengthValue;

  fn strength(
    _item_type: ItemType,
    contrib_idxs: [usize; 2],
    dataset: &Dataset,
  ) -> Self::Value {
    let (l, r) = contrib_idxs
      .iter()
      .map(|&idx| dataset.contributions()[idx].num)
      .next_tuple()
      .unwrap();
    Self::operation([l, r])
  }

  fn expected(
    item_type: ItemType,
    items_idxs: [usize; 2],
    dataset: &Dataset,
  ) -> f64 {
    let mut avoid = Set::default();
    let expected: f64 = items_idxs
      .iter()
      .enumerate()
      .map(|(i, &idx)| -> f64 {
        let first = i == 0;
        let probs = probs(item_type, idx, dataset);
        other_contribution_idxs_iter(item_type, idx, dataset)
          .zip(probs)
          .map(|((contrib_idx, other_contrib_idxs), prob)| {
            debug_assert_ne!(other_contrib_idxs.len(), 0);

            let contrib = dataset.contributions()[contrib_idx];

            let other_idx = contrib.idx[item_type.other()];
            debug_assert_eq!(contrib.idx[item_type], idx);

            if first {
              let out = avoid.insert(other_idx);
              debug_assert!(out);
            } else {
              if avoid.contains(&other_idx) {
                return 0.;
              }
            }

            if other_contrib_idxs.len() == 1 {
              return 0.;
            }
            let len_other_than_us = (other_contrib_idxs.len() - 1) as f64;
            let total_operation = other_contrib_idxs
              .iter()
              .map(|&other_contrib_idx| {
                debug_assert_eq!(
                  contrib.idx[item_type.other()],
                  dataset.contributions()[other_contrib_idx].idx
                    [item_type.other()]
                );

                other_contrib_idx
              })
              .filter(|&other_contrib_idx| {
                // avoid iterating on "our" contribution edge
                contrib_idx != other_contrib_idx
              })
              .map(|other_contrib_idx| {
                let other_contrib = dataset.contributions()[other_contrib_idx];

                Self::operation([contrib.num, other_contrib.num]).to_float()
              })
              .sum::<f64>();

            let mean_operation = total_operation / len_other_than_us;

            mean_operation * prob
          })
          .sum()
      })
      .sum();

    expected
  }

  fn operation(nums: [u32; 2]) -> Self::Value;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NumCommonNodes;

impl ConnectionStrength for NumCommonNodes {
  type Value = usize;

  fn operation(_nums: [u32; 2]) -> Self::Value {
    1
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MinNumEvents;

impl ConnectionStrength for MinNumEvents {
  type Value = usize;

  fn operation(nums: [u32; 2]) -> Self::Value {
    *nums.iter().min().unwrap() as usize
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TotalNumEvents;

impl ConnectionStrength for TotalNumEvents {
  type Value = usize;

  fn operation(nums: [u32; 2]) -> Self::Value {
    nums.iter().sum::<u32>() as usize
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct GeometricMeanEvents;

impl ConnectionStrength for GeometricMeanEvents {
  type Value = NotNan<f64>;

  fn operation(nums: [u32; 2]) -> Self::Value {
    NotNan::new(nums.iter().map(|&n| n as f64).product::<f64>().sqrt()).unwrap()
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Normalized<T: ConnectionStrength> {
  phantom: PhantomData<T>,
}

impl<T: ConnectionStrength> fmt::Debug for Normalized<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "normalized_{:?}", T::default())
  }
}

impl<T: ConnectionStrength> ConnectionStrength for Normalized<T> {
  type Value = NotNan<f64>;

  fn strength(
    item_type: ItemType,
    contrib_idxs: [usize; 2],
    dataset: &Dataset,
  ) -> Self::Value {
    let strength = T::strength(item_type, contrib_idxs, dataset);
    let (l_idx, r_idx) = contrib_idxs
      .iter()
      .map(|&idx| dataset.contributions()[idx].idx[item_type])
      .collect_tuple()
      .unwrap();
    let expected = T::expected(item_type, [l_idx, r_idx], dataset);

    NotNan::new(strength.to_float() / expected).unwrap()
  }

  fn expected(
    _item_type: ItemType,
    _items_idxs: [usize; 2],
    _dataset: &Dataset,
  ) -> f64 {
    1.
  }

  fn operation(_nums: [u32; 2]) -> Self::Value {
    unreachable!();
  }
}

/// Value is if its normalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStrengthTypes {
  NumCommonNodes(bool),
  MinNumEvents(bool),
  TotalNumEvents(bool),
  GeometricMeanEvents(bool),
}

impl FromStr for ConnectionStrengthTypes {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let norm_s = "-normalized";
    let c = |rest: &[_]| match rest {
      [] => Ok(false),
      [""] => Ok(true),
      _ => Err(format!("unexpected suffix after '{}', got '{}'", norm_s, s)),
    };
    let strs: Vec<_> = s.split(norm_s).collect();
    let out = match &strs[..] {
      ["num-common-nodes", ref rest @ ..] => Self::NumCommonNodes(c(rest)?),
      ["min-num-events", ref rest @ ..] => Self::MinNumEvents(c(rest)?),
      ["total-num-events", ref rest @ ..] => Self::TotalNumEvents(c(rest)?),
      ["geometric-mean-events", ref rest @ ..] => {
        Self::GeometricMeanEvents(c(rest)?)
      }
      _ => {
        return Err(format!("Unrecognized connnection strength type: {}", s))
      }
    };

    Ok(out)
  }
}
