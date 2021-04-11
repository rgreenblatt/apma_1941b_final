use crate::{dataset::Dataset, ItemType, UserRepoPair};
use std::{hash::Hash, iter};

/// construct using StartComponent
pub type Component = UserRepoPair<Vec<usize>>;

#[derive(Hash, Eq, PartialEq, Default, Debug, Clone)]
pub struct IdxDist {
  idxs_v: Vec<usize>,
  dists_v: Vec<usize>,
}

/// construct using StartComponent
pub type ComponentDists = UserRepoPair<IdxDist>;

impl IdxDist {
  pub fn idxs(&self) -> &[usize] {
    &self.idxs_v
  }

  pub fn dists(&self) -> &[usize] {
    &self.dists_v
  }

  /// dist then idx
  #[cfg(test)]
  fn as_pairs(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
    self
      .dists()
      .iter()
      .cloned()
      .zip(self.idxs().iter().cloned())
  }

  /// dist then idx
  #[cfg(test)]
  fn from_pairs(iter: impl IntoIterator<Item = (usize, usize)>) -> Self {
    let (dists_v, idxs_v) = iter.into_iter().unzip();
    Self { dists_v, idxs_v }
  }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct StartComponent {
  pub item_type: ItemType,
  pub idx: usize,
}

impl StartComponent {
  pub fn set_visited(self, visted: &mut Visited) {
    visted.as_mut()[self.item_type][self.idx] = true;
  }
}

impl From<StartComponent> for Component {
  fn from(start: StartComponent) -> Self {
    let mut out = Self::default();
    out[start.item_type].push(start.idx);
    out
  }
}

impl From<StartComponent> for ComponentDists {
  fn from(start: StartComponent) -> Self {
    let mut out = Self::default();
    out[start.item_type].add_items(0, iter::once(start.idx));
    out
  }
}

pub type Visited = UserRepoPair<Vec<bool>>;

pub fn default_visited(dataset: &Dataset) -> Visited {
  dataset.names().as_ref().map(|v| vec![false; v.len()])
}

pub fn traverse(
  component: &mut Component,
  visited: &mut UserRepoPair<Vec<bool>>,
  dataset: &Dataset,
  limit: Option<usize>,
  callback: impl FnMut(ItemType, usize),
) {
  traverse_gen(component, visited, dataset, limit, callback)
}

pub fn traverse_dist(
  component: &mut ComponentDists,
  visited: &mut UserRepoPair<Vec<bool>>,
  dataset: &Dataset,
  limit: Option<usize>,
  callback: impl FnMut(ItemType, usize),
) {
  // should be enforced by IdxDist
  #[cfg(debug_assertions)]
  for IdxDist { idxs_v, dists_v } in component.as_ref() {
    debug_assert_eq!(idxs_v.len(), dists_v.len());
  }
  traverse_gen(component, visited, dataset, limit, callback)
}

fn traverse_gen(
  component: &mut UserRepoPair<impl ComponentAccess>,
  visited: &mut UserRepoPair<Vec<bool>>,
  dataset: &Dataset,
  limit: Option<usize>,
  mut callback: impl FnMut(ItemType, usize),
) {
  // one item
  assert!(component.user.idxs().len() + component.repo.idxs().len() <= 1);
  // all items are visited
  assert!(component
    .as_ref()
    .iter_with_types()
    .flat_map(|(item_type, idxs)| idxs
      .idxs()
      .iter()
      .map(move |&i| (item_type, i)))
    .all(|(item_type, i)| visited[item_type][i]));

  let mut start = UserRepoPair { user: 0, repo: 0 };
  let mut dist = 0;

  let limit = limit.unwrap_or(std::usize::MAX);

  let callback = &mut callback;

  'outer: loop {
    // NOTE: order matters
    for &item_type in &[ItemType::Repo, ItemType::User] {
      // this condition is for the case where we start with just users
      if start[item_type] != component[item_type].idxs().len() {
        dist += 1;
      }

      if dist > limit {
        break 'outer;
      }

      traversal_step(
        item_type, dist, &mut start, visited, component, dataset, callback,
      );
    }

    if component.repo.idxs()[start.repo..].is_empty() {
      break;
    }
  }
}

