use github_net::github_api::{get_repo_id, ID};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "repo_name", about = "print out repo id given a name")]
struct Opt {
  owner: String,
  name: String,
}

pub fn main() -> anyhow::Result<()> {
  let Opt { owner, name } = Opt::from_args();

  println!("id is {}", get_repo_id(owner, name)?);

  Ok(())
}
