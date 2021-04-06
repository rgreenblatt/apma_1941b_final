use flate2::read::GzDecoder;
use github_net::db;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::{
  fs::File,
  io::{prelude::*, BufReader},
  path::PathBuf,
  sync::{mpsc::sync_channel, Arc, Mutex},
  thread,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "build_dataset",
  about = "fill the database (postgres) from .json.gz files"
)]
struct Opt {
  /// Input file
  #[structopt(parse(from_os_str))]
  input_list_file: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CsvEntry {
  repo_name: String,
  login: String,
  cnt: i32,
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  let reader = BufReader::new(File::open(opt.input_list_file)?);
  let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
  let bar = ProgressBar::new(479216150);
  bar.set_style(ProgressStyle::default_bar().template(
    "[{elapsed_precise}] {bar} {pos} / {len} {eta_precise} {per_sec}",
  ));
  let bar = Arc::new(Mutex::new(bar));
  let lines = Arc::new(Mutex::new(lines));

  let mut children = Vec::new();

  let (sender, reciever) = sync_channel(0);

  let num_threads = 4;

  for _ in 0..num_threads {
    let bar = bar.clone();
    let lines = lines.clone();
    let sender = sender.clone();
    children.push(thread::spawn(move || {
      let conn = db::establish_connection();

      let mut logins_repos = Vec::new();
      let mut counts = Vec::new();
      loop {
        let line = {
          if let Some(line) = lines.lock().unwrap().pop() {
            line
          } else {
            let _ = sender.send(Ok(()));
            return;
          }
        };

        let f = || -> anyhow::Result<()> {
          // is the inner BufReader needed here?
          let mut csv_reader = csv::Reader::from_reader(GzDecoder::new(
            BufReader::new(File::open(line)?),
          ));

          let add_entries = |logins_repos: &mut Vec<(String, String)>,
                             counts: &mut Vec<i32>|
           -> anyhow::Result<()> {
            let mut new_users = Vec::new();
            let mut new_repos = Vec::new();

            for (repo, login) in logins_repos.iter() {
              new_users.push(db::models::NewUser { login });
              new_repos.push(db::models::NewRepo::new(repo));
            }

            db::add::events_counts(&conn, &new_users, &new_repos, counts)?;

            bar.lock().unwrap().inc(new_users.len() as u64);

            new_users.clear();
            new_repos.clear();
            logins_repos.clear();
            counts.clear();

            Ok(())
          };

          for record in csv_reader.deserialize() {
            let CsvEntry {
              repo_name,
              login,
              cnt,
            } = record?;
            let repo_name = if repo_name.matches('/').count() == 2 {
              let mut iter = repo_name.split('/').skip(1);
              iter.next().unwrap().to_owned() + "/" + iter.next().unwrap()
            } else {
              repo_name
            };
            logins_repos.push((repo_name, login));
            counts.push(cnt);

            if logins_repos.len() >= 2usize.pow(14) {
              add_entries(&mut logins_repos, &mut counts)?;
            }
          }
          add_entries(&mut logins_repos, &mut counts)?;

          Ok(())
        };

        match f() {
          Ok(()) => {}
          Err(err) => {
            let _ = sender.send(Err(err));
            return;
          }
        }
      }
    }));
  }

  let mut num_exited = 0;

  while num_exited < num_threads {
    match reciever.recv().unwrap() {
      Ok(()) => {
        num_exited += 1;
      }
      Err(err) => return Err(err),
    }
  }

  Ok(())
}
