pub mod add;
pub mod models;
pub mod schema;
#[cfg(test)]
mod test_context;

use diesel::{pg::PgConnection, prelude::*};
use dotenv::dotenv;
use std::env;

use crate::github_api::ID as GithubID;

#[cfg(test)]
use test_context::TestContext;

pub fn establish_connection() -> PgConnection {
  dotenv().ok();

  let database_url =
    env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  PgConnection::establish(&database_url)
    .expect(&format!("Error connecting to {}", database_url))
}

pub fn get_repos(
  conn: &PgConnection,
  limit: Option<i64>,
) -> diesel::QueryResult<Vec<models::RepoEntry>> {
  match limit {
    Some(limit) => schema::repos::table.limit(limit).load(conn),
    None => schema::repos::table.load(conn),
  }
}

pub fn counts(conn: &PgConnection) -> diesel::QueryResult<(i64, i64, i64)> {
  Ok((
    schema::repos::table.count().first(conn)?,
    schema::contributions::table.count().first(conn)?,
    schema::users::table.count().first(conn)?,
  ))
}

pub fn get_repos_from_names(
  conn: &PgConnection,
  names: &[String],
) -> anyhow::Result<Vec<models::Repo>> {
  // TODO: fix this hack!
  dbg!(names);
  names
    .iter()
    .map(|name| {
      let mut items = name.split('/');
      let get_err = || anyhow::anyhow!("unexpected owner name layout");
      let owner = items.next().ok_or_else(get_err)?;
      let name = items.next().ok_or_else(get_err)?;
      crate::github_api::get_repo(owner.to_owned(), name.to_owned())
    })
    .collect()
}
