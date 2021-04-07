use github_net::github_api::{get_repo_names, ID};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "get_repo_name", about = "print out repo name given an id")]
struct Opt {
  id: ID,
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  println!("{:?}", get_repo_names(&[opt.id])?[0]);

  Ok(())
}
