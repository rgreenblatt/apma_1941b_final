use flate2::read::GzDecoder;
use github_net::{db, github_api, Repo, User};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::{
  fs::File,
  io::{prelude::*, BufReader},
  path::PathBuf,
  sync::{mpsc::sync_channel, Arc, Mutex},
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "build_dataset",
  about = "fill the database (postgres) from .csv.gz files"
)]
struct Opt {
  #[structopt(parse(from_os_str))]
  user_csv_list: PathBuf,

  #[structopt(parse(from_os_str))]
  repo_csv_list: PathBuf,

  #[structopt(parse(from_os_str))]
  contribution_csv_list: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct UserCsvEntry {
  github_id: github_api::ID,
}

#[derive(Clone, Debug, Deserialize)]
struct RepoCsvEntry {
  github_id: github_api::ID,
  name: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct ContributionCsvEntry {
  user_github_id: github_api::ID,
  repo_github_id: github_api::ID,
  num: i32,
}

pub fn add_items<T, F>(
  list: PathBuf,
  num_threads: usize,
  f: F,
) -> anyhow::Result<()>
where
  T: for<'a> Deserialize<'a>,
  F: 'static
    + Sync
    + Send
    + Clone
    + Fn(&diesel::PgConnection, &[T]) -> anyhow::Result<()>,
{
  let user_reader = BufReader::new(File::open(list)?);
  let lines = user_reader.lines().collect::<Result<Vec<_>, _>>()?;
  let lines = Arc::new(Mutex::new(lines));
  let bar = ProgressBar::new(!0);
  bar.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {pos} {per_sec}"),
  );

  let (sender, reciever) = sync_channel(0);

  let mut threads = Vec::new();
  for _ in 0..num_threads {
    let bar = bar.clone();
    let lines = lines.clone();
    let f = f.clone();
    let to_run = move || -> anyhow::Result<()> {
      let mut new_items = Vec::new();
      let conn = db::establish_connection();
      loop {
        let line = lines.lock().unwrap().pop();
        let line = if let Some(line) = line {
          line
        } else {
          return Ok(());
        };

        let mut csv_reader = csv::Reader::from_reader(GzDecoder::new(
          BufReader::new(File::open(line)?),
        ));

        let add_items = |new_items: &mut Vec<T>| -> anyhow::Result<()> {
          f(&conn, new_items)?;

          bar.inc(new_items.len() as u64);

          new_items.clear();

          Ok(())
        };

        for record in csv_reader.deserialize() {
          new_items.push(record?);

          if new_items.len() >= 2usize.pow(14) {
            add_items(&mut new_items)?;
          }
        }
        add_items(&mut new_items)?;
      }
    };
    let sender = sender.clone();
    let thread = std::thread::spawn(move || {
      let _ = sender.send(to_run());
    });
    threads.push(thread);
  }

  let mut finished = 0;

  while finished < num_threads {
    match reciever.recv().unwrap() {
      Err(err) => return Err(err),
      Ok(()) => finished += 1,
    }
  }

  Ok(())
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
      db::add::add_users(&conn, &users)?;

      Ok(())
    },
  )?;

  println!("adding repos");

  add_items(
    opt.repo_csv_list,
    1,
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
      db::add::add_repos(&conn, &repos)?;
      db::add::add_repo_names(&conn, &names, &repos)?;

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
      db::add::add_contributions(conn, &users, &repos, &counts)?;

      Ok(())
    },
  )?;

  Ok(())
}
