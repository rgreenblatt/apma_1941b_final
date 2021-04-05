use diesel::{pg::PgConnection, prelude::*};
use dotenv::dotenv;
use std::env;

embed_migrations!("migrations/");

// Keep the database info in mind to drop them later
pub struct TestContextInner {
  base_url: String,
  db_name: String,
}

pub struct TestContext {
  inner: TestContextInner,
  db_conn: PgConnection,
}

impl TestContextInner {
  fn postgres_connection(&self) -> PgConnection {
    PgConnection::establish(&format!("{}/postgres", self.base_url))
      .expect("Cannot connect to postgres database.")
  }
}

impl TestContext {
  pub fn new(db_name: &str) -> TestContext {
    dotenv().ok();

    let base_url =
      env::var("BASE_TEST_URL").expect("BASE_TEST_URL must be set");

    let inner = TestContextInner {
      base_url,
      db_name: db_name.to_owned(),
    };

    // First, connect to postgres db to be able to create our test
    // database.
    let postgress_conn = inner.postgres_connection();

    // let query =
    //   diesel::sql_query(format!("DROP DATABASE {}", inner.db_name).as_str());
    // let _ = query.execute(&postgress_conn);

    // Create a new database for the test
    let query =
      diesel::sql_query(format!("CREATE DATABASE {}", inner.db_name).as_str());
    query
      .execute(&postgress_conn)
      .expect(format!("Could not create database {}", inner.db_name).as_str());

    let db_url = format!("{}/{}", inner.base_url, inner.db_name);

    let db_conn = PgConnection::establish(&db_url)
      .expect(&format!("error connecting to test db url {}", db_url));

    embedded_migrations::run(&db_conn).unwrap();

    TestContext { inner, db_conn }
  }

  pub fn conn(&self) -> &PgConnection {
    &self.db_conn
  }
}

impl Drop for TestContext {
  fn drop(&mut self) {
    let postgress_conn = self.inner.postgres_connection();

    let disconnect_users = format!(
      "SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname = '{}';",
      self.inner.db_name
    );

    diesel::sql_query(disconnect_users.as_str())
      .execute(&postgress_conn)
      .unwrap();

    let query = diesel::sql_query(
      format!("DROP DATABASE {}", self.inner.db_name).as_str(),
    );
    query
      .execute(&postgress_conn)
      .expect(&format!("Couldn't drop database {}", self.inner.db_name));
  }
}
