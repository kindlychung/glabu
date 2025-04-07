use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectSearchResponse {
    pub data: SearchData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchData {
    pub projects: SearchProjects,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchProjects {
    pub count: u32,
    pub nodes: Vec<SearchProjectNode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchProjectNode {
    #[serde(rename = "fullPath")]
    pub full_path: String,
    pub description: Option<String>,
    #[serde(rename = "webUrl")]
    pub web_url: String,
    #[serde(rename = "sshUrlToRepo")]
    pub ssh_url_to_repo: String,
}
