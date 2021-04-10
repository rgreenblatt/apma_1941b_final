use github_net::{
  add_items,
  csv_items::{
    get_csv_list_paths, ContributionCsvEntry, RepoNameCsvEntry,
    UserLoginCsvEntry,
  },
  db, Repo, User,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "build_dataset_step_1",
  about = "fill the database (postgres) from .csv.gz files"
)]
struct Opt {
  // no options right now
}

pub fn main() -> anyhow::Result<()> {
  let _ = Opt::from_args();

  let items = get_csv_list_paths();

  println!("adding user logins");

  add_items(
    items.user_login_csv_list,
    6,
    |conn, user_csv_entries| -> anyhow::Result<()> {
      let users: Vec<_> = user_csv_entries
        .iter()
        .cloned()
        .map(|UserLoginCsvEntry { github_id, .. }| User { github_id })
        .collect();
      let logins: Vec<_> = user_csv_entries
        .iter()
        .map(|entry| entry.login.clone())
        .collect();
      db::add_user_logins(&conn, &logins, &users)?;

      Ok(())
    },
  )?;

  println!("adding repo names");

  add_items(
    items.repo_name_csv_list,
    6,
    |conn, repo_csv_entries| -> anyhow::Result<()> {
      let repos: Vec<_> = repo_csv_entries
        .iter()
        .cloned()
        .map(|RepoNameCsvEntry { github_id, .. }| Repo { github_id })
        .collect();
      let names: Vec<_> = repo_csv_entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
      db::add_repo_names(&conn, &names, &repos)?;

      Ok(())
    },
  )?;

  println!("adding contributions");

  add_items(
    items.contribution_csv_list,
    6,
    |conn,
     contribution_csv_entries: &[ContributionCsvEntry]|
     -> anyhow::Result<()> {
      let users: Vec<_> = contribution_csv_entries
        .iter()
        .map(|entry| User {
          github_id: entry.user_github_id,
        })
        .collect();
      let repos: Vec<_> = contribution_csv_entries
        .iter()
        .map(|entry| Repo {
          github_id: entry.repo_github_id,
        })
        .collect();
      let counts: Vec<_> = contribution_csv_entries
        .iter()
        .map(|entry| entry.num)
        .collect();
      db::add_contributions(conn, &users, &repos, &counts)?;

      Ok(())
    },
  )?;

  Ok(())
}
