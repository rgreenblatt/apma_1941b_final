use github_net::{csv_items::DependencyCsvEntry, db, github_api};
use std::{fs::File, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
  name = "download_dependencies",
  about = "download all dependencies to a csv"
)]
struct Opt {
  #[structopt(parse(from_os_str))]
  out_path: PathBuf,
}

pub fn main() -> anyhow::Result<()> {
  let opt = Opt::from_args();

  let conn = db::establish_connection();

  let repos = db::get_repos(&conn, None)?;

  let mut csv_writer = csv::Writer::from_writer(File::create(opt.out_path)?);

  let count = repos.len();

  let mut total_count_err = 0;
  let mut total = 0;

  let chunk_size = 10;

  for repo_chunk in repos.chunks(chunk_size) {
    let repos: Vec<_> = repo_chunk
      .iter()
      .map(|&repo_entry| repo_entry.into())
      .collect();

    for depend in github_api::get_repo_dependencies(&repos) {
      let depend = match depend {
        Ok(depend) => depend,
        Err(err) => {
          total_count_err += 1;
          match err.downcast_ref::<github_api::RepoNotFoundError>() {
            Some(_) => {
              continue;
            }
            None => {
              dbg!(err);
              continue;
            }
          }
        }
      };
      csv_writer.serialize(DependencyCsvEntry {
        from_repo_github_id: depend.from_repo.github_id,
        to_repo_github_id: depend.to_repo.github_id,
        package_manager: depend.package_manager,
      })?;
    }

    total += chunk_size;
    println!("{} / {}", total, count);
  }

  dbg!(total_count_err);

  Ok(())
}