trait ComponentAccess: Hash + Eq {
  fn idxs(&self) -> &[usize];
  fn add_items(&mut self, dist: usize, idxs: impl IntoIterator<Item = usize>);
}

impl ComponentAccess for Vec<usize> {
  fn idxs(&self) -> &[usize] {
    self
  }

  fn add_items(&mut self, _dist: usize, idxs: impl IntoIterator<Item = usize>) {
    self.extend(idxs);
  }
}

impl ComponentAccess for IdxDist {
  fn idxs(&self) -> &[usize] {
    self.idxs()
  }

  fn add_items(&mut self, dist: usize, idxs: impl IntoIterator<Item = usize>) {
    self.idxs_v.extend(idxs);
    self.dists_v.resize(self.idxs_v.len(), dist);
  }
}

fn traversal_step(
  item_type: ItemType,
  dist: usize,
  start: &mut UserRepoPair<usize>,
  visited: &mut UserRepoPair<Vec<bool>>,
  component: &mut UserRepoPair<impl ComponentAccess>,
  dataset: &Dataset,
  callback: &mut impl FnMut(ItemType, usize),
) {
  let start = &mut start[item_type];
  let (idxs, other_idxs) = component.as_mut().as_tup_with_first(item_type);
  for &idx in &idxs.idxs()[*start..] {
    let new_idxs = dataset.contribution_idxs()[item_type][idx]
      .iter()
      .filter_map(|&i| {
        let other_idx = dataset.contributions()[i].idx[item_type.other()];
        let other_visited = &mut visited[item_type.other()][other_idx];
        if *other_visited {
          None
        } else {
          *other_visited = true;
          Some(other_idx)
        }
      });
    other_idxs.add_items(dist, new_idxs);
    callback(item_type, idx);
  }
  *start = component[item_type].idxs().len();
}

#[cfg(test)]
pub mod test {
  use super::*;
  use crate::{dataset::ContributionInput, github_api, Repo, User};
  use proptest::prelude::*;
  use std::iter;

  trait ComponentSort: ComponentAccess {
    fn sort_component(&mut self);
  }

  impl ComponentSort for Vec<usize> {
    fn sort_component(&mut self) {
      self.sort();
    }
  }

  impl ComponentSort for IdxDist {
    fn sort_component(&mut self) {
      let mut together: Vec<_> = self.as_pairs().collect();
      together.sort();
      *self = Self::from_pairs(together);
    }
  }

  fn sort_component(component: &mut UserRepoPair<impl ComponentSort>) {
    component.repo.sort_component();
    component.user.sort_component();
  }

  fn gen_test_no_limit_no_expected(
    dataset: &Dataset,
    actual: &mut UserRepoPair<impl ComponentSort>,
  ) -> Result<(), TestCaseError> {
    for (item_type, idxs) in actual.as_ref().iter_with_types() {
      for &idx in idxs.idxs() {
        for &contrib_idx in dataset.contribution_idxs()[item_type][idx].iter() {
          proptest::prop_assert!(actual[item_type.other()].idxs().contains(
            &dataset.contributions()[contrib_idx].idx[item_type.other()]
          ));
        }
      }
    }
    Ok(())
  }

  fn gen_test_no_limit(
    start: StartComponent,
    dataset: &Dataset,
    mut expected_component: Component,
  ) {
    let mut visited = default_visited(dataset);
    start.set_visited(&mut visited);
    let mut component = start.into();
    traverse(&mut component, &mut visited, dataset, None, |_, _| {});
    gen_test_no_limit_no_expected(dataset, &mut component).unwrap();
    sort_component(&mut component);
    sort_component(&mut expected_component);
    assert_eq!(component, expected_component);
  }

  fn gen_test_dists(
    start: StartComponent,
    limit: Option<usize>,
    dataset: &Dataset,
    mut expected_component: ComponentDists,
  ) {
    let mut visited = default_visited(dataset);
    start.set_visited(&mut visited);
    let mut component = start.into();
    traverse_dist(&mut component, &mut visited, dataset, limit, |_, _| {});
    sort_component(&mut component);
    sort_component(&mut expected_component);
    assert_eq!(component, expected_component);
  }

