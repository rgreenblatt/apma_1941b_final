use github_net::db;
use std::convert::TryInto;

pub fn main() -> anyhow::Result<()> {
  let conn = db::establish_connection();

  let repo_degrees = db::repo_degrees(&conn)?;
  let user_degrees = db::user_degrees(&conn)?;

  dbg!(repo_degrees.len());
  dbg!(user_degrees.len());

  let (min_repo_degree, min_repo_id) = *repo_degrees.iter().min().unwrap();
  let (max_repo_degree, max_repo_id) = *repo_degrees.iter().max().unwrap();

  dbg!(min_repo_id);
  dbg!(max_repo_id);

  let [min_repo_name, max_repo_name]: [String; 2] =
    db::get_repo_names(&conn, &[min_repo_id, max_repo_id])?
      .try_into()
      .unwrap();

  println!("min repo degree {} ({})", min_repo_degree, min_repo_name);
  println!("max repo degree {} ({})", max_repo_degree, max_repo_name);

  let (min_user_degree, min_user_id) = *user_degrees.iter().min().unwrap();
  let (max_user_degree, max_user_id) = *user_degrees.iter().max().unwrap();

  dbg!(min_user_id);
  dbg!(max_user_id);

  let [min_user_login, max_user_login]: [String; 2] =
    db::get_user_logins(&conn, &[min_user_id, max_user_id])?
      .try_into()
      .unwrap();

  println!("min user degree {} ({})", min_user_degree, min_user_login);
  println!("max user degree {} ({})", max_user_degree, max_user_login);

  Ok(())
}
