pub mod add;
pub mod models;
pub mod schema;
#[cfg(test)]
mod test_context;

use diesel::{pg::PgConnection, prelude::*};
use dotenv::dotenv;
use std::env;

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
) -> diesel::QueryResult<Vec<models::Repo>> {
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
