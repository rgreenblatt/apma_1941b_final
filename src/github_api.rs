use crate::Repo;
use anyhow::{anyhow, Result};
use graphql_client::GraphQLQuery;
#[cfg(test)]
use std::collections::HashMap;
use std::{convert::TryInto, error::Error, fmt};

type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "github_schema.graphql",
  query_path = "query_repo_dependencies.graphql",
  response_derives = "Debug"
)]
struct RepoDependencies;

#[derive(Debug)]
struct DependencyIterator {
  manifests_after: Option<String>,
  dependencies_after: Option<String>,
  submodules_after: Option<String>,
  owner: String,
  name: String,
  current_page: Vec<Dependency>,
  api_token: String,
  finished: bool,
}

#[derive(Hash, Ord, PartialOrd, PartialEq, Eq, Debug, Clone)]
pub struct Dependency {
  // TODO: should this be an enum
  pub package_manager: Option<String>,
  pub repo: Repo,
}

#[derive(PartialEq, Eq, Debug)]
pub enum RepoDependencyResponseError {
  RepoNotFound,
  UnexpectedNull(String),
}

impl fmt::Display for RepoDependencyResponseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::RepoNotFound => {
        write!(f, "repo not found")
      }
      Self::UnexpectedNull(s) => {
        write!(f, "{} was null", s)
      }
    }
  }
}

impl Error for RepoDependencyResponseError {}

const GITHUB_GRAPHQL_ENDPOINT: &'static str = "https://api.github.com/graphql";
const GIT_SUBMODULE_MANAGER: &'static str = "git_submodule";
const API_COUNT_LIMIT: i64 = 100;

impl DependencyIterator {
  fn next_result(&mut self) -> Result<Option<Dependency>> {
    // TODO: watch out for this loop not terminating!!!
    loop {
      if let Some(next) = self.current_page.pop() {
        return Ok(Some(next));
      }

      if self.finished {
        return Ok(None);
      }

      match self.next_page() {
        Err(e) => {
          self.finished = true;
          return Err(e);
        }
        _ => {}
      }
    }
  }

  fn next_page(&mut self) -> Result<()> {
    let (manifests_after, manifests_count) =
      Self::after_count(&self.manifests_after);
    let (submodules_after, submodules_count) =
      Self::after_count(&self.submodules_after);
    dbg!(&self);
    let q = RepoDependencies::build_query(repo_dependencies::Variables {
      manifests_after,
      manifests_count,
      dependencies_after: self
        .dependencies_after
        .clone()
        .unwrap_or("".to_owned()),
      submodules_after,
      submodules_count,
      owner: self.owner.clone(),
      name: self.name.clone(),
    });

    let client = reqwest::blocking::Client::builder()
      .user_agent("github_net/0.1.0")
      .build()?;

    let res = client
      .post(GITHUB_GRAPHQL_ENDPOINT)
      .bearer_auth(&self.api_token)
      // extra header for preview
      .header(
        reqwest::header::ACCEPT,
        "application/vnd.github.hawkgirl-preview+json",
      )
      .json(&q)
      .send()?;

    res.error_for_status_ref()?;

    let response_body: graphql_client::Response<
      repo_dependencies::ResponseData,
    > = res.json()?;

    let response_data =
      response_body.data.ok_or(anyhow!("missing response data"))?;

    let repo = response_data
      .repository
      .ok_or(RepoDependencyResponseError::RepoNotFound)?;

    self.handle_manifests(&repo)?;
    self.handle_submodules(&repo)?;

    Ok(())
  }

  fn after_count(after: &Option<String>) -> (String, i64) {
    after
      .as_ref()
      .map(|s| (s.clone(), API_COUNT_LIMIT))
      .unwrap_or(("".to_owned(), 0))
  }

