#[cfg(test)]
use crate::GithubIDWrapper;
use crate::{
  csv_items::{
    get_csv_list_paths, ContributionCsvEntry, RepoNameCsvEntry, UserCsvEntry,
    UserLoginCsvEntry,
  },
  csv_items_iter::csv_items_iter,
  github_api,
  progress_bar::get_bar,
  EdgeVec, HasGithubID, ItemType, Repo, User, UserRepoPair,
};
use fnv::{FnvHashMap as Map, FnvHashSet as Set};
use indicatif::ProgressIterator;
#[cfg(test)]
use proptest::prelude::*;
use std::{
  fs::{self, File},
  hash::Hash,
  path::{Path, PathBuf},
};
use unzip_n::unzip_n;

#[derive(Clone, Copy, Debug)]
pub struct Contribution {
  pub idx: UserRepoPair<usize>,
  pub num: u32,
}

#[derive(Default, Debug)]
pub struct Dataset {
  users_v: Vec<User>,
  repos_v: Vec<Repo>,
  names_v: UserRepoPair<Vec<String>>,
  contributions_v: Vec<Contribution>,
  contribution_idxs_v: UserRepoPair<EdgeVec<usize>>,
}

unzip_n!(3);

type CollectedItems<T> = (Vec<T>, Vec<String>, Map<T, usize>);

#[derive(Clone, Copy, Debug)]
pub struct ContributionInput {
  pub user: User,
  pub repo: Repo,
  pub num: u32,
}

impl Dataset {
  pub fn users(&self) -> &[User] {
    &self.users_v
  }

  pub fn repos(&self) -> &[Repo] {
    &self.repos_v
  }

  pub fn len(&self, item_type: ItemType) -> usize {
    self.names()[item_type].len()
  }

  pub fn filter_contributions(&mut self, min_contribution: u32) {
    if min_contribution == 0 {
      return;
    }
    let mut contributions = UserRepoPair {
      user: vec![Vec::new(); self.users().len()],
      repo: vec![Vec::new(); self.repos().len()],
    };

    self.contributions_v = self
      .contributions_v
      .iter()
      .filter(|contrib| contrib.num >= min_contribution)
      .enumerate()
      .map(|(i, contrib)| {
        for (item_type, idx) in contrib.idx.iter_with_types() {
          contributions[item_type][idx].push(i)
        }
        *contrib
      })
      .collect();
    self.contribution_idxs_v = contributions.map(|v| v.into_iter().collect());
  }

  pub fn names(&self) -> &UserRepoPair<Vec<String>> {
    &self.names_v
  }

  pub fn user_logins(&self) -> &[String] {
    &self.names()[ItemType::User]
  }

  pub fn repo_names(&self) -> &[String] {
    &self.names()[ItemType::Repo]
  }

  pub fn contributions(&self) -> &[Contribution] {
    &self.contributions_v
  }

  pub fn contribution_idxs(&self) -> &UserRepoPair<EdgeVec<usize>> {
    &self.contribution_idxs_v
  }

  pub fn user_contributions(&self) -> &EdgeVec<usize> {
    &self.contribution_idxs()[ItemType::User]
  }

  pub fn repo_contributions(&self) -> &EdgeVec<usize> {
    &self.contribution_idxs()[ItemType::Repo]
  }

