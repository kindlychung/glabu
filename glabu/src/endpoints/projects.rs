use super::profiles::{group_by_id, group_by_name, me};
use std::borrow::Borrow;
use super::setup::{gitlab_api_url_with_query, gitlab_token, httpclient};
use crate::endpoints::setup::gitlab_api_url;
use crate::models::{Project, ProjectPushMirrorPayload, ProjectVisibility};
use crate::models::{ProjectCreatePayload, ProjectSearchResponse};
use reqwest::header;
use std::process::Command;
use urlencoding::encode;

#[derive(Debug, Clone)]
pub struct ProjectCreate {
    pub name: String,
    pub namespace_id: Option<u64>,
    pub description: Option<String>,
    pub visibility: ProjectVisibility,
    pub initialize_with_readme: Option<bool>,
}

impl Into<ProjectCreatePayload> for ProjectCreate {
    fn into(self) -> ProjectCreatePayload {
        ProjectCreatePayload {
            name: self.name,
            namespace_id: self.namespace_id,
            description: self.description.or(Some("".to_string())),
            visibility: self.visibility,
            initialize_with_readme: self.initialize_with_readme.or(Some(false)),
        }
    }
}

impl ProjectCreate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            namespace_id: None,
            description: Some("".to_string()),
            visibility: ProjectVisibility::Private,
            initialize_with_readme: Some(false),
        }
    }
    pub async fn for_group(
        name: &str,
        group_name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            name: name.to_string(),
            namespace_id: Some(group_by_name(group_name).await?.id),
            description: Some("".to_string()),
            visibility: ProjectVisibility::Private,
            initialize_with_readme: Some(false),
        })
    }
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    pub fn visibility(mut self, visibility: ProjectVisibility) -> Self {
        self.visibility = visibility;
        self
    }
    pub fn namespace_id(mut self, namespace_id: u64) -> Self {
        self.namespace_id = Some(namespace_id);
        self
    }
    pub fn initialize_with_readme(mut self, initialize_with_readme: bool) -> Self {
        self.initialize_with_readme = Some(initialize_with_readme);
        self
    }
    pub async fn run(self, mirror_to_github: bool) -> Result<Project, Box<dyn std::error::Error>> {
        let me = me().await?;
        // namespace of the project/repo could be the user's username or a group name
        let namespace = match self.namespace_id {
            Some(namespace_id) => group_by_id(namespace_id).await?.name.clone(),
            None => me.username.clone(),
        };
        eprintln!("namespace: {}", &namespace);
        let full_name = format!("{}/{}", namespace, self.name);
        eprintln!("checking if project already exists: {}", &full_name);
        let mut proj = match project_get(&full_name).await {
            Ok(res) => {
                eprintln!("Project already exists: {}", &full_name);
                Some(res)
            }
            Err(_) => {
                eprintln!("Project does not exist, creating: {}", &full_name);
                None
            }
        };
        if proj.is_none() {
            let payload: ProjectCreatePayload = self.clone().into();
            eprintln!("payload: {:?}", &payload);
            let payload_str = serde_json::to_string(&payload).unwrap();
            eprintln!("payload: {}", &payload_str[0..30]);
            let response = httpclient()
                .post(gitlab_api_url("/projects", )?)
                .header("Private-Token", gitlab_token())
                .json(&payload)
                .send()
                .await?;
            let json_str = response.text().await?;
            eprintln!("parsing project json: {}", &json_str[0..30]);
            proj = Some(serde_json::from_str(&json_str)?);
        }
        let proj = proj.expect("project should exist");
        if mirror_to_github {
            let gh_repo = ghu::repo_create(
                self.name.as_str(),
                self.description.as_ref().unwrap_or(&"".to_string()),
                self.visibility == ProjectVisibility::Public,
            )
            .await?;
            eprintln!(
                "repo on gitlab: {}\nrepo on github: {}",
                &proj.path_with_namespace,
                &gh_repo.full_name.unwrap_or_default()
            );
            let remote_url_with_cred = ghu::repo_link_with_cred(&gh_repo.name).await?;
            ProjectPushMirror::new(proj.id, &remote_url_with_cred)
                .run()
                .await?;
        }
        return Ok(proj);
    }
}

pub async fn project_get(name: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let response = httpclient()
        .get(gitlab_api_url( &format!("/projects/{}", encode(&name)),)?) 
			.header("Private-Token", gitlab_token())
        .send()
        .await?;
    let status = response.status();
    eprintln!("status of getting project {}: {}", name, status);
    if status == 404 {
        return Err("NotFound".into());
    }
    let json_str = response.text().await?;
    eprintln!("parsing json : {}", &json_str[0..30]);
    let res: Project = serde_json::from_str(&json_str)?;
    Ok(res)
}

pub struct ProjectDelete {
    pub full_name: String,
}

impl ProjectDelete {
    pub async fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let full_name = if !name.contains("/") {
            let me = me().await?;
            format!("{}/{}", me.username, name)
        } else {
            name.to_string()
        };
        Ok(Self { full_name })
    }
    pub async fn for_group(group: &str, repo: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // check if group exists
        let _ = group_by_name(group).await?;
        Ok(Self {
            full_name: format!("{}/{}", group, repo),
        })
    }
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = httpclient()
            .delete(gitlab_api_url(
                &format!("/projects/{}", encode(&self.full_name)),
            )?)
            .header("Private-Token", gitlab_token())
            .send()
            .await?;
        let status = response.status();
        eprintln!("status of deleting project {}: {}", &self.full_name, status);
        if let Err(e) = response.error_for_status_ref() {
            return Err(e.into());
        }
        let content = response.text().await?;
        println!("{}", content);
        Ok(())
    }
}

