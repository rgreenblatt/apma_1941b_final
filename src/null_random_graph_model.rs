use crate::{dataset::Contribution, UserRepoPair};
use rand::{
  distributions::{Bernoulli, Uniform},
  prelude::*,
};

pub fn gen_graph<R: RngCore>(
  num_users: usize,
  num_repos: usize,
  alpha: f64,
  beta: f64,
  rng: &mut R,
  mut num_edges_dist: impl FnMut(&mut R) -> usize,
) {
  let use_self_dist = Bernoulli::new(beta).unwrap();
  let use_regardless_dist = Bernoulli::new(alpha).unwrap();

  let repo_dist = Uniform::from(0..num_repos);
  let mut repo_edges = vec![Vec::new(); num_repos];
  let mut repo_contribution_totals = vec![num_repos; 0];
  let mut contributions = Vec::<Contribution>::new();

  let mut max_total_contributions = 0;

  let mut user_edges = vec![Vec::<usize>::new(); num_users];

  for (user_idx, user_edges) in user_edges.iter_mut().enumerate() {
    let start_contributions = contributions.len();

    let num_edges = num_edges_dist(rng); // TODO:

    let mut max_user_contributions = 0;

    for _ in 0..num_edges {
      let repo = if !user_edges.is_empty() && use_self_dist.sample(rng) {
        loop {
          let repo = Uniform::from(0..user_edges.len()).sample(rng);

          // rejection sampling
          let y = Uniform::from(0..=max_user_contributions).sample(rng);

          if contributions[user_edges[repo]].num > y {
            continue;
          }

          break repo;
        }
      } else {
        let use_regardless = use_regardless_dist.sample(rng);
        loop {
          let repo = repo_dist.sample(rng);

          if use_regardless {
            break repo;
          }

          // rejection sampling
          let y = Uniform::from(0..=max_total_contributions).sample(rng);

          if repo_contribution_totals[repo] > y {
            continue;
          }

          break repo;
        }
      };

      repo_contribution_totals[repo] += 1;

      max_total_contributions =
        max_total_contributions.max(repo_contribution_totals[repo]);

      for contrib in &mut contributions[start_contributions..] {
        if contrib.idx.repo == repo {
          contrib.num += 1;
          max_user_contributions = max_user_contributions.max(contrib.num);
        }
      }

      user_edges.push(contributions.len());
      repo_edges[repo].push(contributions.len());

      contributions.push(Contribution {
        num: 1,
        idx: UserRepoPair {
          user: user_idx,
          repo,
        },
      })
    }
  }
}
