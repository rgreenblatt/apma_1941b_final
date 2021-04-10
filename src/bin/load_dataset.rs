use anyhow::Result;
use github_net::loaded_dataset::Dataset;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "load_dataset",
  about = "load the dataset directly into memory from .csv.gz files"
)]
struct Opt {
  // no options right now
}

pub fn main() -> Result<()> {
  let _ = Opt::from_args();

  let dataset = Dataset::load_limited(100000)?;

  let get_repo_name =
    |(c, idx): (usize, usize)| (c, dataset.repo_names[idx].clone());
  let get_user_login =
    |(c, idx): (usize, usize)| (c, dataset.user_logins[idx].clone());

  dbg!(get_user_login(
    dataset
      .user_contributions
      .iter()
      .enumerate()
      .map(|(i, v)| (v.len(), i))
      .max()
      .unwrap()
  ));
  dbg!(get_user_login(
    dataset
      .user_contributions
      .iter()
      .enumerate()
      .map(|(i, v)| (v.len(), i))
      .min()
      .unwrap()
  ));
  dbg!(get_repo_name(
    dataset
      .repo_contributions
      .iter()
      .enumerate()
      .map(|(i, v)| (v.len(), i))
      .max()
      .unwrap()
  ));
  dbg!(get_repo_name(
    dataset
      .repo_contributions
      .iter()
      .enumerate()
      .map(|(i, v)| (v.len(), i))
      .min()
      .unwrap()
  ));

  Ok(())
}
