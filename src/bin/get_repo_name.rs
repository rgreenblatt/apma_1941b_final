use github_net::{
  github_api::{get_repo_names, ID},
  Repo,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "get_repo_name", about = "print out repo name given an id")]
struct Opt {
  github_id: ID,
}

pub fn main() -> anyhow::Result<()> {
  let Opt { github_id } = Opt::from_args();

  println!("{:?}", get_repo_names(&[Repo { github_id }])?[0]);

  Ok(())
}
