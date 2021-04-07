use super::{
  get_token, NodeIDWrapper, RepoNotFoundError, UnexpectedNullError,
  API_COUNT_LIMIT, GITHUB_GRAPHQL_ENDPOINT,
};
use crate::db;
use crate::Repo;
use anyhow::{anyhow, Result};
use graphql_client::GraphQLQuery;
#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;

type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "graphql/github_schema.graphql",
  query_path = "graphql/query_repo_dependencies.graphql",
  response_derives = "Debug"
)]
struct RepoDependencies;

struct DependencyIterator {
  conn: diesel::PgConnection,
  manifests_after: Option<String>,
  dependencies_after: Option<String>,
  submodules_after: Option<String>,
  queried_repos: HashSet<Repo>,
  node_ids: Vec<String>,
  current_page: Vec<Dependency>,
  current_submodules: Vec<String>,
  current_submodule_from_repos: Vec<Repo>,
  api_token: String,
  finished: bool,
}

#[derive(Hash, Ord, PartialOrd, PartialEq, Eq, Debug, Clone)]
pub struct Dependency {
  pub from_repo: Repo,
  pub to_repo: Repo,
  // TODO: should this be an enum?
  pub package_manager: Option<String>,
}

const GIT_SUBMODULE_MANAGER: &'static str = "git_submodule";

