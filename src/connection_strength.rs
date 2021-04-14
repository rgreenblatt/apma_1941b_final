use crate::{dataset::Dataset, progress_bar::get_bar, ItemType, UserRepoPair};
use fnv::FnvHashMap as Map;
use indicatif::ProgressIterator;
use itertools::Itertools;
use ordered_float::NotNan;
use std::{fmt, hash::Hash, iter, marker::PhantomData, ops, str::FromStr};

pub trait ConnectionStrengthValue:
  PartialOrd
  + Sync
  + Send
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

pub fn bin_float_place(v: f64, place: usize) -> NotNan<f64> {
  let place = NotNan::new(place as f64).unwrap();
  NotNan::new((NotNan::new(v).unwrap() * place).round()).unwrap() / place
}

pub fn bin_float(v: f64) -> NotNan<f64> {
  bin_float_place(v, 4)
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

pub struct ExpectationAccelerator<T: ConnectionStrength> {
  maps: Vec<Map<usize, f64>>,
  totals: Vec<f64>,
  _phantom: PhantomData<T>,
}
impl<T: ConnectionStrength> ExpectationAccelerator<T> {
  pub fn new(item_type: ItemType, dataset: &Dataset) -> Self {
    let base_probs: Vec<_> = dataset.contribution_idxs()[item_type.other()]
      .iter()
      .map(|v| v.len().saturating_sub(1) as f64)
      .collect();

    let total_degree = dataset.contributions().len() as f64;

    let bar = get_bar(Some(dataset.len(item_type) as u64), 100_000);

    let (maps, totals) = dataset.contribution_idxs()[item_type]
      .iter()
      .enumerate()
      .progress_with(bar)
      .map(|(idx, contribs)| {
        let degree = contribs.len() as f64;

        let p_connect_per = degree / total_degree;

        let mut probs: Vec<_> = contribs
          .iter()
          .map(|&contrib_idx| {
            let other_idx =
              dataset.contributions()[contrib_idx].idx[item_type.other()];
            base_probs[other_idx] * p_connect_per
          })
          .collect();

        let denom = 1. - probs.iter().map(|&p| 1. - p).product::<f64>();

        for p in &mut probs {
          *p /= denom;
        }

        let out: Map<_, _> = contribs
          .iter()
          .zip(probs)
          .map(|(&contrib_idx, prob)| {
            let contrib = dataset.contributions()[contrib_idx];

            debug_assert_eq!(contrib.idx[item_type], idx);

            let other_idx = contrib.idx[item_type.other()];

            let other_contrib_idxs =
              &dataset.contribution_idxs()[item_type.other()][other_idx];

            debug_assert_ne!(other_contrib_idxs.len(), 0);

            if other_contrib_idxs.len() == 1 {
              // to avoid nan value
              return (other_idx, 0.);
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

                T::operation([contrib.num, other_contrib.num]).to_float()
              })
              .sum::<f64>();

            let mean_operation = total_operation / len_other_than_us;

            (other_idx, mean_operation * prob)
          })
          .collect();

        let total = out.values().sum::<f64>();

        (out, total)
      })
      .unzip();

    Self {
      maps,
      totals,
      _phantom: PhantomData {},
    }
  }

  pub fn expectation(
    &self,
    items_idxs: [usize; 2],
    common_other_idxs: &[usize],
  ) -> f64 {
    let mut total = items_idxs.iter().map(|&idx| self.totals[idx]).sum::<f64>();

    for idx in common_other_idxs {
      total -= self.maps[items_idxs[1]].get(idx).unwrap();
    }

    debug_assert!(total > 0.);

    total
  }
}

pub trait ConnectionStrength: Clone + Copy + fmt::Debug + Sync + Send {
  type Value: ConnectionStrengthValue;

  fn strength(
    &self,
    _item_type: ItemType,
    contrib_idxs: &[[usize; 2]],
    _common_other_idxs: &[usize],
    dataset: &Dataset,
  ) -> Self::Value {
    contrib_idxs
      .iter()
      .map(|contrib_idxs| {
        let (l, r) = contrib_idxs
          .iter()
          .map(|&idx| dataset.contributions()[idx].num)
          .next_tuple()
          .unwrap();
        Self::operation([l, r])
      })
      .sum()
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

#[derive(Clone, Copy)]
pub struct Normalized<'a, T: ConnectionStrength> {
  pub inner: T,
  pub accelerators: &'a UserRepoPair<ExpectationAccelerator<T>>,
}

impl<'a, T: ConnectionStrength> fmt::Debug for Normalized<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "normalized_{:?}", &self.inner)
  }
}

impl<'a, T: ConnectionStrength> ConnectionStrength for Normalized<'a, T> {
  type Value = NotNan<f64>;

  fn strength(
    &self,
    item_type: ItemType,
    contrib_idxs: &[[usize; 2]],
    common_other_idxs: &[usize],
    dataset: &Dataset,
  ) -> Self::Value {
    let strength =
      self
        .inner
        .strength(item_type, contrib_idxs, common_other_idxs, dataset);

    let get_items = |iter: &[usize; 2]| {
      iter
        .iter()
        .map(|&idx| dataset.contributions()[idx].idx[item_type])
        .collect_tuple()
        .unwrap()
    };

    let first = contrib_idxs.iter().next().unwrap();
    let (l_idx, r_idx) = get_items(first);

    debug_assert!(contrib_idxs
      .iter()
      .map(get_items)
      .all(|(l, r)| l == l_idx && r == r_idx));

    let expected = self.accelerators[item_type]
      .expectation([l_idx, r_idx], common_other_idxs);

    NotNan::new(strength.to_float() / expected).unwrap()
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
