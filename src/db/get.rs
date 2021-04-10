use super::{
  models::{NewDepencency, Repo, RepoEntry, RepoNameEntry, UserLoginEntry},
  schema::{
    contributions::dsl as contrib_dsl, repo_names::dsl as repo_name_dsl,
    repos::dsl as repo_dsl, user_logins::dsl as user_login_dsl,
    users::dsl as user_dsl,
  },
  utils::reorder_entries,
};
use diesel::{
  pg::{expression::dsl as pg_dsl, PgConnection},
  prelude::*,
  QueryResult,
};
use itertools::Itertools;

pub fn get_dependencies_from_names(
  conn: &PgConnection,
  repo: RepoEntry,
  names: &[String],
) -> QueryResult<Vec<NewDepencency>> {
  if names.is_empty() {
    return Ok(Vec::new());
  }

  let mut str_names: Vec<_> = names.iter().collect();
  str_names.sort();
  let deduped_names: Vec<_> = str_names.iter().dedup().cloned().collect();

  // it might be possible to optimize this somewhat...
  let entries: Vec<RepoNameEntry> = repo_name_dsl::repo_names
    .filter(repo_name_dsl::name.eq(pg_dsl::any(&deduped_names)))
    .get_results(conn)?;

  let out = reorder_entries(names, &entries, |entry| entry.name.clone())
    .into_iter()
    .filter_map(|repo_name| {
      repo_name.map(|repo_name| NewDepencency {
        repo_from_id: repo.id,
        repo_to_id: repo_name.repo_id,
      })
    })
    .collect();

  Ok(out)
}

pub fn get_repos_from_names(
  conn: &PgConnection,
  names: &[String],
) -> QueryResult<Vec<Option<Repo>>> {
  if names.is_empty() {
    return Ok(Vec::new());
  }

  let mut str_names: Vec<_> = names.iter().collect();
  str_names.sort();
  let deduped_names: Vec<_> = str_names.iter().dedup().cloned().collect();

  // it might be possible to optimize this somewhat...
  let entries: Vec<RepoNameEntry> = repo_name_dsl::repo_names
    .filter(repo_name_dsl::name.eq(pg_dsl::any(&deduped_names)))
    .get_results(conn)?;

  let repo_ids = reorder_entries(names, &entries, |entry| entry.name.clone())
    .into_iter()
    .map(|repo_name| repo_name.map(|repo_name| repo_name.repo_id))
    .collect_vec();

  let repo_entries: Vec<RepoEntry> = repo_dsl::repos
    .filter(repo_dsl::id.eq(pg_dsl::any(
      &repo_ids.iter().filter_map(|v| *v).collect_vec(),
    )))
    .get_results(conn)?;

  let out = reorder_entries(&repo_ids, &repo_entries, |entry| Some(entry.id))
    .into_iter()
    .map(|repo_entry| repo_entry.map(|v| v.into()))
    .collect();

  Ok(out)
}

pub fn get_repos(
  conn: &PgConnection,
  limit: Option<i64>,
) -> QueryResult<Vec<RepoEntry>> {
  match limit {
    Some(limit) => repo_dsl::repos.limit(limit).load(conn),
    None => repo_dsl::repos.load(conn),
  }
}

pub fn get_repo(conn: &PgConnection, id: i32) -> QueryResult<RepoEntry> {
  repo_dsl::repos.find(id).get_result(conn)
}

pub fn get_repo_names(
  conn: &PgConnection,
  ids: &[i32],
) -> QueryResult<Vec<String>> {
  let name_entries: Vec<RepoNameEntry> = repo_name_dsl::repo_names
    .filter(repo_name_dsl::repo_id.eq(pg_dsl::any(ids)))
    .get_results(conn)?;

  let out = reorder_entries(ids, &name_entries, |entry| entry.repo_id)
    .into_iter()
    .map(|name_entry| name_entry.unwrap().name)
    .collect();

  Ok(out)
}

pub fn get_user_logins(
  conn: &PgConnection,
  ids: &[i32],
) -> QueryResult<Vec<String>> {
  let login_entries: Vec<UserLoginEntry> = user_login_dsl::user_logins
    .filter(user_login_dsl::user_id.eq(pg_dsl::any(ids)))
    .get_results(conn)?;

  let out = reorder_entries(ids, &login_entries, |entry| entry.user_id)
    .into_iter()
    .map(|login_entry| login_entry.unwrap().login)
    .collect();

  Ok(out)
}

#[derive(Debug, Clone, Copy)]
pub struct Counts {
  pub users: i64,
  pub repos: i64,
  pub user_logins: i64,
  pub repo_names: i64,
  pub contributions: i64,
}

/// this is mostly for debugging purposes
pub fn counts(conn: &PgConnection) -> QueryResult<Counts> {
  Ok(Counts {
    users: user_dsl::users.count().first(conn)?,
    repos: repo_dsl::repos.count().first(conn)?,
    user_logins: user_login_dsl::user_logins.count().first(conn)?,
    repo_names: repo_name_dsl::repo_names.count().first(conn)?,
    contributions: contrib_dsl::contributions.count().first(conn)?,
  })
}

pub fn repo_degrees(conn: &PgConnection) -> QueryResult<Vec<(i64, i32)>> {
  contrib_dsl::contributions
    .group_by(contrib_dsl::repo_id)
    .select((diesel::dsl::count_star(), contrib_dsl::repo_id))
    .get_results(conn)
}

pub fn user_degrees(conn: &PgConnection) -> QueryResult<Vec<(i64, i32)>> {
  contrib_dsl::contributions
    .group_by(contrib_dsl::user_id)
    .select((diesel::dsl::count_star(), contrib_dsl::user_id))
    .get_results(conn)
}
