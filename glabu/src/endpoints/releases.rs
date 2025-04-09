use super::projects::project_get;
use super::setup::{gitlab_api_url, gitlab_api_url_with_query, gitlab_token, httpclient};
use crate::models::ProjectRelease;
use either::Either;

pub struct ProjectReleasesGet {
    pub project_id: u64,
}

impl ProjectReleasesGet {
    pub fn new(project_id: u64) -> Self {
        Self { project_id }
    }
    pub async fn from_full_path(full_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let project_id = project_get(full_path).await?.id;
        Ok(Self::new(project_id))
    }
    pub async fn run(
        &self,
    ) -> Result<Either<String, Vec<ProjectRelease>>, Box<dyn std::error::Error>> {
        let response = httpclient()
            .get(gitlab_api_url( &format!("/projects/{}/releases", self.project_id),)?)
            .header("Private-Token", gitlab_token())
            .send()
            .await?;
        let json_str = response.text().await?;
        let res = serde_json::from_str(&json_str)?;
        Ok(res)
    }

    pub async fn latest(&self) -> Result<ProjectRelease, Box<dyn std::error::Error>> {
        let response = httpclient()
            .get(gitlab_api_url( &format!("/projects/{}/releases/permalink/latest", self.project_id),)?)
            .header("Private-Token", gitlab_token())
            .send()
            .await?;
        let json_str = response.text().await?;
        let res: ProjectRelease = serde_json::from_str(&json_str)?;
        Ok(res)
    }
}
