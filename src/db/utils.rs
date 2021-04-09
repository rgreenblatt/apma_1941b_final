use super::{
  models::{HasGithubID, Repo, RepoEntry, User, UserEntry},
  schema::{repos::dsl as repo_dsl, users::dsl as user_dsl},
  GithubID,
};
use diesel::{
  pg::{expression::dsl as pg_dsl, PgConnection},
  prelude::*,
  QueryResult,
};
use dotenv::dotenv;
use itertools::Itertools;
use std::{collections::HashMap, env, hash::Hash};

/// note: we will pick arbitrarily if there are mutliple valid options!
pub(super) fn reorder_entries<K, T, GetKey>(
  keys: &[K],
  entries: &[T],
  get_key: GetKey,
) -> Vec<Option<T>>
where
  K: Hash + Eq,
  T: Clone + PartialEq,
  GetKey: Fn(&T) -> K,
{
  assert!(entries.len() <= keys.len());

  let id_to_idxs = {
    let mut map = HashMap::new();
    for (i, key) in keys.iter().enumerate() {
      map.entry(key).or_insert_with(Vec::new).push(i)
    }

    map
  };
  let mut out = vec![Default::default(); keys.len()];

  for entry in entries {
    for &i in id_to_idxs.get(&get_key(entry)).unwrap() {
      out[i] = Some(entry.clone());
    }
  }

  assert!(out.iter().zip(keys).all(|(v, key)| v
    .as_ref()
    .map(|entry| &get_key(entry) == key)
    .unwrap_or(true)));

  out
}

#[test]
fn reorder_entries_test() {
  assert_eq!(reorder_entries(&[], &[], |_: &i32| -> i32 { 0 }).len(), 0);
  assert_eq!(
    reorder_entries(
      &[8, 1, 2, 12, 3, 4, 0, 8],
      &[(0, 3), (8, 2), (12, 7)],
      |tup: &(usize, usize)| -> usize { tup.0 }
    ),
    vec![
      Some((8, 2)),
      None,
      None,
      Some((12, 7)),
      None,
      None,
      Some((0, 3)),
      Some((8, 2))
    ],
  );
}

fn get_entries_helper<T>(ids: &[GithubID], entries: &[T]) -> Vec<T>
where
  T: HasGithubID + Clone + Default + PartialEq,
{
  let out = reorder_entries(ids, entries, |entry| entry.get_github_id());

  out
    .into_iter()
    .map(|v| v.expect("entry should exist in this case"))
    .collect()
}

fn get_repo_entries_id(
  conn: &PgConnection,
  repo_ids: &[GithubID],
) -> QueryResult<Vec<RepoEntry>> {
  let results = repo_dsl::repos
    .filter(repo_dsl::github_id.eq(pg_dsl::any(repo_ids)))
    .get_results(conn)?;
  Ok(get_entries_helper(repo_ids, &results))
}

fn get_user_entries_id(
  conn: &PgConnection,
  user_ids: &[GithubID],
) -> QueryResult<Vec<UserEntry>> {
  let results = user_dsl::users
    .filter(user_dsl::github_id.eq(pg_dsl::any(user_ids)))
    .get_results(conn)?;
  Ok(get_entries_helper(user_ids, &results))
}

pub(super) fn get_repo_entries(
  conn: &PgConnection,
  repos: &[Repo],
) -> QueryResult<Vec<RepoEntry>> {
  get_repo_entries_id(
    conn,
    &repos.iter().map(|repo| repo.github_id).collect_vec(),
  )
}

pub(super) fn get_user_entries(
  conn: &PgConnection,
  users: &[User],
) -> QueryResult<Vec<UserEntry>> {
  get_user_entries_id(
    conn,
    &users.iter().map(|user| user.github_id).collect_vec(),
  )
}

pub fn establish_connection() -> PgConnection {
  dotenv().ok();

  let database_url =
    env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  PgConnection::establish(&database_url)
    .expect(&format!("Error connecting to {}", database_url))
}
