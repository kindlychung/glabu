use std::{collections::HashMap, sync::OnceLock};
use std::borrow::Borrow;

use reqwest::{Client as ReqwestClient, Url};
use urlencoding::encode;

static GITLAB_TOKEN: OnceLock<String> = OnceLock::new();

pub fn gitlab_token() -> &'static String {
    GITLAB_TOKEN.get_or_init(|| std::env::var("GITLAB_TOKEN").unwrap())
}

static GITLAB_HOST: OnceLock<String> = OnceLock::new();

pub fn gitlab_host() -> &'static String {
    GITLAB_HOST
        .get_or_init(|| std::env::var("GITLAB_HOST").unwrap_or("https://gitlab.com".to_string()))
}


pub fn gitlab_api_url_with_query<I, K, V>(path: &str, query_params: I) -> Result<Url, Box<dyn std::error::Error>> 
where
    I: IntoIterator,
    K: AsRef<str>,
    V: AsRef<str>,
    I::Item: Borrow<(K, V)>,
{
    let base_url = format!("{}/api/v4{}", gitlab_host(), path);
	Ok(Url::parse_with_params(&base_url, query_params)?)
}

pub fn gitlab_api_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> 
{
    let base_url = format!("{}/api/v4{}", gitlab_host(), path);
	Ok(Url::parse(&base_url)?)
}

static HTTPCLIENT: OnceLock<ReqwestClient> = OnceLock::new();
pub fn httpclient() -> &'static ReqwestClient {
    HTTPCLIENT.get_or_init(|| ReqwestClient::new())
}
