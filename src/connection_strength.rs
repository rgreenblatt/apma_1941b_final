use crate::{dataset::Dataset, edge_vec::EdgeVec, ItemType, UserRepoPair};
use fnv::FnvHashMap as Map;
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

pub struct ExpectationAccelerator<'a, T: ConnectionStrength> {
  cached_items: EdgeVec<(i32, f64)>,
  overall_counts: Vec<i32>,
  dataset: &'a Dataset,
  item_type: ItemType,
  _phantom: PhantomData<T>,
}

impl<'a, T: ConnectionStrength> ExpectationAccelerator<'a, T> {
  pub fn new(item_type: ItemType, dataset: &'a Dataset) -> Self {
    let (cached_items, overall_counts) = dataset.contribution_idxs()[item_type]
      .iter()
      .enumerate()
      .map(|(idx, contribs)| {
        let mut overall_count = 0;
        let mut totals = Map::default();
        for &contrib_idx in contribs {
          let contrib = dataset.contributions()[contrib_idx];

          debug_assert_eq!(contrib.idx[item_type], idx);

          let other_idx = contrib.idx[item_type.other()];

          let other_contrib_idxs =
            &dataset.contribution_idxs()[item_type.other()][other_idx];

          debug_assert_ne!(other_contrib_idxs.len(), 0);

          if other_contrib_idxs.len() == 1 {
            // to avoid nan value
            continue;
          }

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

          let len_other_than_us = (other_contrib_idxs.len() - 1) as i32;

          overall_count += len_other_than_us;

          *totals.entry(len_other_than_us).or_insert(0.) +=
            total_operation / len_other_than_us as f64;
        }

        (totals, overall_count)
      })
      .unzip();

    Self {
      cached_items,
      overall_counts,
      item_type,
      dataset,
      _phantom: PhantomData {},
    }
  }

  pub fn expectation(&self, items_idxs: [usize; 2]) -> f64 {
    let total_degree = self.dataset.contributions().len() as f64;
    items_idxs
      .iter()
      .zip(items_idxs.iter().rev())
      .map(|(&idx, &other_idx)| {
        let other_degree = self.dataset.contribution_idxs()[self.item_type]
          [other_idx]
          .len() as f64;
        let p = other_degree / total_degree;

        self.cached_items[idx]
          .iter()
          .map(|&(pow, mean_op)| (1. - (1. - p).powi(pow)) * mean_op)
          .sum::<f64>()
          / (1. - (1. - p).powi(self.overall_counts[idx]))
      })
      .sum::<f64>()
      / 2.
  }
}

pub trait ConnectionStrength: Clone + Copy + fmt::Debug + Sync + Send {
  type Value: ConnectionStrengthValue;

  fn strength(
    &self,
    _item_type: ItemType,
    contrib_idxs: &[[usize; 2]],
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
  pub accelerators: &'a UserRepoPair<ExpectationAccelerator<'a, T>>,
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
    dataset: &Dataset,
  ) -> Self::Value {
    let strength = self.inner.strength(item_type, contrib_idxs, dataset);

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

    let expected = self.accelerators[item_type].expectation([l_idx, r_idx]);

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

#[test]
pub fn basic_expectation() {
  use super::*;
  use crate::traversal::test::{contrib_num, repos, users};

  let large_repo: Vec<_> = (0..10).map(|i| contrib_num(i, 0, 10 + i)).collect();
  let other_large_repo: Vec<_> =
    (5..9).map(|i| contrib_num(i, 2, 10 + i)).collect();

  let contributions = vec![
    contrib_num(4, 1, 5),
    contrib_num(4, 2, 10),
    contrib_num(4, 3, 10),
    contrib_num(11, 3, 10),
  ];

  let dataset = Dataset::new(
    users(11),
    repos(4),
    large_repo
      .into_iter()
      .chain(other_large_repo)
      .chain(contributions),
    false,
  );

  let accel =
    ExpectationAccelerator::<NumCommonNodes>::new(ItemType::Repo, &dataset);

  const EPS: f64 = 1e-10;

  let total_degree = dataset.contributions().len() as f64;

  let p_1 = 1. / total_degree;

  let v = (1.
    + (4. * (1. - (1. - p_1)) + (1. - (1. - p_1).powi(3)))
      / (1. - (1. - p_1).powi(7)))
    / 2.;

  assert!((accel.expectation([1, 2]) - v).abs() < EPS);
  assert!((accel.expectation([2, 1]) - v).abs() < EPS);
  assert!((accel.expectation([1, 3]) - 1.).abs() < EPS);
  assert!((accel.expectation([3, 1]) - 1.).abs() < EPS);

  let p_2 = 5. / total_degree;
  let p_0 = 10. / total_degree;

  let v = ((4. * (1. - (1. - p_0)) + (1. - (1. - p_0).powi(3)))
    / (1. - (1. - p_0).powi(7))
    + (4. * (1. - (1. - p_2)) + (1. - (1. - p_2).powi(3)))
      / (1. - (1. - p_2).powi(7)))
    / 2.;

  assert!((accel.expectation([0, 2]) - v).abs() < EPS);

  let accel =
    ExpectationAccelerator::<MinNumEvents>::new(ItemType::Repo, &dataset);

  let v = (5.
    + ((1. - (1. - p_1)) * 15.
      + (1. - (1. - p_1)) * 16.
      + (1. - (1. - p_1)) * 17.
      + (1. - (1. - p_1)) * 18.
      + (1. - (1. - p_1).powi(3)) * (10. + 10. + 5.) / 3.)
      / (1. - (1. - p_1).powi(7)))
    / 2.;

  assert!((accel.expectation([1, 2]) - v).abs() < EPS);
  assert!((accel.expectation([2, 1]) - v).abs() < EPS);
  let v = (5. + (10. + 10. + 5.) / 3.) / 2.;
  assert!((accel.expectation([1, 3]) - v).abs() < EPS);
  assert!((accel.expectation([3, 1]) - v).abs() < EPS);
}
