use github_net::{
  add_items,
  csv_items::{RepoCsvEntry, UserCsvEntry},
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
  user_csv_list: PathBuf,

  #[structopt(parse(from_os_str))]
  repo_csv_list: PathBuf,
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  println!("adding users");

  add_items(
    opt.user_csv_list,
    6,
    |conn, user_csv_entries| -> anyhow::Result<()> {
      let users: Vec<_> = user_csv_entries
        .iter()
        .cloned()
        .map(|UserCsvEntry { github_id }| User { github_id })
        .collect();
      db::add_users(&conn, &users)?;

      Ok(())
    },
  )?;

  println!("adding repos");

  add_items(
    opt.repo_csv_list,
    6,
    |conn, repo_csv_entries| -> anyhow::Result<()> {
      let repos: Vec<_> = repo_csv_entries
        .iter()
        .cloned()
        .map(|RepoCsvEntry { repo_github_id }| Repo { github_id : repo_github_id })
        .collect();
      db::add_repos(&conn, &repos)?;

      Ok(())
    },
  )?;

  Ok(())
}
