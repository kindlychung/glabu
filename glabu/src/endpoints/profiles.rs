use std::{borrow::Borrow, error::Error};

use crate::{endpoints::setup::gitlab_api_url, models::{Group, User}};

use super::setup::{gitlab_api_url_with_query, gitlab_token, httpclient};

/// Fetch the current user's information from GitLab.
pub async fn me() -> Result<User, Box<dyn std::error::Error>> {
    let response = httpclient()
        .get(gitlab_api_url("/user", )?)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let json = response.text().await?;
    eprintln!("me json: {}......", &json[0..30]);
    let user = serde_json::from_str(&json)?;
    Ok(user)
}

/// Fetch groups info owned by current user from GitLab
async fn groups_get_helper<I, K, V>(
    path: &str,
    query: I,
) -> Result<Vec<u8>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    K: AsRef<str>,
    V: AsRef<str>,
    I::Item: Borrow<(K, V)>,
{
	let url = gitlab_api_url_with_query(&format!(
		"/groups{}",
		path
	), query)?;
    let response = httpclient()
        .get(url)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let json_bytes = response.bytes().await?.to_vec();
    return Ok(json_bytes);
}

pub async fn groups_get<I, K, V>(query: I) -> Result<Vec<Group>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    K: AsRef<str>,
    V: AsRef<str>,
    I::Item: Borrow<(K, V)>,
{
    let json = groups_get_helper("", query).await?;
    let gs: Vec<Group> = serde_json::from_slice(&json)?;
    return Ok(gs);
}

/// Get group id by name
pub async fn group_by_name(group_name: &str) -> Result<Group, Box<dyn std::error::Error>> {
    eprintln!("group_by_name: {}", group_name);
    let json = groups_get_helper(&format!("/{}", group_name), &[("", "")]).await?;
    let group: Group = serde_json::from_slice(&json)?;
    Ok(group)
}

/// Get group name by id
pub async fn group_by_id(id: u64) -> Result<Group, Box<dyn Error>> {
    group_by_name(id.to_string().as_str()).await
}
