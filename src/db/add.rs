#[cfg(test)]
use super::{
  models::{ContributionEntry, DependencyEntry, UserEntry},
  TestContext,
};
use super::{
  models::{
    NewContribution, NewDepencency, NewRepoName, Repo, RepoEntry, User,
  },
  schema::{
    contributions::dsl as contrib_dsl, dependencies::dsl as depends_dsl,
    repo_names::dsl as repo_name_dsl, repos::dsl as repo_dsl,
    users::dsl as user_dsl,
  },
  utils::{get_repo_entries, get_user_entries},
};
use diesel::{pg::PgConnection, prelude::*, QueryResult};
use itertools::Itertools;

#[cfg(any(test, debug_assertions))]
use std::collections::HashSet;

pub fn add_contributions(
  conn: &PgConnection,
  users: &[User],
  repos: &[Repo],
  counts: &[i32],
) -> QueryResult<()> {
  assert_eq!(users.len(), repos.len());
  assert_eq!(users.len(), counts.len());

  let users = get_user_entries(conn, users)?;
  let repos = get_repo_entries(conn, repos)?;

  let new_contributions = users
    .iter()
    .zip(repos)
    .zip(counts)
    .map(|((user, repo), &num)| NewContribution {
      user_id: user.id,
      repo_id: repo.id,
      num,
    })
    .collect_vec();

  diesel::insert_into(contrib_dsl::contributions)
    .values(new_contributions)
    .execute(conn)?;

  Ok(())
}

pub fn add_repo_names(
  conn: &PgConnection,
  names: &[String],
  repos: &[Repo],
) -> QueryResult<()> {
  assert_eq!(names.len(), repos.len());

  let repos = get_repo_entries(conn, repos)?;

  let new_repo_names = names
    .iter()
    .zip(repos)
    .map(|(name, repo)| NewRepoName {
      repo_id: repo.id,
      name: name.clone(),
    })
    .collect_vec();

  diesel::insert_into(repo_name_dsl::repo_names)
    .values(new_repo_names)
    .execute(conn)?;

  Ok(())
}

pub fn add_users(conn: &PgConnection, new_users: &[User]) -> QueryResult<()> {
  diesel::insert_into(user_dsl::users)
    .values(new_users)
    .execute(conn)?;

  Ok(())
}

pub fn add_repos(conn: &PgConnection, new_repos: &[Repo]) -> QueryResult<()> {
  diesel::insert_into(repo_dsl::repos)
    .values(new_repos)
    .on_conflict_do_nothing()
    .execute(conn)?;

  Ok(())
}

#[cfg(debug_assertions)]
use std::hash::Hash;

#[cfg(debug_assertions)]
fn has_unique_elements<T>(iter: T) -> bool
where
  T: IntoIterator,
  T::Item: Eq + Hash,
{
  let mut uniq = HashSet::new();
  iter.into_iter().all(move |x| uniq.insert(x))
}

/// "to" is what the "from" repo is depending on
pub fn add_dependencies(
  conn: &PgConnection,
  from: &RepoEntry,
  to: &[Repo],
) -> QueryResult<()> {
  #[cfg(debug_assertions)]
  debug_assert!(has_unique_elements(to));

  let to_repo_entries = get_repo_entries(conn, to)?;

  let new_dependencies = to_repo_entries
    .iter()
    .map(|to| NewDepencency {
      repo_from_id: from.id,
      repo_to_id: to.id,
    })
    .collect_vec();

  diesel::insert_into(depends_dsl::dependencies)
    .values(&new_dependencies)
    .execute(conn)?;

  Ok(())
}

#[cfg(test)]
fn expected_users(n: i32) -> Vec<UserEntry> {
  (1..n + 1)
    .map(|id| UserEntry {
      id,
      github_id: id + 10000,
    })
    .collect()
}

#[cfg(test)]
fn expected_repos(n: i32) -> Vec<RepoEntry> {
  (1..n + 1)
    .map(|id| RepoEntry {
      id,
      github_id: id + 10000000,
    })
    .collect()
}

#[test]
fn n_contributions() -> QueryResult<()> {
  for &n in &[1, 3, 8] {
    let ctx = TestContext::new("add_n_contributions");
    let conn = ctx.conn();

    let contribution_users_expected = expected_users(n);
    let contribution_repos_expected = expected_repos(n);

    let contribution_users: Vec<_> = contribution_users_expected
      .iter()
      .map(|&v| v.into())
      .collect();

    let contribution_repos: Vec<_> = contribution_repos_expected
      .iter()
      .map(|&v| v.into())
      .collect();

    let counts = (0..n as i32).collect_vec();

    add_users(conn, &contribution_users)?;
    add_repos(conn, &contribution_repos)?;

    add_contributions(conn, &contribution_users, &contribution_repos, &counts)?;
    let (users, repos, contributions): (
      Vec<UserEntry>,
      Vec<RepoEntry>,
      Vec<ContributionEntry>,
    ) = (
      user_dsl::users.load(conn)?,
      repo_dsl::repos.load(conn)?,
      contrib_dsl::contributions.load(conn)?,
    );

    assert_eq!(users.len(), n as usize);
    assert_eq!(repos.len(), n as usize);
    assert_eq!(contributions.len(), n as usize);

    assert_eq!(users, contribution_users_expected);
    assert_eq!(repos, contribution_repos_expected);
    for (
      i,
      (
        ContributionEntry {
          id,
          repo_id,
          user_id,
          num,
        },
        count,
      ),
    ) in contributions.iter().zip(counts).enumerate()
    {
      let expected_id = i as i32 + 1;
      assert_eq!(*id, expected_id);
      assert_eq!(*repo_id, expected_id);
      assert_eq!(*user_id, expected_id);
      assert_eq!(*num, count);
    }
  }

  Ok(())
}

#[test]
fn simple_dependencies() -> QueryResult<()> {
  let ctx = TestContext::new("add_simple_dependencies");
  let conn = ctx.conn();

  let repos_owned = expected_repos(4);
  let repos: Vec<Repo> = repos_owned.iter().map(|&repo| repo.into()).collect();

  add_repos(conn, &repos)?;

  add_dependencies(conn, &repos_owned[2], &repos[0..2])?;

  let depends = depends_dsl::dependencies.load::<DependencyEntry>(conn)?;

  assert_eq!(depends.len(), 2);
  assert_eq!(depends[0].repo_from_id, repos_owned[2].id);
  assert_eq!(depends[1].repo_from_id, repos_owned[2].id);
  assert_eq!(depends[0].repo_to_id, repos_owned[0].id);
  assert_eq!(depends[1].repo_to_id, repos_owned[1].id);

  add_dependencies(conn, &repos_owned[1], &repos[2..3])?;

  let depends = depends_dsl::dependencies.load::<DependencyEntry>(conn)?;

  assert_eq!(depends.len(), 3);
  assert_eq!(depends[2].repo_from_id, repos_owned[1].id);
  assert_eq!(depends[2].repo_to_id, repos_owned[2].id);

  Ok(())
}
