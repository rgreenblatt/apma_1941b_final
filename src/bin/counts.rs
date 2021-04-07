use github_net::db;

pub fn main() -> anyhow::Result<()> {
  let conn = db::establish_connection();

  dbg!(db::counts(&conn)?);

  Ok(())
}
