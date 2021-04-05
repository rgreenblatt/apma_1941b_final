use flate2::read::GzDecoder;
use github_net::db;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
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

struct State {
  conn: diesel::PgConnection,
  lines: Vec<String>,
  bar: ProgressBar,
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  let conn = db::establish_connection();

  let reader = BufReader::new(File::open(opt.input_list_file)?);
  let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
  let bar = ProgressBar::new(lines.len() as u64);
  bar.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {bar} {pos} / {len} {eta_precise}"),
  );

  let state = Arc::new(Mutex::new(State { conn, lines, bar }));

  let mut children = Vec::new();

  let (sender, reciever) = sync_channel(0);

  let num_threads = num_cpus::get();

  for _ in 0..num_threads {
    let state = state.clone();
    let sender = sender.clone();
    children.push(thread::spawn(move || {
      let mut logins_repos = Vec::new();
      loop {
        // is the inner BufReader needed here?
        let line = {
          if let Some(line) = state.lock().unwrap().lines.pop() {
            line
          } else {
            let _ = sender.send(Ok(()));
            return;
          }
        };

        let f = || -> anyhow::Result<()> {
          let json_reader =
            BufReader::new(GzDecoder::new(BufReader::new(File::open(line)?)));

          for line in json_reader.lines() {
            let line = line?;
            let v: Value = serde_json::from_str(&line)?;
            let v = v.as_object().unwrap();
            let login = v["actor"].as_object().unwrap()["login"]
              .as_str()
              .unwrap()
              .to_owned();
            let repo = v["repo"].as_object().unwrap()["name"]
              .as_str()
              .unwrap()
              .to_owned();

            logins_repos.push((login, repo));
          }

          let state = state.lock().unwrap();

          for chunk in logins_repos.chunks(2usize.pow(14)) {
            let mut new_users = Vec::new();
            let mut new_repos = Vec::new();

            for (login, repo) in chunk {
              new_users.push(db::models::NewUser { login });
              new_repos.push(db::models::NewRepo::new(repo));
            }

            db::add::events(&state.conn, &new_users, &new_repos)?;
            new_users.clear();
            new_repos.clear();
          }

          logins_repos.clear();

          state.bar.inc(1);

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
