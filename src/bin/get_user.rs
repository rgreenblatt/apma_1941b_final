use github_net::github_api::get_user;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "user_name", about = "print out user id given a login")]
struct Opt {
  login: String,
}

pub fn main() -> anyhow::Result<()> {
  let Opt { login } = Opt::from_args();

  println!("id is {}", get_user(login)?.github_id);

  Ok(())
}