impl DependencyIterator {
  fn next_result(&mut self) -> Result<Option<Dependency>> {
    // TODO: watch out for this loop not terminating!!!
    loop {
      assert_eq!(
        self.current_submodules.len(),
        self.current_submodule_from_repos.len()
      );
      for (&to_repo, &from_repo) in
        db::get_repos_from_names(&self.conn, &self.current_submodules)?
          .iter()
          .zip(&self.current_submodule_from_repos)
      {
        self.current_page.push(Dependency {
          to_repo,
          from_repo,
          package_manager: Some(GIT_SUBMODULE_MANAGER.to_owned()),
        });
      }

      self.current_submodules.clear();
      self.current_submodule_from_repos.clear();

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
    let q = RepoDependencies::build_query(repo_dependencies::Variables {
      manifests_after,
      manifests_count,
      dependencies_after: self
        .dependencies_after
        .clone()
        .unwrap_or("".to_owned()),
      submodules_after,
      submodules_count,
      ids: self.node_ids.clone(),
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

    for node in response_data.nodes {
      let node = node.as_ref().ok_or(RepoNotFoundError {})?;
      use repo_dependencies::RepoDependenciesNodesOn::*;
      if let Repository(repo) = &node.on {
        let from_repo = Repo::from_node_id(&repo.id)?;
        if !self.queried_repos.contains(&from_repo) {
          return Err(anyhow!("repo id wasn't one of the queried ids"));
        }
        self.handle_manifests(from_repo, &repo)?;
        self.handle_submodules(from_repo, &repo)?;
      } else {
        return Err(RepoNotFoundError {}.into());
      }
    }

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
    from_repo: Repo,
    repo: &repo_dependencies::RepoDependenciesNodesOnRepository,
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
        let to_repo = if let Some(repo) = &depend.repository {
          Repo::from_node_id(&repo.id)?
        } else {
          continue;
        };
        self.current_page.push(Dependency {
          package_manager: depend.package_manager.clone(),
          from_repo,
          to_repo,
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
    from_repo: Repo,
    repo: &repo_dependencies::RepoDependenciesNodesOnRepository,
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

      self
        .current_submodules
        .push(format!("{}/{}", owner, git_url.name));
      self.current_submodule_from_repos.push(from_repo);
    }

    self.finished = self.finished && !submodules.page_info.has_next_page;
    self.submodules_after = submodules.page_info.end_cursor.clone();

    Ok(())
  }

  fn null_err(s: &str) -> UnexpectedNullError {
    UnexpectedNullError(s.to_owned())
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

pub fn get_repo_dependencies(repos: &[Repo]) -> impl Iterator<Item = IterItem> {
  // TODO: dedup code
  DependencyIterator {
    conn: db::establish_connection(),
    manifests_after: Some("".to_owned()),
    dependencies_after: Some("".to_owned()),
    submodules_after: Some("".to_owned()),
    queried_repos: repos.iter().cloned().collect(),
    node_ids: repos.iter().map(Repo::as_node_id).collect(),
    current_page: Vec::new(),
    current_submodules: Vec::new(),
    current_submodule_from_repos: Vec::new(),
    api_token: get_token(),
    finished: false,
  }
}

#[test]
fn repo_not_found() -> Result<()> {
  let mut iter = get_repo_dependencies(&[Repo { github_id: 0 }]);

  let next = iter.next();
  assert!(next.is_some());
  let next = next.unwrap();
  assert!(next.is_err());
  let next_err = next.unwrap_err();
  assert!(iter.next().is_none());

  crate::check_error(next_err, &RepoNotFoundError {})
}

#[test]
fn single_submodule() -> Result<()> {
  let from_repo = super::get_repo(
    "rgreenblatt".to_owned(),
    "repo_with_single_submodule".to_owned(),
  )?;
  let mut iter = get_repo_dependencies(&[from_repo]);
  assert_eq!(
    iter.next().unwrap()?,
    Dependency {
      package_manager: Some(GIT_SUBMODULE_MANAGER.to_owned()),
      from_repo,
      to_repo: super::get_repo(
        "octokit".to_owned(),
        "graphql-schema".to_owned()
      )?,
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
  let from_repo = super::get_repo(owner.to_owned(), name.to_owned())?;
  let all =
    get_repo_dependencies(&[from_repo]).collect::<Result<Vec<Dependency>>>()?;

  assert_eq!(all.len(), expected_count);

  for depend in &all {
    assert_eq!(depend.from_repo, from_repo);
  }

  if let Some(expected_items) = expected_items {
    let conn = db::establish_connection();

    let mut map = HashMap::new();
    for depend in &all {
      *map.entry(&depend.to_repo).or_insert(0) += 1;
    }

    assert_eq!(map.len(), expected_items.len());

    for (owner_name, count) in expected_items {
      let to_repo =
        db::get_repos_from_names(&conn, &[owner_name.to_owned()])?[0];

      assert_eq!(map.get(&to_repo).unwrap(), &count);
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

// TODO: renable when db::get_repos_from_names is actually implemented!
// #[test]
// fn many_pages_of_submodules() -> Result<()> {
//   gen_test(
//     "rgreenblatt",
//     "repo_with_many_pages_of_submodules",
//     380,
//     Some(vec![("rgreenblatt/repo_with_many_submodules", 380)]),
//   )
// }

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

// NOTE: the exact numbers on this test aren't important (this might change as
// packages are shifted around etc...)
#[test]
fn many_pages_of_dependencies() -> Result<()> {
  gen_test(
    "rgreenblatt",
    "repo_with_many_pages_of_dependencies",
    370,
    None,
  )
}

#[test]
fn multiple_repos_with_many_pages_of_dependencies() -> Result<()> {
  let count = 5;
  let repos = vec![
    super::get_repo(
      "rgreenblatt".to_owned(),
      "repo_with_many_pages_of_dependencies".to_owned()
    )?;
    count
  ];

  let depends =
    get_repo_dependencies(&repos).collect::<Result<Vec<Dependency>>>()?;

  assert_eq!(depends.len(), 370 * count);

  Ok(())
}

// Github limits the number of manifests, so this is the most I could find.
// We just check that we don't error.
#[test]
fn many_manifests() -> Result<()> {
  let repo = super::get_repo("gimlichael".to_owned(), "Cuemon".to_owned())?;
  get_repo_dependencies(&[repo]).collect::<Result<Vec<Dependency>>>()?;

  Ok(())
}

// TODO: renable when db::get_repos_from_names is actually implemented!
// NOTE: the exact numbers on this test aren't important (this might change as
// packages are shifted around etc...)
// #[test]
// fn many_pages_of_everything() -> Result<()> {
//   gen_test(
//     "rgreenblatt",
//     "repo_with_many_pages_of_submodules_and_dependencies",
//     370 + 380,
//     None,
//   )
// }