  fn handle_manifests(
    &mut self,
    repo: &repo_dependencies::RepoDependenciesRepository,
  ) -> Result<()> {
    let manifests = repo
      .dependency_graph_manifests
      .as_ref()
      .ok_or(Self::null_err("manifests"))?;

    let nodes = manifests
      .nodes
      .as_ref()
      .ok_or(Self::null_err("manifest nodes"))?;

    self.finished = true;
    self.dependencies_after = None;

    for node in nodes {
      let node = node.as_ref().ok_or(Self::null_err("manifest node"))?;

      let dependencies = node
        .dependencies
        .as_ref()
        .ok_or(Self::null_err("dependencies"))?;

      let dep_nodes = dependencies
        .nodes
        .as_ref()
        .ok_or(Self::null_err("dependency nodes"))?;
      for depend in dep_nodes {
        let depend = depend.as_ref().ok_or(Self::null_err("dependency"))?;
        let repo = if let Some(repo) = &depend.repository {
          repo.name_with_owner.clone().try_into()?
        } else {
          continue;
        };
        self.current_page.push(Dependency {
          package_manager: depend.package_manager.clone(),
          repo,
        })
      }
      if dependencies.page_info.has_next_page {
        self.finished = false;
        let next_dependencies_after = dependencies
          .page_info
          .end_cursor
          .clone()
          .ok_or(Self::null_err(
            "dependency end cursor was null with a next page!",
          ))?;
        if let Some(dependencies_after) = &self.dependencies_after {
          if dependencies_after != &next_dependencies_after {
            return Err(anyhow!("dependencies must have same next page!"));
          }
        }
        self.dependencies_after = Some(next_dependencies_after);
      }
    }

    if self.dependencies_after.is_none() {
      self.finished = self.finished && !manifests.page_info.has_next_page;
      self.manifests_after = manifests.page_info.end_cursor.clone();
    }

    Ok(())
  }

  fn handle_submodules(
    &mut self,
    repo: &repo_dependencies::RepoDependenciesRepository,
  ) -> Result<()> {
    let submodules = &repo.submodules;

    for node in submodules
      .nodes
      .as_ref()
      .ok_or(Self::null_err("submodule nodes"))?
    {
      let node = node.as_ref().ok_or(Self::null_err("submodule node"))?;
      let git_url = git_url_parse::GitUrl::parse(&node.git_url)?;
      match git_url.host {
        Some(s) if &s == "github.com" => {}
        _ => continue,
      }

      let owner: &str = git_url
        .owner
        .as_ref()
        .ok_or(anyhow!("submodule url was parsed with missing owner!"))?;

      self.current_page.push(Dependency {
        package_manager: Some(GIT_SUBMODULE_MANAGER.to_owned()),
        repo: Repo(format!("{}/{}", owner, git_url.name)),
      });
    }

    self.finished = self.finished && !submodules.page_info.has_next_page;
    self.submodules_after = submodules.page_info.end_cursor.clone();

    Ok(())
  }

  fn null_err(s: &str) -> RepoDependencyResponseError {
    RepoDependencyResponseError::UnexpectedNull(s.to_owned())
  }
}

type IterItem = Result<Dependency>;

impl Iterator for DependencyIterator {
  type Item = IterItem;

  fn next(&mut self) -> Option<Self::Item> {
    match self.next_result() {
      Ok(Some(value)) => Some(Ok(value)),
      Err(err) => Some(Err(err)),
      Ok(None) => None,
    }
  }
}

pub fn get_repo_dependencies<'a>(
  owner: &str,
  name: &str,
) -> impl Iterator<Item = IterItem> {
  dotenv::dotenv().ok();

  let api_token =
    std::env::var("GITHUB_API_TOKEN").expect("GITHUB_API_TOKEN must be set");

  DependencyIterator {
    manifests_after: Some("".to_owned()),
    dependencies_after: Some("".to_owned()),
    submodules_after: Some("".to_owned()),
    owner: owner.to_owned(),
    name: name.to_owned(),
    current_page: Vec::new(),
    api_token,
    finished: false,
  }
}

