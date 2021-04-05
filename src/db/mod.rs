pub mod models;
pub mod add;
mod schema;
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