  pub fn github_ids(
    &self,
    item_type: ItemType,
  ) -> Box<dyn Iterator<Item = github_api::ID> + '_> {
    match item_type {
      ItemType::Repo => {
        Box::new(self.repos().iter().map(HasGithubID::get_github_id))
      }
      ItemType::User => {
        Box::new(self.users().iter().map(HasGithubID::get_github_id))
      }
    }
  }

  /// *Slowly* find an item (linear search)
  /// Its faster to iterate for the few we need instead of building a hashmap
  /// etc.
  pub fn find_item(&self, item_type: ItemType, name: &str) -> Option<usize> {
    self.names()[item_type]
      .iter()
      .enumerate()
      .find(|(_, other_name)| other_name == &name)
      .map(|(idx, _)| idx)
  }

  fn collect_items<T: Hash + Eq + Clone, E>(
    iter: impl IntoIterator<Item = Result<(T, String), E>>,
  ) -> Result<CollectedItems<T>, E> {
    itertools::process_results(iter, |iter| {
      iter
        .enumerate()
        .map(|(i, (item, name))| (item.clone(), name, (item, i)))
        .unzip_n()
    })
  }

  pub fn new_error<E>(
    user_iter: impl IntoIterator<Item = Result<(User, String), E>>,
    repo_iter: impl IntoIterator<Item = Result<(Repo, String), E>>,
    contributions_iter: impl IntoIterator<Item = Result<ContributionInput, E>>,
    all_contributions_must_be_used: bool,
  ) -> Result<Self, E> {
    let (users_v, user_logins_v, user_to_idx) = Self::collect_items(user_iter)?;
    let (repos_v, repo_names_v, repo_to_idx) = Self::collect_items(repo_iter)?;

    let mut user_contributions = vec![Vec::new(); users_v.len()];
    let mut repo_contributions = vec![Vec::new(); repos_v.len()];

    let mut total = 0;

    let contributions_v = contributions_iter
      .into_iter()
      .filter_map(|v| match v {
        Ok(ContributionInput { repo, user, num }) => {
          let (user_idx, repo_idx) =
            (user_to_idx.get(&user), repo_to_idx.get(&repo));

          let (user_idx, repo_idx) =
            if let (Some(&u), Some(&r)) = (user_idx, repo_idx) {
              (u, r)
            } else {
              assert!(!all_contributions_must_be_used);
              return None;
            };

          user_contributions[user_idx].push(total);
          repo_contributions[repo_idx].push(total);

          total += 1;

          let contribution = Contribution {
            idx: UserRepoPair {
              user: user_idx,
              repo: repo_idx,
            },
            num,
          };
          Some(Ok(contribution))
        }
        Err(v) => Some(Err(v)),
      })
      .collect::<Result<_, E>>()?;

    drop(user_to_idx);
    drop(repo_to_idx);

    let contribution_idxs_v = UserRepoPair {
      user: user_contributions.into_iter().collect(),
      repo: repo_contributions.into_iter().collect(),
    };

    let out = Self {
      users_v,
      repos_v,
      names_v: UserRepoPair {
        user: user_logins_v,
        repo: repo_names_v,
      },
      contributions_v,
      contribution_idxs_v,
    };

    #[cfg(debug_assertions)]
    out
      .contribution_idxs_v
      .as_ref()
      .into_iter()
      .flat_map(|v| v.iter().flat_map(|v| v.iter()))
      .for_each(|&idx| {
        debug_assert!(idx < out.contributions_v.len());
      });

    Ok(out)
  }

  pub fn new(
    user_iter: impl IntoIterator<Item = (User, String)>,
    repo_iter: impl IntoIterator<Item = (Repo, String)>,
    contributions_iter: impl IntoIterator<Item = ContributionInput>,
    all_contributions_must_be_used: bool,
  ) -> Self {
    // should be never type
    let out: Result<Self, ()> = Self::new_error(
      user_iter.into_iter().map(Ok),
      repo_iter.into_iter().map(Ok),
      contributions_iter.into_iter().map(Ok),
      all_contributions_must_be_used,
    );
    out.unwrap()
  }

  pub fn load_limited_exclude(
    limit: Option<usize>,
    users_to_exclude: &Set<User>,
  ) -> anyhow::Result<Self> {
    let items = get_csv_list_paths();

    let get_bar = || get_bar(None, 10_000);

    let user_iter = csv_items_iter(items.user_login_csv_list)?
      .progress_with(get_bar())
      .filter_map(|v| {
        let v = v.map(|UserLoginCsvEntry { github_id, login }| {
          let user = User { github_id };
          if users_to_exclude.contains(&user) {
            None
          } else {
            Some((user, login))
          }
        });
        match v {
          Err(e) => Some(Err(e)),
          Ok(Some(v)) => Some(Ok(v)),
          Ok(None) => None,
        }
      });
    let repo_iter = csv_items_iter(items.repo_name_csv_list)?
      .progress_with(get_bar())
      .map(|v| {
        v.map(|RepoNameCsvEntry { github_id, name }| (Repo { github_id }, name))
      });
    let contributions_iter = csv_items_iter(items.contribution_csv_list)?
      .progress_with(get_bar())
      .map(|v| {
        v.map(
          |ContributionCsvEntry {
             repo_github_id,
             user_github_id,
             num,
           }| ContributionInput {
            repo: Repo {
              github_id: repo_github_id,
            },
            user: User {
              github_id: user_github_id,
            },
            num,
          },
        )
      });

    if let Some(limit) = limit {
      Self::new_error(
        user_iter.take(limit),
        repo_iter.take(limit),
        contributions_iter.take(limit),
        false,
      )
    } else {
      Self::new_error(
        user_iter,
        repo_iter,
        contributions_iter,
        users_to_exclude.is_empty(),
      )
    }
  }

  fn cache_dir() -> &'static Path {
    Path::new("excluded_cache/")
  }

  fn excluded_cache_path(thresh: usize) -> PathBuf {
    let dir: PathBuf = Self::cache_dir().into();
    dir.join(format!("{}_limit_excluded.csv", thresh))
  }

  fn cache_lookup(thresh: usize) -> Option<csv::Result<Set<User>>> {
    let file = File::open(Self::excluded_cache_path(thresh));
    let file = if let Ok(file) = file {
      file
    } else {
      return None;
    };

    let out = csv::Reader::from_reader(file)
      .deserialize()
      .map(|entry| entry.map(|UserCsvEntry { github_id }| User { github_id }))
      .collect();
    Some(out)
  }

  fn cache_save(thresh: usize, excluded: &Set<User>) -> anyhow::Result<()> {
    fs::create_dir_all(Self::cache_dir())?;
    let file = File::create(Self::excluded_cache_path(thresh))?;
    let mut writer = csv::Writer::from_writer(file);
    for &User { github_id } in excluded {
      writer.serialize(UserCsvEntry { github_id })?;
    }

    Ok(())
  }

  pub fn load_limited(
    limit: Option<usize>,
    user_exclude_contributions_thresh: Option<usize>,
  ) -> anyhow::Result<Self> {
    if let Some(excluded) =
      user_exclude_contributions_thresh.and_then(Self::cache_lookup)
    {
      let excluded = excluded?;
      return Self::load_limited_exclude(limit, &excluded);
    }
    let out = Self::load_limited_exclude(limit, &Default::default());
    if let Some(thresh) = user_exclude_contributions_thresh {
      let out = out?;
      let excluded: Set<_> = out
        .user_contributions()
        .iter()
        .zip(out.users())
        .filter_map(|(contribs, &user)| {
          let total_contribs = contribs
            .iter()
            .map(|&contrib_idx| out.contributions()[contrib_idx].num as usize)
            .sum::<usize>();

          if total_contribs >= thresh {
            Some(user)
          } else {
            None
          }
        })
        .collect();

      if limit.is_none() {
        Self::cache_save(thresh, &excluded)?;
      }

      if excluded.is_empty() {
        return Ok(out);
      }

      // this is inefficient, but saves memory
      drop(out);
      Self::load_limited_exclude(limit, &excluded)
    } else {
      out
    }
  }
}

