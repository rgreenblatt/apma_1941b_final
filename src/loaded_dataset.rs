use super::{
  csv_items::{
    get_csv_list_paths, ContributionCsvEntry, RepoNameCsvEntry,
    UserLoginCsvEntry,
  },
  Repo, User,
};
use anyhow::Result;
use flate2::read::GzDecoder;
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

#[derive(Clone, Copy, Debug)]
pub struct Contribution {
  pub user_idx: usize,
  pub repo_idx: usize,
  pub num: i32,
}

#[derive(Clone, Debug)]
pub struct EdgeVec<T> {
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

fn add_items<T, F>(list: PathBuf, limit: Option<usize>, mut f: F) -> Result<()>
where
  T: for<'a> Deserialize<'a>,
  F: FnMut(T),
{
  let user_reader = BufReader::new(File::open(list)?);
  let lines = user_reader
    .lines()
    .collect::<std::result::Result<Vec<_>, _>>()?;
  let bar = ProgressBar::new(!0);
  bar.set_style(
    ProgressStyle::default_bar()
      .template("[{elapsed_precise}] {pos} {per_sec}"),
  );

  let mut overall_count = 0;

  for line in lines {
    let mut csv_reader =
      csv::Reader::from_reader(GzDecoder::new(File::open(line)?));

    let mut bar_count = 0;
    for record in csv_reader.deserialize() {
      f(record?);
      bar_count += 1;
      if bar_count % 100000 == 0 {
        bar.inc(bar_count);
        bar_count = 0;
      }

      overall_count += 1;

      if let Some(limit) = limit {
        if overall_count >= limit {
          break;
        }
      }
    }
    bar.inc(bar_count);
  }

  Ok(())
}

pub struct Dataset {
  pub users: Vec<User>,
  pub repos: Vec<Repo>,
  pub user_logins: Vec<String>,
  pub repo_names: Vec<String>,
  pub contributions: Vec<Contribution>,
  pub user_contributions: EdgeVec<usize>,
  pub repo_contributions: EdgeVec<usize>,
}

impl Dataset {
  fn gen_load(limit: Option<usize>) -> Result<Self> {
    let items = get_csv_list_paths();

    let mut users = Vec::new();
    let mut user_logins = Vec::new();
    let mut user_to_idx = HashMap::new();

    add_items(
      items.user_login_csv_list,
      limit,
      |UserLoginCsvEntry { github_id, login }| {
        let user = User { github_id };
        user_to_idx.insert(user, users.len());
        users.push(user);
        user_logins.push(login);
      },
    )?;

    let mut repos = Vec::new();
    let mut repo_names = Vec::new();
    let mut repo_to_idx = HashMap::new();

    add_items(
      items.repo_name_csv_list,
      limit,
      |RepoNameCsvEntry { github_id, name }| {
        let repo = Repo { github_id };
        repo_to_idx.insert(repo, repos.len());
        repos.push(repo);
        repo_names.push(name);
      },
    )?;

    let mut user_contributions = vec![Vec::new(); users.len()];
    let mut repo_contributions = vec![Vec::new(); repos.len()];

    let mut contributions = Vec::new();

    add_items(
      items.contribution_csv_list,
      limit,
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

        let (user_idx, repo_idx) =
          (user_to_idx.get(&user), repo_to_idx.get(&repo));

        let (user_idx, repo_idx) =
          if let (Some(&u), Some(&r)) = (user_idx, repo_idx) {
            (u, r)
          } else {
            assert!(limit.is_some());
            return;
          };

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

    let user_contributions: EdgeVec<_> =
      user_contributions.into_iter().collect();
    let repo_contributions: EdgeVec<_> =
      repo_contributions.into_iter().collect();

    let out = Self {
      repos,
      users,
      user_logins,
      repo_names,
      contributions,
      user_contributions,
      repo_contributions,
    };

    Ok(out)
  }

  pub fn load() -> Result<Self> {
    Self::gen_load(None)
  }

  pub fn load_limited(limit: usize) -> Result<Self> {
    Self::gen_load(Some(limit))
  }
}
