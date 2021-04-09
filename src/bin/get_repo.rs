use github_net::{
  db::{establish_connection, get_repos_from_names},
  github_api::get_repo,
};
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

  println!("id is {}", get_repo(owner.clone(), name.clone())?.github_id);
  let conn = establish_connection();
  println!(
    "id is {}",
    get_repos_from_names(&conn, &[format!("{}/{}", owner, name)])?[0]
      .unwrap()
      .github_id
  );

  Ok(())
}
