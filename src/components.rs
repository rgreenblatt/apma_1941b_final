use crate::{
  dataset::{Dataset, Lens},
  traversal::{default_visited, traverse, Component, Node},
  UserRepoPair,
};
/// MUCH better perf on pop
use std::collections::BTreeSet as Set;

struct ComponentIterator<'a, F> {
  dataset: &'a Dataset,
  visited: UserRepoPair<Vec<bool>>,
  not_visited: UserRepoPair<Set<usize>>,
  empty: bool,
  callback: F,
}

pub fn pop<T>(set: &mut Set<T>) -> Option<T>
where
  T: Eq + Clone + Ord,
{
  let elt = set.iter().next().cloned()?;
  set.remove(&elt);
  Some(elt)
}

impl<'a, F> Iterator for ComponentIterator<'a, F>
where
  F: Fn(Node),
{
  type Item = Component;

  fn next(&mut self) -> Option<Self::Item> {
    let start = self
      .not_visited
      .as_mut()
      .iter_with()
      .map(|(item_type, not_visited)| {
        pop(not_visited).map(|idx| Node { idx, item_type })
      })
      .find(|v| v.is_some())
      .map(|v| v.unwrap());

    let callback = &self.callback;
    let not_visited = &mut self.not_visited;

    let mut component = if let Some(start) = start {
      start.set_visited(&mut self.visited);
      callback(start);
      start.into()
    } else {
      self.empty = true;
      assert!(
        self.visited.user.iter().all(|&v| v)
          && self.visited.repo.iter().all(|&v| v)
      );
      return None;
    };

    traverse(
      &mut component,
      &mut self.visited,
      self.dataset,
      None,
      |node, _| {
        callback(node);
        let Node { item_type, idx } = node;
        let present = not_visited.as_mut()[item_type].remove(&idx);
        debug_assert!(present);
      },
    );

    Some(component)
  }
}

pub fn components_callback<'a>(
  dataset: &'a Dataset,
  callback: impl Fn(Node) + 'a,
) -> impl Iterator<Item = Component> + 'a {
  ComponentIterator {
    dataset,
    visited: default_visited(dataset),
    not_visited: dataset.lens().map(|l| (0..l).collect()),
    empty: false,
    callback,
  }
}

pub fn components(dataset: &Dataset) -> impl Iterator<Item = Component> + '_ {
  components_callback(dataset, |_| {})
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::dataset;
  use crate::traversal::test::{
    fully_connected_dataset, single_repo_dataset, single_user_dataset,
    small_disconnected_dataset, two_dense_components_dataset,
    two_dense_components_several_disconnected_dataset,
  };
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
      for (item_type, idxs) in component.as_ref().iter_with() {
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
          1 as usize..100,
          1 as usize..100,
          1 as usize..=2,
          1usize..1000,
        ),
      ) {
        gen_test_no_expected(&dataset)?;
      }
  }
}