pub struct ProjectPushMirror {
    pub project_id: u64,
    pub remote_url_with_cred: String,
    pub enabled: bool,
    pub only_protected_branches: bool,
    pub keep_divergent_refs: bool,
}

impl Into<ProjectPushMirrorPayload> for ProjectPushMirror {
    fn into(self) -> ProjectPushMirrorPayload {
        ProjectPushMirrorPayload {
            url: self.remote_url_with_cred,
            enabled: self.enabled,
            only_protected_branches: self.only_protected_branches,
            keep_divergent_refs: self.keep_divergent_refs,
        }
    }
}

impl ProjectPushMirror {
    pub fn new(project_id: u64, remote_url_with_cred: &str) -> Self {
        Self {
            project_id,
            remote_url_with_cred: remote_url_with_cred.to_string(),
            enabled: true,
            only_protected_branches: false,
            keep_divergent_refs: false,
        }
    }
    pub async fn from_repo_path(
        repo_path: &str,
        remote_url_with_cred: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let repo = project_get(repo_path).await?;
        Ok(Self::new(repo.id, remote_url_with_cred))
    }
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_id = self.project_id;
        let body: ProjectPushMirrorPayload = self.into();
        let api_url = gitlab_api_url(&format!("/projects/{}/remote_mirrors", repo_id), 
)?;
        let response = httpclient()
            .post(api_url)
            .header("Private-Token", gitlab_token())
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        if status == 404 {
            return Err("NotFound".into());
        }
        Ok(())
    }
}

pub struct ProjectSearch {
    pub search_term: String,
}

impl ProjectSearch {
    pub fn new(search_term: &str) -> Self {
        Self {
            search_term: search_term.replace("\"", " ").to_string(),
        }
    }
    pub async fn run(&self) -> Result<ProjectSearchResponse, Box<dyn std::error::Error>> {
        // Construct the GraphQL query
        let query = serde_json::json!({
            "query": format!(r#"
            query {{
                projects(membership: true, search: "{}") {{
                    count
                    nodes {{
                        fullPath
                        description
                        webUrl
                        sshUrlToRepo
                    }}
                }}
            }}
        "#, self.search_term.as_str())
        });
        // Make the API request
        let response = httpclient()
            .post("https://gitlab.com/api/graphql")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", gitlab_token()))
            .json(&query)
            .send()
            .await?;
        let json_str = response.text().await?;
        let res: ProjectSearchResponse = serde_json::from_str(&json_str)?;
        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct ProjectForkPrivate {
    pub source_url: String,
    pub target_name: String,
    pub target_namespace_id: Option<u64>,
    pub description: Option<String>,
}

impl Into<ProjectCreate> for ProjectForkPrivate {
    fn into(self) -> ProjectCreate {
        ProjectCreate {
            name: self.target_name,
            namespace_id: self.target_namespace_id,
            description: self.description,
            visibility: ProjectVisibility::Private,
            initialize_with_readme: Some(false),
        }
    }
}

impl ProjectForkPrivate {
    pub fn new(source_url: &str, target_name: &str) -> Self {
        Self {
            source_url: source_url.to_string(),
            target_name: target_name.to_string(),
            target_namespace_id: None,
            description: None,
        }
    }
    pub async fn for_group(
        source_url: &str,
        group: &str,
        name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let group_id = group_by_name(group).await?.id;
        Ok(Self {
            source_url: source_url.to_string(),
            target_name: name.to_string(),
            target_namespace_id: Some(group_id),
            description: None,
        })
    }
    pub async fn run(self, mirror_to_github: bool) -> Result<Project, Box<dyn std::error::Error>> {
        // change to temp directory
        let temp_dir = std::env::temp_dir();
        let temp_repo_path = temp_dir.join(self.target_name.clone());
        std::env::set_current_dir(temp_dir)?;
        let exit_status = Command::new("git")
            .arg("clone")
            .arg("--bare")
            .arg(&self.source_url)
            .arg(&self.target_name)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status()?;
        if !exit_status.success() {
            return Err("Failed to clone repo".into());
        }
        std::env::set_current_dir(temp_repo_path)?;
        // create a new project on gitlab
        let project_create: ProjectCreate = self.clone().into();
        let project = project_create.run(false).await?;
        let exit_status = Command::new("git")
            .arg("push")
            .arg("--mirror")
            .arg(&project.ssh_url_to_repo)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status()?;
        if !exit_status.success() {
            return Err("Failed to push to gitlab".into());
        }
        if mirror_to_github {
            let gh_repo = ghu::repo_create(
                &self.target_name,
                &self.description.unwrap_or_default(),
                false,
            )
            .await?;
            let remote_url_with_cred = ghu::repo_link_with_cred(&gh_repo.name).await?;
            ProjectPushMirror::new(project.id, &remote_url_with_cred)
                .run()
                .await?;
        }
        return Ok(project);
    }
}



/// Helper function for fetching information of packages
pub async fn projects_get_helper<I, K, V>(
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
		"/projects{}",
		path
	), query)?;
    let response = httpclient()
        .get(url)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let json_bytes = response.bytes().await?.to_vec();
    Ok(json_bytes)
}