#[cfg(test)]
fn strat_contributions(
  user: impl Strategy<Value = User>,
  repo: impl Strategy<Value = Repo>,
  num: impl Strategy<Value = u32> + 'static + Clone,
  size: impl Into<proptest::collection::SizeRange>,
) -> impl Strategy<Value = impl IntoIterator<Item = ContributionInput>> {
  proptest::collection::btree_set((user, repo), size).prop_flat_map(move |v| {
    proptest::collection::vec(num.clone(), v.len()).prop_map(move |nums| {
      v.clone()
        .into_iter()
        .zip(nums)
        .map(|((user, repo), num)| ContributionInput { user, repo, num })
    })
  })
}

#[cfg(test)]
pub fn strategy(
  num_users: impl Strategy<Value = github_api::ID>,
  num_repos: impl Strategy<Value = github_api::ID>,
  contribution_num: impl Strategy<Value = u32> + 'static + Clone,
  num_contribution: impl Into<proptest::collection::SizeRange> + Clone,
) -> impl Strategy<Value = Dataset> {
  (num_users, num_repos).prop_flat_map(move |(num_users, num_repos)| {
    strat_contributions(
      (0..num_users).prop_map(GithubIDWrapper::from_github_id),
      (0..num_repos).prop_map(GithubIDWrapper::from_github_id),
      contribution_num.clone(),
      num_contribution.clone(),
    )
    .prop_map(move |contributions| {
      Dataset::new(
        (0..num_users)
          .map(|github_id| (User { github_id }, format!("user_{}", github_id))),
        (0..num_repos)
          .map(|github_id| (Repo { github_id }, format!("repo_{}", github_id))),
        contributions,
        true,
      )
    })
  })
}
