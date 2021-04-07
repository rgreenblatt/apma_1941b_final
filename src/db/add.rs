#[cfg(test)]
use super::{
  models::{ContributionEntry, DependencyEntry},
  TestContext,
};
use super::{
  models::{
    HasGithubID, NewContribution, NewDepencency, Repo, RepoEntry, User,
    UserEntry,
  },
  schema::{
    contributions::dsl as contrib_dsl, dependencies::dsl as depends_dsl,
    repos::dsl as repo_dsl, users::dsl as user_dsl,
  },
  GithubID,
};
use diesel::{
  pg::{expression::dsl as pg_dsl, PgConnection},
  prelude::*,
  QueryResult,
};
use itertools::Itertools;
use std::collections::HashMap;

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

pub fn add_users(conn: &PgConnection, new_users: &[User]) -> QueryResult<()> {
  diesel::insert_into(user_dsl::users)
    .values(new_users)
    .execute(conn)?;

  Ok(())
}

pub fn add_repos(conn: &PgConnection, new_repos: &[Repo]) -> QueryResult<()> {
  diesel::insert_into(repo_dsl::repos)
    .values(new_repos)
    .execute(conn)?;

  Ok(())
}

fn get_entries_helper<T>(ids: &[GithubID], results: Vec<T>) -> Vec<T>
where
  T: HasGithubID + Clone + Default + PartialEq,
{
  assert!(results.len() <= ids.len());

  let id_to_idxs = {
    let mut map = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
      map.entry(id).or_insert_with(Vec::new).push(i)
    }

    map
  };
  let mut out = vec![Default::default(); ids.len()];

  for entry in results {
    for &i in id_to_idxs.get(&entry.get_github_id()).unwrap() {
      out[i] = entry.clone();
    }
  }

  assert!(out.iter().all(|v| *v != Default::default()));
  assert!(out.iter().zip(ids).all(|(v, id)| v.get_github_id() == *id));

  out
}

fn get_repo_entries_id(
  conn: &PgConnection,
  repo_ids: &[GithubID],
) -> QueryResult<Vec<RepoEntry>> {
  let results = repo_dsl::repos
    .filter(repo_dsl::github_id.eq(pg_dsl::any(repo_ids)))
    .get_results(conn)?;
  Ok(get_entries_helper(repo_ids, results))
}

fn get_user_entries_id(
  conn: &PgConnection,
  user_ids: &[GithubID],
) -> QueryResult<Vec<UserEntry>> {
  let results = user_dsl::users
    .filter(user_dsl::github_id.eq(pg_dsl::any(user_ids)))
    .get_results(conn)?;
  Ok(get_entries_helper(user_ids, results))
}

fn get_repo_entries(
  conn: &PgConnection,
  repos: &[Repo],
) -> QueryResult<Vec<RepoEntry>> {
  get_repo_entries_id(
    conn,
    &repos.iter().map(|repo| repo.github_id).collect_vec(),
  )
}

fn get_user_entries(
  conn: &PgConnection,
  users: &[User],
) -> QueryResult<Vec<UserEntry>> {
  get_user_entries_id(
    conn,
    &users.iter().map(|user| user.github_id).collect_vec(),
  )
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
pub fn dependencies(
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

  dependencies(conn, &repos_owned[2], &repos[0..2])?;

  let depends = depends_dsl::dependencies.load::<DependencyEntry>(conn)?;

  assert_eq!(depends.len(), 2);
  assert_eq!(depends[0].repo_from_id, repos_owned[2].id);
  assert_eq!(depends[1].repo_from_id, repos_owned[2].id);
  assert_eq!(depends[0].repo_to_id, repos_owned[0].id);
  assert_eq!(depends[1].repo_to_id, repos_owned[1].id);

  dependencies(conn, &repos_owned[1], &repos[2..3])?;

  let depends = depends_dsl::dependencies.load::<DependencyEntry>(conn)?;

  assert_eq!(depends.len(), 3);
  assert_eq!(depends[2].repo_from_id, repos_owned[1].id);
  assert_eq!(depends[2].repo_to_id, repos_owned[2].id);

  Ok(())
}
