use crate::{
  dataset::{Contribution, DatasetWithInfo},
  UserRepoPair,
};
use fnv::{FnvHashMap as Map, FnvHashSet as Set};
use rand::distributions::Uniform;
use rand::prelude::*;

#[derive(Debug, Clone, Copy)]
struct DegreeItem {
  i: usize,
  j: usize,
  num: usize,
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

/// TODO: fix this being with info etc...
pub fn gen_graph(dataset_info: &mut DatasetWithInfo) {
  let mut counts = Map::default();
  let dataset = dataset_info.dataset();

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
  let min_per_bin = 100_000;

  let &(last, _) = counts.last().expect("empty input not allowed");

  let mut tally = 0;
  for (num, count) in counts {
    tally += count;
    if tally > min_per_bin {
      tally = 0;
      bins.push(num);
    }
  }

  let &actual_last = bins.last().expect("empty input not allowed");

  if actual_last != last {
    bins.push(last);
  }

  println!(
    "num bins is {} with first {:?} and last {:?}",
    bins.len(),
    bins.get(0),
    bins.last()
  );

  let mut rng = StdRng::seed_from_u64(812388383);

  degrees.repo.shuffle(&mut rng);

  let mut binned_degrees = vec![UserRepoPair::<Vec<_>>::default(); bins.len()];

  for (item_type, v) in degrees.iter_with_types() {
    for v in v {
      let i = partition_point(&bins, |bin| bin < &v.num);
      assert!(bins[i] >= v.num);
      let before = if i == 0 { 0 } else { bins[i - 1] };
      assert!(before <= v.num);

      binned_degrees[i][item_type].push(v);
    }
  }

  // TODO: fix this!!!
  let mut contribution_idxs = dataset.contribution_idxs().clone();
  contribution_idxs.as_mut().map_with(|idxs, item_type| {
    for i in 0..dataset.lens()[item_type] {
      idxs[i].iter_mut().for_each(|v| *v = std::usize::MAX);
    }
  });

  let mut contributions = Vec::new();

  let mut connected = Set::default();

  for binned in binned_degrees {
    for (repo, user) in binned.repo.iter().zip(binned.user.iter()) {
      if connected.contains(&(repo.i, user.i)) {
        // ignore multi edge (for now)
        continue;
      }

      let i = contributions.len();
      let num = Uniform::from(repo.num.min(user.num)..=repo.num.max(user.num))
        .sample(&mut rng);
      contribution_idxs.repo[repo.i][repo.j] = i;
      contribution_idxs.user[user.i][user.j] = i;
      contributions.push(Contribution {
        num,
        idx: UserRepoPair {
          user: user.i,
          repo: repo.i,
        },
      });
      connected.insert((repo.i, user.i));
    }
  }

  println!(
    "removed {} duplicate edges",
    dataset.contributions().len() - contributions.len()
  );

  dataset_info.set_edges(
    contributions,
    contribution_idxs.map(|v| {
      v.iter()
        .map(|idxs| idxs.iter().cloned().filter(|&v| v != std::usize::MAX))
        .collect()
    }),
  );
}