  pub fn users(n: github_api::ID) -> impl Iterator<Item = (User, String)> {
    (0..n).map(|github_id| (User { github_id }, "".to_owned()))
  }

  pub fn repos(n: github_api::ID) -> impl Iterator<Item = (Repo, String)> {
    (0..n).map(|github_id| (Repo { github_id }, "".to_owned()))
  }

  pub fn single_user_dataset() -> Dataset {
    Dataset::new(users(1), iter::empty(), iter::empty(), true)
  }

  #[test]
  fn single_user() {
    let dataset = single_user_dataset();

    let start = StartComponent {
      item_type: ItemType::User,
      idx: 0,
    };

    gen_test_no_limit(
      start,
      &dataset,
      Component {
        user: vec![0],
        repo: Vec::new(),
      },
    );

    for &limit in &[None, Some(1), Some(100)] {
      gen_test_dists(
        start,
        limit,
        &dataset,
        ComponentDists {
          user: IdxDist {
            idxs_v: vec![0],
            dists_v: vec![0],
          },
          repo: Default::default(),
        },
      );
    }
  }

  pub fn single_repo_dataset() -> Dataset {
    Dataset::new(iter::empty(), repos(1), iter::empty(), true)
  }

  #[test]
  fn single_repo() {
    let dataset = single_repo_dataset();
    let start = StartComponent {
      item_type: ItemType::Repo,
      idx: 0,
    };
    gen_test_no_limit(
      start,
      &dataset,
      Component {
        user: Vec::new(),
        repo: vec![0],
      },
    );

    for &limit in &[None, Some(0), Some(1), Some(100)] {
      gen_test_dists(
        start,
        limit,
        &dataset,
        ComponentDists {
          user: Default::default(),
          repo: IdxDist {
            idxs_v: vec![0],
            dists_v: vec![0],
          },
        },
      );
    }
  }

  pub fn contrib(
    user_github_id: github_api::ID,
    repo_github_id: github_api::ID,
  ) -> ContributionInput {
    ContributionInput {
      user: User {
        github_id: user_github_id,
      },
      repo: Repo {
        github_id: repo_github_id,
      },
      num: 1,
    }
  }

  pub fn small_disconnected_dataset(count: u32) -> Dataset {
    Dataset::new(
      users(count),
      repos(count),
      (0..count).into_iter().map(|i| contrib(i, i)),
      false,
    )
  }

  fn filter_limit(
    limit: Option<usize>,
    component: ComponentDists,
  ) -> ComponentDists {
    if let Some(limit) = limit {
      component.map(|comp| {
        IdxDist::from_pairs(comp.as_pairs().filter(|(dist, _)| dist <= &limit))
      })
    } else {
      component
    }
  }

  #[test]
  fn small_disconnected() {
    for &count in &[1, 2, 3, 8] {
      let dataset = small_disconnected_dataset(count);

      for idx in 0..count as usize {
        for &item_type in &[ItemType::User, ItemType::Repo] {
          let start = StartComponent { idx, item_type };
          gen_test_no_limit(
            start,
            &dataset,
            Component {
              user: vec![idx],
              repo: vec![idx],
            },
          );
          let (user_dist, repo_dist) = match item_type {
            ItemType::User => (0, 1),
            ItemType::Repo => (1, 0),
          };
          for &limit in &[None, Some(0), Some(1), Some(100)] {
            gen_test_dists(
              start,
              limit,
              &dataset,
              filter_limit(
                limit,
                ComponentDists {
                  user: IdxDist {
                    idxs_v: vec![idx],
                    dists_v: vec![user_dist],
                  },
                  repo: IdxDist {
                    idxs_v: vec![idx],
                    dists_v: vec![repo_dist],
                  },
                },
              ),
            );
          }
        }
      }
    }
  }

  pub fn fully_connected_dataset(count: u32) -> Dataset {
    Dataset::new(
      users(count),
      repos(count),
      (0..count)
        .into_iter()
        .map(|i| contrib(i, i))
        .chain((0..count - 1).into_iter().map(|i| contrib(i, i + 1))),
      false,
    )
  }

