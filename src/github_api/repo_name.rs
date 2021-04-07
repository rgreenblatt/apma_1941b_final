use super::{
  as_node_id, from_node_id, get_token, ResponseError, GITHUB_GRAPHQL_ENDPOINT,
  ID,
};
use anyhow::{anyhow, Result};
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "github_schema.graphql",
  query_path = "query_repo_names.graphql",
  response_derives = "Debug"
)]
struct RepoNames;

#[derive(Debug, Clone)]
pub struct OwnerName {
  pub owner: String,
  pub name: String,
}

pub fn get_repo_names(ids: &[ID]) -> Result<Vec<OwnerName>> {
  // TODO: dedup code
  let ids = ids.iter().cloned().map(as_node_id).collect();

  let q = RepoNames::build_query(repo_names::Variables { ids });

  let client = reqwest::blocking::Client::builder()
    .user_agent("github_net/0.1.0")
    .build()?;

  let res = client
    .post(GITHUB_GRAPHQL_ENDPOINT)
    .bearer_auth(&get_token())
    .json(&q)
    .send()?;

  res.error_for_status_ref()?;

  let response_body: graphql_client::Response<repo_names::ResponseData> =
    res.json()?;

  let response_data =
    response_body.data.ok_or(anyhow!("missing response data"))?;

  response_data
    .nodes
    .iter()
    .map(|node| {
      let get_not_found = || ResponseError::RepoNotFound.into();
      let node = node.as_ref().ok_or_else(get_not_found)?;
      use repo_names::RepoNamesNodesOn::*;
      if let Repository(repo) = &node.on {
        let mut items = repo.name_with_owner.split('/');
        let get_err = || anyhow!("unexpected nameWithOwner format");
        let owner = items.next().ok_or_else(get_err)?.to_owned();
        let name = items.next().ok_or_else(get_err)?.to_owned();
        Ok(OwnerName { owner, name })
      } else {
        Err(get_not_found())
      }
    })
    .collect()
}

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "github_schema.graphql",
  query_path = "query_repo_id.graphql",
  response_derives = "Debug"
)]
struct RepoID;

pub fn get_repo_id(owner: String, name: String) -> Result<ID> {
  let q = RepoID::build_query(repo_id::Variables { owner, name });

  let client = reqwest::blocking::Client::builder()
    .user_agent("github_net/0.1.0")
    .build()?;

  let res = client
    .post(GITHUB_GRAPHQL_ENDPOINT)
    .bearer_auth(&get_token())
    .json(&q)
    .send()?;

  res.error_for_status_ref()?;

  let response_body: graphql_client::Response<repo_id::ResponseData> =
    res.json()?;

  let response_data =
    response_body.data.ok_or(anyhow!("missing response data"))?;

  let repo = response_data
    .repository
    .ok_or(ResponseError::RepoNotFound)?;

  from_node_id(&repo.id)
}
