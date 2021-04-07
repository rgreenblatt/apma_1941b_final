use github_net::github_api::get_repo;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "repo_name",
  about = "print out repo id given an owner and name"
)]
struct Opt {
  owner: String,
  name: String,
}

pub fn main() -> anyhow::Result<()> {
  let Opt { owner, name } = Opt::from_args();

  println!("id is {}", get_repo(owner, name)?.github_id);

  Ok(())
}
