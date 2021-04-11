use crate::{
  dataset::Dataset,
  traversal::{default_visited, traverse, Component, StartComponent},
  ItemType, UserRepoPair,
};

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

  fn lookup_start_component_update(
    &mut self,
    item_type: ItemType,
  ) -> Option<StartComponent> {
    let to_visit_idx = self.next_to_visit[item_type].unwrap_or(0);
    let start_idx = Self::find(&self.visited[item_type][to_visit_idx..])
      .map(|i| i + to_visit_idx)
      .or_else(|| Self::find(&self.visited[item_type][..to_visit_idx]));

    self.next_to_visit[item_type] = start_idx.and_then(|i| {
      let idx = i + 1;
      self.visited[item_type].get(idx).and_then(|is_visited| {
        if !is_visited {
          Some(idx)
        } else {
          None
        }
      })
    });
    start_idx.map(|idx| StartComponent { item_type, idx })
  }
}

impl<'a> Iterator for ComponentIterator<'a> {
  type Item = Component;

  fn next(&mut self) -> Option<Self::Item> {
    let start =
      [ItemType::Repo, ItemType::User]
        .iter()
        .fold(None, |op, &item_type| {
          op.or_else(|| self.lookup_start_component_update(item_type))
        });

    let mut component = if let Some(start) = start {
      start.set_visited(&mut self.visited);
      start.into()
    } else {
      self.empty = true;
      debug_assert!(
        self.visited.user.iter().all(|&v| v)
          && self.visited.repo.iter().all(|&v| v)
      );
      return None;
    };

    let next_to_visit = &mut self.next_to_visit;

    traverse(
      &mut component,
      &mut self.visited,
      self.dataset,
      None,
      |item_type, idx| {
        if Some(idx) == next_to_visit[item_type] {
          next_to_visit[item_type] = Some(idx + 1);
        }
      },
    );

    Some(component)
  }
}
pub fn components(dataset: &Dataset) -> impl Iterator<Item = Component> + '_ {
  ComponentIterator {
    dataset,
    visited: default_visited(dataset),
    next_to_visit: UserRepoPair::same(None),
    empty: false,
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::traversal::test::{
    fully_connected_dataset, single_repo_dataset, single_user_dataset,
    small_disconnected_dataset, two_dense_components_dataset,
    two_dense_components_several_disconnected_dataset,
  };
  use crate::{dataset, github_api};
  use proptest::prelude::*;
  use std::{collections::HashSet, iter};

  #[test]
  fn empty() {
    let dataset = Default::default();

    gen_test(&dataset, iter::empty());
  }

  fn sort_component(mut component: Component) -> Component {
    component.repo.sort();
    component.user.sort();
    component
  }

  fn gen_test_no_expected(
    dataset: &Dataset,
  ) -> Result<HashSet<Component>, TestCaseError> {
    let actual: HashSet<_> = components(dataset).map(sort_component).collect();
    for component in actual.iter() {
      for (item_type, idxs) in component.as_ref().iter_with_types() {
        for &idx in idxs {
          for &contrib_idx in dataset.contribution_idxs()[item_type][idx].iter()
          {
            proptest::prop_assert!(component[item_type.other()].contains(
              &dataset.contributions()[contrib_idx].idx[item_type.other()]
            ));
          }
        }
      }
    }
    Ok(actual)
  }

  fn gen_test(
    dataset: &Dataset,
    expected_components: impl IntoIterator<Item = Component>,
  ) {
    let expected: HashSet<_> = expected_components
      .into_iter()
      .map(sort_component)
      .collect();
    let actual = gen_test_no_expected(dataset).unwrap();
    assert_eq!(actual, expected);
  }

  #[test]
  fn single_user() {
    let dataset = single_user_dataset();
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
    let dataset = single_repo_dataset();
    gen_test(
      &dataset,
      vec![Component {
        user: Vec::new(),
        repo: vec![0],
      }],
    );
  }

  #[test]
  fn small_disconnected() {
    for &count in &[1, 2, 3, 8] {
      let dataset = small_disconnected_dataset(count);

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
      let dataset = fully_connected_dataset(count);

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
    let dataset = two_dense_components_dataset();

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
  fn two_dense_components_several_disconnected() {
    let dataset = two_dense_components_several_disconnected_dataset();

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

  proptest::proptest! {
      #[test]
      fn proptest_components(
        dataset in dataset::strategy(
          1 as github_api::ID..100,
          1 as github_api::ID..100,
          1 as u32..=2,
          1usize..1000,
        ),
      ) {
        gen_test_no_expected(&dataset)?;
      }
  }
}
