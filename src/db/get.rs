use super::{
  models::{NewDepencency, Repo, RepoEntry, RepoNameEntry},
  schema::{
    contributions::dsl as contrib_dsl, repo_names::dsl as repo_name_dsl,
    repos::dsl as repo_dsl, users::dsl as user_dsl,
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
  if names.len() == 0 {
    return Ok(Vec::new());
  }

  let mut str_names: Vec<_> = names.iter().collect();
  str_names.sort();
  let deduped_names: Vec<_> = str_names.iter().dedup().cloned().collect();

  // it might be possible to optimize this somewhat...
  let entries: Vec<RepoNameEntry> = repo_name_dsl::repo_names
    .filter(repo_name_dsl::name.eq(pg_dsl::any(&deduped_names)))
    .get_results(conn)?;

  // TODO: fix this clone
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
  if names.len() == 0 {
    return Ok(Vec::new());
  }

  let mut str_names: Vec<_> = names.iter().collect();
  str_names.sort();
  let deduped_names: Vec<_> = str_names.iter().dedup().cloned().collect();

  // it might be possible to optimize this somewhat...
  let entries: Vec<RepoNameEntry> = repo_name_dsl::repo_names
    .filter(repo_name_dsl::name.eq(pg_dsl::any(&deduped_names)))
    .get_results(conn)?;

  // TODO: fix this clone
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

/// this is mostly for debugging purposes
pub fn counts(conn: &PgConnection) -> QueryResult<(i64, i64, i64)> {
  Ok((
    repo_dsl::repos.count().first(conn)?,
    contrib_dsl::contributions.count().first(conn)?,
    user_dsl::users.count().first(conn)?,
  ))
}
