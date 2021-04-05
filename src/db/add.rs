#[cfg(test)]
use super::{models::Contribution, TestContext};
use super::{
  models::{NewContribution, NewDepencency, NewRepo, NewUser, Repo, User},
  schema::{
    contributions::dsl as contrib_dsl, dependencies::dsl as depends_dsl,
    repos::dsl as repo_dsl, users::dsl as user_dsl,
  },
};
use diesel::{
  pg::{expression::dsl as pg_dsl, upsert, PgConnection},
  prelude::*,
};
use itertools::Itertools;
use std::collections::HashMap;
#[cfg(test)]
use std::{collections::HashSet, error::Error};

const USERS_LOGIN_UNIQUE: &'static str = "users_login_unique";
const REPOS_OWNER_NAME_UNIQUE: &'static str = "repos_owner_name_unique";
const CONTRIBUTIONS_REPO_USER_UNIQUE: &'static str =
  "contributions_repo_user_unique";

fn dedup_with_ordering<T: Ord + Clone>(values: &[T]) -> (Vec<T>, Vec<usize>) {
  let sorted_vals = {
    let mut sorted_vals: Vec<_> = values.iter().cloned().enumerate().collect();
    sorted_vals.sort_unstable_by_key(|tup| tup.1.clone());
    sorted_vals
  };
  let mut last: Option<(usize, T)> = None;

  let mut ordering = Vec::new();
  ordering.resize(values.len(), 0);

  let mut deduped = Vec::new();

  for (orig_idx, item) in sorted_vals {
    if last.as_ref().map(|v| v.1 != item).unwrap_or(true) {
      deduped.push(item.clone());
    }

    ordering[orig_idx] = deduped.len() - 1;
    last = Some((deduped.len() - 1, item));
  }

  (deduped, ordering)
}

pub fn events(
  conn: &PgConnection,
  event_users: &[NewUser],
  event_repos: &[NewRepo],
) -> Result<(), diesel::result::Error> {
  assert_eq!(event_users.len(), event_repos.len());

  let (deduped_new_users, deduped_users_idxs) =
    dedup_with_ordering(event_users);

  let (deduped_new_repos, deduped_repos_idxs) =
    dedup_with_ordering(event_repos);

  let deduped_users: Vec<User> = diesel::insert_into(user_dsl::users)
    .values(&deduped_new_users)
    .on_conflict(upsert::on_constraint(USERS_LOGIN_UNIQUE))
    .do_update()
    // spurious update to ensure we get results!
    .set(user_dsl::id.eq(user_dsl::id))
    .get_results(conn)?;

  let deduped_repos: Vec<Repo> = diesel::insert_into(repo_dsl::repos)
    .values(&deduped_new_repos)
    .on_conflict(upsert::on_constraint(REPOS_OWNER_NAME_UNIQUE))
    .do_update()
    // spurious update to ensure we get results!
    .set(repo_dsl::id.eq(repo_dsl::id))
    .get_results(conn)?;

  assert_eq!(deduped_users.len(), deduped_new_users.len());
  assert_eq!(deduped_repos.len(), deduped_new_repos.len());

  let mut idxs: Vec<_> = deduped_users_idxs
    .iter()
    .zip(&deduped_repos_idxs)
    .map(|(&user_idx, &repo_idx)| {
      (deduped_users[user_idx].id, deduped_repos[repo_idx].id)
    })
    .collect();
  idxs.sort();
  let new_contributions: Vec<_> = idxs
    .iter()
    .dedup_with_count()
    .map(|(count, &(user_id, repo_id))| NewContribution {
      user_id,
      repo_id,
      num: count as i32,
    })
    .collect();

  diesel::insert_into(contrib_dsl::contributions)
    .values(new_contributions)
    .on_conflict(upsert::on_constraint(CONTRIBUTIONS_REPO_USER_UNIQUE))
    .do_update()
    // increment
    .set(
      contrib_dsl::num
        .eq(contrib_dsl::num + upsert::excluded(contrib_dsl::num)),
    )
    .execute(conn)?;

  Ok(())
}

/// "to" is what the "from" repo is depending on
pub fn dependencies(
  conn: &PgConnection,
  from: &Repo,
  to: &[NewRepo],
) -> Result<(), diesel::result::Error> {
  let names = to
    .iter()
    .map(|NewRepo { owner_name }| *owner_name)
    .collect_vec();

  let owner_name_to_id: HashMap<String, i32> = repo_dsl::repos
    .filter(repo_dsl::owner_name.eq(pg_dsl::any(&names)))
    .load::<Repo>(conn)?
    .into_iter()
    .map(|Repo { id, owner_name }| (owner_name, id))
    .collect();

  assert_eq!(owner_name_to_id.len(), to.len());

  let new_dependencies = to
    .iter()
    .map(|NewRepo { owner_name }| NewDepencency {
      repo_from_id: from.id,
      repo_to_id: *owner_name_to_id.get(*owner_name).unwrap(),
    })
    .collect_vec();

  diesel::insert_into(depends_dsl::dependencies)
    .values(&new_dependencies)
    .execute(conn)?;

  Ok(())
}

#[cfg(test)]
fn expected_users(n: i32) -> Vec<User> {
  (1..n + 1)
    .map(|id| User {
      id,
      login: format!("user_login_{}", id),
    })
    .collect()
}

#[cfg(test)]
fn expected_repos(n: i32) -> Vec<Repo> {
  (1..n + 1)
    .map(|id| Repo {
      id,
      owner_name: format!("repo_owner_{0}/repo_owner_{0}", id),
    })
    .collect()
}

