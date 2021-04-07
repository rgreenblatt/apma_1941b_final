use anyhow::{anyhow, Result};

pub fn make_request(q: Q) -> Result<()> {
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
}
