use github_net::{
  add_items,
  csv_items::{ContributionCsvEntry, RepoCsvEntry},
  db, Repo, User,
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "build_dataset_step_1",
  about = "fill the database (postgres) from .csv.gz files"
)]
struct Opt {
  #[structopt(parse(from_os_str))]
  repo_csv_list: PathBuf,

  #[structopt(parse(from_os_str))]
  contribution_csv_list: PathBuf,
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  println!("adding repo names");

  add_items(
    opt.repo_csv_list,
    6,
    |conn, repo_csv_entries| -> anyhow::Result<()> {
      let repos: Vec<_> = repo_csv_entries
        .iter()
        .cloned()
        .map(|RepoCsvEntry { github_id, .. }| Repo { github_id })
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
    opt.contribution_csv_list,
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
