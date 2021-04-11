#[cfg(test)]
use crate::{dataset::ContributionInput, github_api, Repo, User};
use crate::{dataset::Dataset, ItemType, UserRepoPair};
#[cfg(test)]
use std::{collections::HashSet, iter};

pub type Component = UserRepoPair<Vec<usize>>;

struct ComponentIterator<'a> {
  dataset: &'a Dataset,
  visited: UserRepoPair<Vec<bool>>,
  next_to_visit: UserRepoPair<Option<usize>>,
  empty: bool,
}

impl<'a> ComponentIterator<'a> {
  fn find(visited: &[bool]) -> Option<usize> {
    visited
      .iter()
      .cloned()
      .enumerate()
      .find(|(_, is_visited)| !is_visited)
      .map(|(i, _)| i)
  }

  fn lookup_orig_update(&mut self, item_type: ItemType) -> Vec<usize> {
    let orig_idx = self.next_to_visit[item_type].unwrap_or(0);
    let orig = Self::find(&self.visited[item_type][orig_idx..])
      .map(|i| i + orig_idx)
      .or_else(|| Self::find(&self.visited[item_type][..orig_idx]));

    self.next_to_visit[item_type] = orig.and_then(|i| {
      let idx = i + 1;
      self.visited[item_type].get(idx).and_then(|is_visited| {
        if !is_visited {
          Some(idx)
        } else {
          None
        }
      })
    });
    if let Some(idx) = orig {
      self.visited[item_type][idx] = true;
    }
    orig.into_iter().collect()
  }

  fn do_traversal(
    &mut self,
    item_type: ItemType,
    start: &mut UserRepoPair<usize>,
    component: &mut Component,
    dataset: &Dataset,
  ) {
    let start = &mut start[item_type];
    let (idxs, other_idxs) = match item_type {
      ItemType::User => (&component.user, &mut component.repo),
      ItemType::Repo => (&component.repo, &mut component.user),
    };
    for &idx in &idxs[*start..] {
      other_idxs.extend(
        dataset.contribution_idxs()[item_type][idx]
          .iter()
          .filter_map(|&i| {
            let other_idx = dataset.contributions()[i].idx[item_type.other()];
            let other_visited = &mut self.visited[item_type.other()][other_idx];
            if *other_visited {
              None
            } else {
              *other_visited = true;
              Some(other_idx)
            }
          }),
      );
      if Some(idx) == self.next_to_visit[item_type] {
        self.next_to_visit[item_type] = Some(idx + 1);
      }
    }
    *start = component[item_type].len();
  }
}

impl<'a> Iterator for ComponentIterator<'a> {
  type Item = Component;

  fn next(&mut self) -> Option<Self::Item> {
    let mut component = Component {
      user: Vec::new(),
      repo: self.lookup_orig_update(ItemType::Repo),
    };

    if component.repo.is_empty() {
      component.user = self.lookup_orig_update(ItemType::User);
      if component.user.is_empty() {
        self.empty = true;
        debug_assert!(
          self.visited.user.iter().all(|&v| v)
            && self.visited.repo.iter().all(|&v| v)
        );
        return None;
      }
    }

    let mut start = UserRepoPair { user: 0, repo: 0 };

    loop {
      for &item in &[ItemType::Repo, ItemType::User] {
        self.do_traversal(item, &mut start, &mut component, self.dataset);
      }

      if component.repo[start.repo..].is_empty() {
        break;
      }
    }

    Some(component)
  }
}
pub fn components(dataset: &Dataset) -> impl Iterator<Item = Component> + '_ {
  ComponentIterator {
    dataset,
    visited: dataset.names().as_ref().map(|v| vec![false; v.len()]),
    next_to_visit: UserRepoPair::same(None),
    empty: false,
  }
}

#[test]
fn empty() {
  let dataset = Default::default();

  gen_test(&dataset, iter::empty());
}

#[cfg(test)]
fn sort_component(mut component: Component) -> Component {
  component.repo.sort();
  component.user.sort();
  component
}

