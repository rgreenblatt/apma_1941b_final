use super::db;
use diesel::prelude::*;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::{
  fs::File,
  io::{prelude::*, BufReader},
  path::PathBuf,
  sync::{mpsc::sync_channel, Arc, Mutex},
};

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
      conn.transaction(|| loop {
        let line = lines.lock().unwrap().pop();
        let line = if let Some(line) = line {
          line
        } else {
          return Ok(());
        };

        let mut csv_reader =
          csv::Reader::from_reader(GzDecoder::new(File::open(line)?));

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
      })
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