#[test]
fn n_events() -> Result<(), Box<dyn Error>> {
  for &n in &[1, 3, 8] {
    let ctx = TestContext::new("add_n_events");
    let conn = ctx.conn();

    let event_users_expected = expected_users(n);
    let event_repos_expected = expected_repos(n);

    let event_users: Vec<_> =
      event_users_expected.iter().map(|v| v.to_new()).collect();

    let event_repos: Vec<_> =
      event_repos_expected.iter().map(|v| v.to_new()).collect();

    events(conn, &event_users, &event_repos)?;
    let (users, repos, contributions): (
      Vec<User>,
      Vec<Repo>,
      Vec<Contribution>,
    ) = (
      user_dsl::users.load(conn)?,
      repo_dsl::repos.load(conn)?,
      contrib_dsl::contributions.load(conn)?,
    );

    assert_eq!(users.len(), n as usize);
    assert_eq!(repos.len(), n as usize);
    assert_eq!(contributions.len(), n as usize);

    assert_eq!(users, event_users_expected);
    assert_eq!(repos, event_repos_expected);
    for (
      i,
      Contribution {
        id,
        repo_id,
        user_id,
        num,
      },
    ) in contributions.iter().enumerate()
    {
      let expected_id = i as i32 + 1;
      assert_eq!(*id, expected_id);
      assert_eq!(*repo_id, expected_id);
      assert_eq!(*user_id, expected_id);
      assert_eq!(*num, 1);
    }
  }

  Ok(())
}

#[cfg(test)]
fn permute_clone<T: Clone>(values: &[T], permutation: &[usize]) -> Vec<T> {
  permutation.iter().map(|&i| values[i].clone()).collect()
}

#[cfg(test)]
fn users_as_set(users: &[User]) -> HashSet<String> {
  users
    .iter()
    .cloned()
    .map(|User { login, .. }| login)
    .collect()
}

#[cfg(test)]
fn repos_as_set(repos: &[Repo]) -> HashSet<String> {
  repos
    .iter()
    .cloned()
    .map(|Repo { owner_name, .. }| (owner_name))
    .collect()
}

#[test]
fn dup_event() -> Result<(), Box<dyn Error>> {
  let ctx = TestContext::new("add_dup_events");
  let conn = ctx.conn();

  let overall_users = expected_users(3);
  let overall_repos = expected_repos(4);

  let overall_new_users: Vec<_> =
    overall_users.iter().map(|v| v.to_new()).collect();

  let overall_new_repos: Vec<_> =
    overall_repos.iter().map(|v| v.to_new()).collect();

  struct Test {
    user_event_ordering: Vec<usize>,
    repo_event_ordering: Vec<usize>,
    expected_contributions: Vec<(usize, usize, i32)>,
    expected_users: Vec<usize>,
    expected_repos: Vec<usize>,
  }

  let tests = vec![
    Test {
      user_event_ordering: vec![0, 0, 1, 0, 1, 1, 1, 0],
      repo_event_ordering: vec![0, 1, 0, 0, 1, 0, 0, 2],
      expected_contributions: vec![
        (0, 0, 2),
        (0, 1, 1),
        (1, 0, 3),
        (1, 1, 1),
        (0, 2, 1),
      ],
      expected_users: vec![0, 1],
      expected_repos: vec![0, 1, 2],
    },
    Test {
      user_event_ordering: vec![0, 2, 0, 1, 1, 1, 0],
      repo_event_ordering: vec![0, 1, 0, 1, 0, 0, 2],
      expected_contributions: vec![
        (0, 0, 4),
        (0, 1, 1),
        (1, 0, 5),
        (1, 1, 2),
        (0, 2, 2),
        (2, 1, 1),
      ],
      expected_users: vec![0, 1, 2],
      expected_repos: vec![0, 1, 2],
    },
  ];

  for Test {
    user_event_ordering,
    repo_event_ordering,
    expected_contributions,
    expected_users,
    expected_repos,
  } in tests
  {
    let event_users: Vec<_> =
      permute_clone(&overall_new_users, &user_event_ordering);
    let event_repos: Vec<_> =
      permute_clone(&overall_new_repos, &repo_event_ordering);

    events(conn, &event_users, &event_repos)?;

    let (users, repos, contributions): (
      Vec<User>,
      Vec<Repo>,
      Vec<Contribution>,
    ) = (
      user_dsl::users.load(conn)?,
      repo_dsl::repos.load(conn)?,
      contrib_dsl::contributions.load(conn)?,
    );

    assert_eq!(
      users_as_set(&users),
      users_as_set(&permute_clone(&overall_users, &expected_users))
    );
    assert_eq!(
      repos_as_set(&repos),
      repos_as_set(&permute_clone(&overall_repos, &expected_repos))
    );

    let expected_new_contributions: Vec<_> = expected_contributions
      .into_iter()
      .map(|(user_idx, repo_idx, num)| NewContribution {
        user_id: users
          .iter()
          .find(|user| user.login == overall_users[user_idx].login)
          .unwrap()
          .id,
        repo_id: repos
          .iter()
          .find(|repo| repo.owner_name == overall_repos[repo_idx].owner_name)
          .unwrap()
          .id,
        num,
      })
      .collect();
    for Contribution {
      repo_id,
      user_id,
      num,
      ..
    } in contributions
    {
      assert_eq!(
        num,
        expected_new_contributions
          .iter()
          .find(
            |contrib| contrib.repo_id == repo_id && contrib.user_id == user_id
          )
          .unwrap()
          .num
      );
    }
  }

  Ok(())
}
