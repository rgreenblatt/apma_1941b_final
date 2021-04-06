use github_net::{db, github_api};

pub fn main() -> anyhow::Result<()> {
  let conn = db::establish_connection();

  let repos: Vec<db::models::Repo> = db::get_repos(&conn, Some(5000))?;

  for repo in repos {
    dbg!(github_api::get_repo_dependencies(repo.owner(), repo.name())
      .collect::<Vec<_>>());
  }

  Ok(())
}