  #[test]
  fn fully_connected() {
    for &count in &[1, 2, 3, 8] {
      let dataset = fully_connected_dataset(count);

      for idx in 0..count as usize {
        for &item_type in &[ItemType::User, ItemType::Repo] {
          let start = StartComponent { idx, item_type };
          gen_test_no_limit(
            start,
            &dataset,
            Component {
              user: (0..count as usize).collect(),
              repo: (0..count as usize).collect(),
            },
          );
          let offsets = match item_type {
            ItemType::User => UserRepoPair { user: 0, repo: 1 },
            ItemType::Repo => UserRepoPair { user: -1, repo: 0 },
          };
          for limit in [None]
            .iter()
            .cloned()
            .chain((0..count as usize + 1).map(Some))
          {
            let dists = offsets.map(|offset: isize| {
              let idx = idx as isize;
              (0..count as isize)
                .map(|j| {
                  let diff = if j > idx { j - idx } else { idx - j };
                  let offset = if j > idx {
                    -offset
                  } else if j == idx {
                    offset.abs()
                  } else {
                    offset
                  };
                  (2 * diff + offset) as usize
                })
                .collect()
            });

            dbg!(count);
            dbg!(idx);
            dbg!(item_type);
            dbg!(limit);
            gen_test_dists(
              start,
              limit,
              &dataset,
              filter_limit(
                limit,
                ComponentDists {
                  user: IdxDist {
                    idxs_v: (0..count as usize).collect(),
                    dists_v: dists.user,
                  },
                  repo: IdxDist {
                    idxs_v: (0..count as usize).collect(),
                    dists_v: dists.repo,
                  },
                },
              ),
            );
          }
        }
      }
    }
  }

  pub fn two_dense_components_dataset() -> Dataset {
    let contributions = vec![
      contrib(0, 0),
      contrib(1, 0),
      contrib(5, 0),
      contrib(3, 0),
      contrib(0, 1),
      contrib(3, 2),
      contrib(5, 2),
      contrib(5, 3),
      contrib(4, 4),
      contrib(6, 5),
      contrib(7, 5),
      contrib(2, 6),
      contrib(2, 7),
      contrib(7, 7),
      contrib(4, 7),
    ];
    Dataset::new(users(8), repos(8), contributions, false)
  }

  #[test]
  fn two_dense_components() {
    let dataset = two_dense_components_dataset();
    let first_comp = Component {
      user: vec![0, 1, 3, 5],
      repo: vec![0, 1, 2, 3],
    };
    let second_comp = Component {
      user: vec![2, 4, 6, 7],
      repo: vec![4, 5, 6, 7],
    };
    gen_test_no_limit(
      StartComponent {
        item_type: ItemType::User,
        idx: 3,
      },
      &dataset,
      first_comp.clone(),
    );
    gen_test_no_limit(
      StartComponent {
        item_type: ItemType::Repo,
        idx: 2,
      },
      &dataset,
      first_comp.clone(),
    );
    gen_test_no_limit(
      StartComponent {
        item_type: ItemType::User,
        idx: 4,
      },
      &dataset,
      second_comp.clone(),
    );
    gen_test_no_limit(
      StartComponent {
        item_type: ItemType::Repo,
        idx: 6,
      },
      &dataset,
      second_comp.clone(),
    );
  }

  pub fn two_dense_components_several_disconnected_dataset() -> Dataset {
    let contributions = vec![
      contrib(0, 1),
      contrib(1, 2),
      contrib(3, 2),
      contrib(5, 2),
      contrib(0, 3),
      contrib(5, 3),
      contrib(4, 4),
      contrib(6, 5),
      contrib(7, 5),
      contrib(2, 6),
      contrib(2, 7),
      contrib(7, 7),
      contrib(4, 7),
    ];
    Dataset::new(users(9), repos(9), contributions, false)
  }

  #[test]
  fn two_dense_components_several_disconnected() {
    let dataset = two_dense_components_several_disconnected_dataset();

    gen_test_no_limit(
      StartComponent {
        item_type: ItemType::Repo,
        idx: 0,
      },
      &dataset,
      Component {
        user: vec![],
        repo: vec![0],
      },
    );
    gen_test_no_limit(
      StartComponent {
        item_type: ItemType::User,
        idx: 5,
      },
      &dataset,
      Component {
        user: vec![0, 1, 3, 5],
        repo: vec![1, 2, 3],
      },
    );
  }
}
