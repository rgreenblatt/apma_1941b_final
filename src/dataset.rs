use crate::{
  csv_items::{
    get_csv_list_paths, ContributionCsvEntry, RepoNameCsvEntry,
    UserLoginCsvEntry,
  },
  csv_items_iter::csv_items_iter,
  github_api, EdgeVec, HasGithubID, ItemType, Repo, User, UserRepoPair,
};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use std::{collections::HashMap, hash::Hash};
use unzip_n::unzip_n;

#[derive(Clone, Copy, Debug)]
pub struct Contribution {
  pub idx: UserRepoPair<usize>,
  pub num: i32,
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

type CollectedItems<T> = (Vec<T>, Vec<String>, HashMap<T, usize>);

#[derive(Clone, Copy, Debug)]
pub struct ContributionInput {
  pub user: User,
  pub repo: Repo,
  pub num: i32,
}

impl Dataset {
  pub fn users(&self) -> &[User] {
    &self.users_v
  }

  pub fn repos(&self) -> &[Repo] {
    &self.repos_v
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

  pub fn names(&self) -> &UserRepoPair<Vec<String>> {
    &self.names_v
  }

  pub fn contribution_idxs(&self) -> &UserRepoPair<EdgeVec<usize>> {
    &self.contribution_idxs_v
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

  pub fn load_limited(limit: Option<usize>) -> anyhow::Result<Self> {
    let items = get_csv_list_paths();

    let get_bar = || {
      let bar = ProgressBar::new(!0);
      bar.set_style(
        ProgressStyle::default_bar()
          .template("[{elapsed_precise}] {pos} {per_sec}"),
      );
      bar.set_draw_delta(100_000);
      bar
    };

    let user_iter = csv_items_iter(items.user_login_csv_list)?
      .progress_with(get_bar())
      .map(|v| {
        v.map(|UserLoginCsvEntry { github_id, login }| {
          (User { github_id }, login)
        })
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
      Self::new_error(user_iter, repo_iter, contributions_iter, true)
    }
  }

  pub fn load() -> anyhow::Result<Self> {
    Self::load_limited(None)
  }
}