#[cfg(test)]
fn gen_test_no_expected(dataset: &Dataset) -> HashSet<Component> {
  let actual: HashSet<_> = components(dataset).map(sort_component).collect();
  for component in actual.iter() {
    for &user in &component.user {
      for &contrib_idx in &dataset.user_contributions()[user] {
        assert!(component
          .repo
          .contains(&dataset.contributions()[contrib_idx].idx.repo));
      }
    }
    for &repo in &component.repo {
      for &contrib_idx in &dataset.repo_contributions()[repo] {
        assert!(component
          .user
          .contains(&dataset.contributions()[contrib_idx].idx.user));
      }
    }
  }
  actual
}

#[cfg(test)]
fn gen_test<E: IntoIterator<Item = Component>>(
  dataset: &Dataset,
  expected_components: E,
) {
  let expected: HashSet<_> = expected_components
    .into_iter()
    .map(sort_component)
    .collect();
  let actual = gen_test_no_expected(dataset);
  assert_eq!(actual, expected);
}

#[cfg(test)]
fn users(n: github_api::ID) -> impl Iterator<Item = (User, String)> {
  (0..n).map(|github_id| (User { github_id }, "".to_owned()))
}

#[cfg(test)]
fn repos(n: github_api::ID) -> impl Iterator<Item = (Repo, String)> {
  (0..n).map(|github_id| (Repo { github_id }, "".to_owned()))
}

#[test]
fn single_user() {
  let dataset = Dataset::new(users(1), iter::empty(), iter::empty(), true);
  gen_test(
    &dataset,
    vec![Component {
      user: vec![0],
      repo: Vec::new(),
    }],
  );
}

#[test]
fn single_repo() {
  let dataset = Dataset::new(iter::empty(), repos(1), iter::empty(), true);

  gen_test(
    &dataset,
    vec![Component {
      user: Vec::new(),
      repo: vec![0],
    }],
  );
}

#[cfg(test)]
fn contrib(
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

#[test]
fn small_disconnected() {
  for &count in &[1, 2, 3, 8] {
    let dataset = Dataset::new(
      users(count),
      repos(count),
      (0..count).into_iter().map(|i| contrib(i, i)),
      false,
    );

    gen_test(
      &dataset,
      (0..count as usize).into_iter().map(|i| Component {
        user: vec![i],
        repo: vec![i],
      }),
    );
  }
}

#[test]
fn fully_connected() {
  for &count in &[1, 2, 3, 8] {
    let dataset = Dataset::new(
      users(count),
      repos(count),
      (0..count)
        .into_iter()
        .map(|i| contrib(i, i))
        .chain((0..count - 1).into_iter().map(|i| contrib(i, i + 1))),
      false,
    );

    gen_test(
      &dataset,
      iter::once(Component {
        user: (0..count as usize).collect(),
        repo: (0..count as usize).collect(),
      }),
    );
  }
}

#[test]
fn two_dense_components() {
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
  let dataset = Dataset::new(users(8), repos(8), contributions, false);

  gen_test(
    &dataset,
    vec![
      Component {
        user: vec![0, 1, 3, 5],
        repo: vec![0, 1, 2, 3],
      },
      Component {
        user: vec![2, 4, 6, 7],
        repo: vec![4, 5, 6, 7],
      },
    ],
  );
}

#[test]
fn two_dense_components_several_diconnected() {
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
  let dataset = Dataset::new(users(9), repos(9), contributions, false);

  gen_test(
    &dataset,
    vec![
      Component {
        user: vec![],
        repo: vec![0],
      },
      Component {
        user: vec![0, 1, 3, 5],
        repo: vec![1, 2, 3],
      },
      Component {
        user: vec![2, 4, 6, 7],
        repo: vec![4, 5, 6, 7],
      },
      Component {
        user: vec![8],
        repo: vec![],
      },
      Component {
        user: vec![],
        repo: vec![8],
      },
    ],
  );
}
