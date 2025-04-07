use std::{collections::HashMap, sync::OnceLock};

use reqwest::Client as ReqwestClient;
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

pub fn gitlab_api_url(path: &str, query_params: Option<HashMap<&str, &str>>) -> String {
    let base_url = format!("{}/api/v4{}", gitlab_host(), path);

    match query_params {
        Some(params) if !params.is_empty() => {
            let query_string = params
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", base_url, query_string)
        }
        _ => base_url,
    }
}

static HTTPCLIENT: OnceLock<ReqwestClient> = OnceLock::new();
pub fn httpclient() -> &'static ReqwestClient {
    HTTPCLIENT.get_or_init(|| ReqwestClient::new())
}
