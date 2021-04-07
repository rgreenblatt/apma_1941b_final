use github_net::{
  github_api::{get_user_logins, ID},
  User,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "get_user_login", about = "print out user name given an id")]
struct Opt {
  github_id: ID,
}

pub fn main() -> anyhow::Result<()> {
  let Opt { github_id } = Opt::from_args();

  println!("{:?}", get_user_logins(&[User { github_id }])?[0]);

  Ok(())
}
