use flate2::read::GzDecoder;
use github_net::{github_api, Repo, User};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::{
  collections::HashMap,
  fs::File,
  io::{prelude::*, BufReader},
  iter::FromIterator,
  ops,
  path::PathBuf,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "load_dataset",
  about = "load the dataset directly into memory from .csv.gz files"
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
  num: u32,
}

#[derive(Clone, Copy, Debug)]
struct Contribution {
  user_idx: usize,
  repo_idx: usize,
  num: u32,
}

#[derive(Clone, Debug)]
struct EdgeVec<T> {
  ends: Vec<usize>,
  values: Vec<T>,
}

impl<T> EdgeVec<T> {
  pub fn new() -> Self {
    EdgeVec {
      ends: Vec::new(),
      values: Vec::new(),
    }
  }

  pub fn push<V: IntoIterator<Item = T>>(&mut self, items: V) {
    self.values.extend(items);
    self.ends.push(self.values.len());
  }

  pub fn start(&self, i: usize) -> usize {
    if i == 0 {
      0
    } else {
      self.ends[i - 1]
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = &[T]> {
    (0..self.ends.len()).map(move |i| &self[i])
  }
}

impl<T, V: IntoIterator<Item = T>> FromIterator<V> for EdgeVec<T> {
  fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
    let mut edge_vec = EdgeVec::new();

    for v in iter {
      edge_vec.push(v);
    }

    edge_vec
  }
}

impl<T> ops::Index<usize> for EdgeVec<T> {
  type Output = [T];

  fn index(&self, i: usize) -> &Self::Output {
    let start = self.start(i);
    &self.values[start..self.ends[i]]
  }
}

impl<T> ops::IndexMut<usize> for EdgeVec<T> {
  fn index_mut(&mut self, i: usize) -> &mut Self::Output {
    let start = self.start(i);
    &mut self.values[start..self.ends[i]]
  }
}

pub fn add_items<T, F>(list: PathBuf, mut f: F) -> anyhow::Result<()>
where
  T: for<'a> Deserialize<'a>,
  F: FnMut(T),
{
  let user_reader = BufReader::new(File::open(list)?);
  let lines = user_reader.lines().collect::<Result<Vec<_>, _>>()?;
  let bar = ProgressBar::new(!0);
  bar.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {pos} {per_sec}"),
  );

  for line in lines {
    let mut csv_reader = csv::Reader::from_reader(GzDecoder::new(
      BufReader::new(File::open(line)?),
    ));

    let mut count = 0;
    for record in csv_reader.deserialize() {
      f(record?);
      count += 1;
      if count % 100000 == 0 {
        bar.inc(count);
        count = 0;
      }
    }
    bar.inc(count);
  }

  Ok(())
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  let mut users = Vec::new();
  let mut user_to_idx = HashMap::new();

  add_items(opt.user_csv_list, |UserCsvEntry { github_id }| {
    let user = User { github_id };
    user_to_idx.insert(user, users.len());
    users.push(user);
  })?;

  let mut repos = Vec::new();
  let mut repo_names = Vec::new();
  let mut repo_to_idx = HashMap::new();

  add_items(opt.repo_csv_list, |RepoCsvEntry { github_id, name }| {
    let repo = Repo { github_id };
    repo_to_idx.insert(repo, repos.len());
    repos.push(repo);
    repo_names.push(name);
  })?;

  let mut user_contributions = vec![Vec::new(); users.len()];
  let mut repo_contributions = vec![Vec::new(); repos.len()];

  let mut contributions = Vec::new();

  add_items(
    opt.contribution_csv_list,
    |ContributionCsvEntry {
       user_github_id,
       repo_github_id,
       num,
     }| {
      let repo = Repo {
        github_id: repo_github_id,
      };
      let user = User {
        github_id: user_github_id,
      };

      let idx = contributions.len();

      let user_idx = *user_to_idx.get(&user).unwrap();
      let repo_idx = *repo_to_idx.get(&repo).unwrap();

      user_contributions[user_idx].push(idx);
      repo_contributions[repo_idx].push(idx);

      contributions.push(Contribution {
        user_idx,
        repo_idx,
        num,
      });
    },
  )?;

  drop(user_to_idx);
  drop(repo_to_idx);

  let user_contributions: EdgeVec<_> = user_contributions.into_iter().collect();
  let repo_contributions: EdgeVec<_> = repo_contributions.into_iter().collect();

  dbg!(user_contributions.iter().map(|v| v.len()).max());
  dbg!(user_contributions.iter().map(|v| v.len()).min());
  dbg!(repo_contributions.iter().map(|v| v.len()).max());
  dbg!(repo_contributions.iter().map(|v| v.len()).min());

  dbg!("fin");
  std::thread::sleep(std::time::Duration::new(8, 0));

  Ok(())
}