#[test]
fn repo_not_found() -> Result<()> {
  let mut iter = get_repo_dependencies(
    "rgreenblatt",
    "a_repo_which_certainly_does_not_exist",
  );

  let next = iter.next();
  assert!(next.is_some());
  let next = next.unwrap();
  assert!(next.is_err());
  let next_err = next.unwrap_err();

  assert_eq!(
    match next_err.downcast_ref::<RepoDependencyResponseError>() {
      Some(err) => err,
      None => return Err(next_err).into(),
    },
    &RepoDependencyResponseError::RepoNotFound,
  );
  assert!(iter.next().is_none());

  Ok(())
}

#[test]
fn single_submodule() -> Result<()> {
  let mut iter =
    get_repo_dependencies("rgreenblatt", "repo_with_single_submodule");
  assert_eq!(
    iter.next().unwrap()?,
    Dependency {
      package_manager: Some(GIT_SUBMODULE_MANAGER.to_owned()),
      repo: ("octokit", "graphql-schema").try_into().unwrap(),
    }
  );
  assert!(iter.next().is_none());

  Ok(())
}

#[cfg(test)]
fn gen_test(
  owner: &str,
  name: &str,
  expected_count: usize,
  expected_items: Option<Vec<(&'static str, usize)>>,
) -> Result<()> {
  let all =
    get_repo_dependencies(owner, name).collect::<Result<Vec<Dependency>>>()?;

  assert_eq!(all.len(), expected_count);

  let mut map = HashMap::new();
  for depend in &all {
    *map.entry(depend.repo.owner_name().to_owned()).or_insert(0) += 1;
  }

  if let Some(expected_items) = expected_items {
    assert_eq!(map.len(), expected_items.len());

    for (owner_name, count) in expected_items {
      assert_eq!(map.get(owner_name).unwrap(), &count);
    }
  }

  Ok(())
}

#[test]
fn single_ssh_submodule() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_single_ssh_submodule",
    1,
    Some(vec![("octokit/graphql-schema", 1)]),
  )
}

#[test]
fn many_submodules() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_many_submodules",
    6,
    Some(vec![
      ("rgreenblatt/repo_with_many_submodules", 1),
      ("rgreenblatt/repo_with_single_submodule", 1),
      ("octokit/graphql-schema", 3),
      ("numpy/numpy", 1),
    ]),
  )
}

#[test]
fn many_pages_of_submodules() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_many_pages_of_submodules",
    380,
    Some(vec![("rgreenblatt/repo_with_many_submodules", 380)]),
  )
}

#[test]
fn single_dependency() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_single_dependency",
    1,
    Some(vec![("flori/json", 1)]),
  )
}

#[test]
fn many_dependencies() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_many_dependencies",
    10,
    Some(vec![
      ("mongodb/bson-ruby", 1),
      ("flori/json", 2),
      ("whitequark/ast", 2),
      ("aws/aws-sdk-ruby", 3),
      ("bcrypt-ruby/bcrypt-ruby", 1),
      ("jimweirich/builder", 1),
    ]),
  )
}

// NOTE: the exact numbers on this test aren't important (this might
// changes as packages are shifted around etc...)
#[test]
fn many_pages_of_dependencies() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_many_pages_of_dependencies",
    370,
    None,
  )
}

// Github limits the number of manifests, so this is the most I could find.
// We just check that we don't error.
#[test]
fn many_manifests() -> Result<()> {
  get_repo_dependencies("gimlichael", "Cuemon")
    .collect::<Result<Vec<Dependency>>>()?;

  Ok(())
}

// NOTE: the exact numbers on this test aren't important (this might
// changes as packages are shifted around etc...)
#[test]
fn many_pages_of_everything() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_many_pages_of_submodules_and_dependencies",
    370 + 380,
    None,
  )
}
