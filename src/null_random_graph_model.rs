use crate::{
  dataset::{Contribution, Dataset},
  edge_vec::EdgeVec,
  ItemType, UserRepoPair,
};
use fnv::FnvHashMap as Map;
use rand::prelude::*;

#[derive(Debug, Clone, Copy)]
struct DegreeItem {
  i: usize,
  j: usize,
  num: u32,
}

fn partition_point<T, P>(slice: &[T], mut pred: P) -> usize
where
  P: FnMut(&T) -> bool,
{
  let mut left = 0;
  let mut right = slice.len();

  while left != right {
    let mid = left + (right - left) / 2;
    let value = &slice[mid];
    if pred(value) {
      left = mid + 1;
    } else {
      right = mid;
    }
  }

  left
}

pub fn gen_graph(dataset: &Dataset) {
  let alpha = 0.001;
  let beta = 0.4;
  // let 

  let mut counts = Map::default();

  let mut degrees: UserRepoPair<Vec<DegreeItem>> =
    UserRepoPair::<()>::default().map_with(|_, item_type| {
      dataset.contribution_idxs()[item_type]
        .iter()
        .enumerate()
        .flat_map(|(i, idxs)| {
          idxs
            .iter()
            .enumerate()
            .map(|(j, &idx)| {
              let num = dataset.contributions()[idx].num;

              *counts.entry(num).or_insert(0) += 1;

              DegreeItem { i, j, num }
            })
            .collect::<Vec<DegreeItem>>()
        })
        .collect()
    });

  let mut counts: Vec<_> = counts.into_iter().collect();
  counts.sort();
  let mut bins = Vec::new();
  let min_per_bin = 1000;

  let &(last, _) = counts.last().expect("empty input not allowed");

  let mut tally = 0;
  for (num, count) in counts {
    tally += count;
    if tally > min_per_bin {
      bins.push(num);
    }
  }

  let &actual_last = bins.last().expect("empty input not allowed");

  if actual_last != last {
    bins.push(last);
  }

  let mut rng = rand::thread_rng();

  degrees.repo.shuffle(&mut rng);

  let mut binned_degrees = vec![UserRepoPair::<Vec<_>>::default(); bins.len()];

  for (item_type, v) in degrees.iter_with_types() {
    for v in v {
      let i = partition_point(&bins, |bin| bin < &v.num) + 1;
      binned_degrees[i][item_type].push(v);
    }
  }

  let mut contribution_idxs = dataset.contribution_idxs().clone();
  contribution_idxs.as_mut().map_with(|idxs, item_type| {
    for i in 0..dataset.len(item_type) {
      idxs[i].iter_mut().for_each(|v| *v = std::usize::MAX);
    }
  });

  let mut contributions = Vec::new();

  for binned in binned_degrees {
    for (repo, user) in binned.repo.iter().zip(binned.user.iter()) {
      let i = contributions.len();
      let num = (repo.num + user.num) / 2;
      contribution_idxs.repo[repo.i][repo.j] = i;
      contribution_idxs.user[user.i][user.j] = i;
      contributions.push(Contribution {
        num,
        idx: UserRepoPair {
          user: user.i,
          repo: repo.i,
        },
      });
    }
  }

  assert_eq!(contributions.len(), dataset.contributions().len());

  for idxs in contribution_idxs.as_ref() {
    for idxs in idxs.iter() {
      for &v in idxs {
        assert_ne!(v, std::usize::MAX);
      }
    }
  }

  dataset.set_edges(contributions, contribution_idxs);
}
