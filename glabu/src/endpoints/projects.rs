use super::profiles::{group_by_id, group_by_name, me};
use super::setup::{EMPTY_QUERY, gitlab_api_url_with_query, gitlab_token, httpclient};
use crate::endpoints::setup::gitlab_api_url;
use crate::models::ProjectCreatePayload;
use crate::models::{Project, ProjectPushMirrorPayload, ProjectVisibility};
use std::borrow::Borrow;
use urlencoding::encode;
use xshell::{Shell, cmd};

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
        let mut proj = match project_get_by_id(&full_name).await {
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
                .post(gitlab_api_url("/projects")?)
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
            .delete(gitlab_api_url(&format!(
                "/projects/{}",
                encode(&self.full_name)
            ))?)
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
        let repo = project_get_by_id(repo_path).await?;
        Ok(Self::new(repo.id, remote_url_with_cred))
    }
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_id = self.project_id;
        let body: ProjectPushMirrorPayload = self.into();
        let api_url = gitlab_api_url(&format!("/projects/{}/remote_mirrors", repo_id))?;
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

#[derive(Debug, Clone)]
pub struct ProjectForkPrivate {
    pub source_url: String,
    pub target_name: String,
    pub target_namespace_id: Option<u64>,
    pub description: Option<String>,
    pub mirror_to_github: bool,
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
            mirror_to_github: true,
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
            mirror_to_github: true,
        })
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn mirror_to_github(mut self, mirror_to_github: bool) -> Self {
        self.mirror_to_github = mirror_to_github;
        self
    }

    pub async fn run(self) -> Result<Project, Box<dyn std::error::Error>> {
        // change to temp directory
        let temp_dir = std::env::temp_dir();
        let temp_repo_path = temp_dir.join(self.target_name.clone());
        let sh = Shell::new()?;
        let source_url = &self.source_url;
        let target_name = &self.target_name;
        sh.change_dir(&temp_dir);
        cmd!(&sh, "git clone --bare {source_url} {target_name}").run()?;
        // create a new project on gitlab
        let project_create: ProjectCreate = self.clone().into();
        let project = project_create.run(false).await?;
        let ssh_url_to_repo = &project.ssh_url_to_repo;
        dbg!(ssh_url_to_repo);
        sh.change_dir(&temp_repo_path);
        cmd!(&sh, "ls -lh").run()?;
        cmd!(&sh, "git push --mirror {ssh_url_to_repo}").run()?;
        if self.mirror_to_github {
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
    let url = gitlab_api_url_with_query(&format!("/projects{}", path), query)?;
    let response = httpclient()
        .get(url)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let json_bytes = response.bytes().await?.to_vec();
    Ok(json_bytes)
}

/// Get a single project by its ID or path (with namespace prefix, e.g. "user/repo")
pub async fn project_get_by_id(id: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let id = if id.contains("/") {
        encode(id).to_string()
    } else {
        id.to_string()
    };
    let json_bytes = projects_get_helper(&format!("/{}", id), EMPTY_QUERY).await?;
    let project = serde_json::from_slice::<Project>(&json_bytes)?;
    Ok(project)
}

pub async fn projects_search(
    term: &str,
    owned: bool,
) -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let json = projects_get_helper("", &[("search", term), ("owned", &owned.to_string())]).await?;
    // eprintln!("json: {}", String::from_utf8_lossy(&json));
    let res: Vec<Project> = serde_json::from_slice(&json)?;
    Ok(res)
}

// json: [{"id":68749765,"description":"","name":"glabu","name_with_namespace":"puterize / glabu","path":"glabu","path_with_namespace":"puterize/glabu","created_at":"2025-04-07T20:21:31.980Z","default_branch":"master","tag_list":[],"topics":[],"ssh_url_to_repo":"git@gitlab.com:puterize/glabu.git","http_url_to_repo":"https://gitlab.com/puterize/glabu.git","web_url":"https://gitlab.com/puterize/glabu","readme_url":"https://gitlab.com/puterize/glabu/-/blob/master/readme.md","forks_count":0,"avatar_url":null,"star_count":0,"last_activity_at":"2025-04-09T07:57:35.474Z","namespace":{"id":63741244,"name":"puterize","path":"puterize","kind":"group","full_path":"puterize","parent_id":null,"avatar_url":"/uploads/-/system/group/avatar/63741244/taal.png","web_url":"https://gitlab.com/groups/puterize"},"container_registry_image_prefix":"registry.gitlab.com/puterize/glabu","_links":{"self":"https://gitlab.com/api/v4/projects/68749765","issues":"https://gitlab.com/api/v4/projects/68749765/issues","merge_requests":"https://gitlab.com/api/v4/projects/68749765/merge_requests","repo_branches":"https://gitlab.com/api/v4/projects/68749765/repository/branches","labels":"https://gitlab.com/api/v4/projects/68749765/labels","events":"https://gitlab.com/api/v4/projects/68749765/events","members":"https://gitlab.com/api/v4/projects/68749765/members","cluster_agents":"https://gitlab.com/api/v4/projects/68749765/cluster_agents"},"packages_enabled":true,"empty_repo":false,"archived":false,"visibility":"public","resolve_outdated_diff_discussions":false,"container_expiration_policy":{"cadence":"1d","enabled":false,"keep_n":10,"older_than":"90d","name_regex":".*","name_regex_keep":null,"next_run_at":"2025-04-08T20:21:32.002Z"},"repository_object_format":"sha1","issues_enabled":true,"merge_requests_enabled":true,"wiki_enabled":true,"jobs_enabled":true,"snippets_enabled":true,"container_registry_enabled":true,"service_desk_enabled":true,"service_desk_address":"contact-project+puterize-glabu-68749765-issue-@incoming.gitlab.com","can_create_merge_request_in":true,"issues_access_level":"enabled","repository_access_level":"enabled","merge_requests_access_level":"enabled","forking_access_level":"enabled","wiki_access_level":"enabled","builds_access_level":"enabled","snippets_access_level":"enabled","pages_access_level":"private","analytics_access_level":"enabled","container_registry_access_level":"enabled","security_and_compliance_access_level":"private","releases_access_level":"enabled","environments_access_level":"enabled","feature_flags_access_level":"enabled","infrastructure_access_level":"enabled","monitor_access_level":"enabled","model_experiments_access_level":"enabled","model_registry_access_level":"enabled","emails_disabled":false,"emails_enabled":true,"shared_runners_enabled":true,"lfs_enabled":true,"creator_id":7907829,"import_url":null,"import_type":null,"import_status":"none","open_issues_count":0,"description_html":"","updated_at":"2025-04-09T07:57:35.474Z","ci_default_git_depth":20,"ci_delete_pipelines_in_seconds":null,"ci_forward_deployment_enabled":true,"ci_forward_deployment_rollback_allowed":true,"ci_job_token_scope_enabled":false,"ci_separated_caches":true,"ci_allow_fork_pipelines_to_run_in_parent_project":true,"ci_id_token_sub_claim_components":["project_path","ref_type","ref"],"build_git_strategy":"fetch","keep_latest_artifact":true,"restrict_user_defined_variables":false,"ci_pipeline_variables_minimum_override_role":"developer","runners_token":null,"runner_token_expiration_interval":null,"group_runners_enabled":true,"auto_cancel_pending_pipelines":"enabled","build_timeout":3600,"auto_devops_enabled":false,"auto_devops_deploy_strategy":"continuous","ci_push_repository_for_job_token_allowed":false,"ci_config_path":"","public_jobs":true,"shared_with_groups":[],"only_allow_merge_if_pipeline_succeeds":false,"allow_merge_on_skipped_pipeline":null,"request_access_enabled":true,"only_allow_merge_if_all_discussions_are_resolved":false,"remove_source_branch_after_merge":true,"printing_merge_request_link_enabled":true,"merge_method":"merge","squash_option":"default_off","enforce_auth_checks_on_uploads":true,"suggestion_commit_message":null,"merge_commit_template":null,"squash_commit_template":null,"issue_branch_template":null,"warn_about_potentially_unwanted_characters":true,"autoclose_referenced_issues":true,"max_artifacts_size":null,"external_authorization_classification_label":"","requirements_enabled":false,"requirements_access_level":"enabled","security_and_compliance_enabled":true,"compliance_frameworks":[],"permissions":{"project_access":null,"group_access":{"access_level":50,"notification_level":3}}},{"id":55331319,"description":null,"name":"bglabutils","name_with_namespace":"Evgenii Kurbatov / bglabutils","path":"bglabutils","path_with_namespace":"ekurbatov/bglabutils","created_at":"2024-02-27T08:39:17.762Z","default_branch":"master","tag_list":[],"topics":[],"ssh_url_to_repo":"git@gitlab.com:ekurbatov/bglabutils.git","http_url_to_repo":"https://gitlab.com/ekurbatov/bglabutils.git","web_url":"https://gitlab.com/ekurbatov/bglabutils","readme_url":null,"forks_count":0,"avatar_url":null,"star_count":0,"last_activity_at":"2025-03-25T00:12:04.963Z","namespace":{"id":2651694,"name":"Evgenii Kurbatov","path":"ekurbatov","kind":"user","full_path":"ekurbatov","parent_id":null,"avatar_url":"https://secure.gravatar.com/avatar/0a0f082aec1ecc074df3c26e4f71912352db9a83c15c721e078e7a64c9264a87?s=80\u0026d=identicon","web_url":"https://gitlab.com/ekurbatov"},"container_registry_image_prefix":"registry.gitlab.com/ekurbatov/bglabutils","_links":{"self":"https://gitlab.com/api/v4/projects/55331319","issues":"https://gitlab.com/api/v4/projects/55331319/issues","merge_requests":"https://gitlab.com/api/v4/projects/55331319/merge_requests","repo_branches":"https://gitlab.com/api/v4/projects/55331319/repository/branches","labels":"https://gitlab.com/api/v4/projects/55331319/labels","events":"https://gitlab.com/api/v4/projects/55331319/events","members":"https://gitlab.com/api/v4/projects/55331319/members","cluster_agents":"https://gitlab.com/api/v4/projects/55331319/cluster_agents"},"packages_enabled":true,"empty_repo":false,"archived":false,"visibility":"public","owner":{"id":2132624,"username":"ekurbatov","name":"Evgenii Kurbatov","state":"active","locked":false,"avatar_url":"https://secure.gravatar.com/avatar/0a0f082aec1ecc074df3c26e4f71912352db9a83c15c721e078e7a64c9264a87?s=80\u0026d=identicon","web_url":"https://gitlab.com/ekurbatov"},"resolve_outdated_diff_discussions":false,"container_expiration_policy":{"cadence":"1d","enabled":false,"keep_n":10,"older_than":"90d","name_regex":".*","name_regex_keep":null,"next_run_at":"2024-02-28T08:39:17.785Z"},"repository_object_format":"sha1","issues_enabled":true,"merge_requests_enabled":true,"wiki_enabled":true,"jobs_enabled":true,"snippets_enabled":true,"container_registry_enabled":true,"service_desk_enabled":true,"can_create_merge_request_in":true,"issues_access_level":"enabled","repository_access_level":"enabled","merge_requests_access_level":"enabled","forking_access_level":"enabled","wiki_access_level":"enabled","builds_access_level":"enabled","snippets_access_level":"enabled","pages_access_level":"enabled","analytics_access_level":"enabled","container_registry_access_level":"enabled","security_and_compliance_access_level":"private","releases_access_level":"enabled","environments_access_level":"enabled","feature_flags_access_level":"enabled","infrastructure_access_level":"enabled","monitor_access_level":"enabled","model_experiments_access_level":"enabled","model_registry_access_level":"enabled","emails_disabled":false,"emails_enabled":true,"shared_runners_enabled":true,"lfs_enabled":true,"creator_id":2132624,"import_status":"none","open_issues_count":0,"description_html":"","updated_at":"2025-03-25T00:12:04.963Z","ci_config_path":"","public_jobs":true,"shared_with_groups":[],"only_allow_merge_if_pipeline_succeeds":false,"allow_merge_on_skipped_pipeline":null,"request_access_enabled":true,"only_allow_merge_if_all_discussions_are_resolved":false,"remove_source_branch_after_merge":true,"printing_merge_request_link_enabled":true,"merge_method":"merge","squash_option":"default_off","enforce_auth_checks_on_uploads":true,"suggestion_commit_message":null,"merge_commit_template":null,"squash_commit_template":null,"issue_branch_template":null,"warn_about_potentially_unwanted_characters":true,"autoclose_referenced_issues":true,"max_artifacts_size":null,"external_authorization_classification_label":"","requirements_enabled":false,"requirements_access_level":"enabled","security_and_compliance_enabled":false,"compliance_frameworks":[],"permissions":{"project_access":null,"group_access":null}},{"id":5505104,"description":"","name":"ElectricBillCalculator_Pioray_Paglabuan","name_with_namespace":"CCC_CS322_WebDesign2_2017-2018_CS3A / ElectricBillCalculator_Pioray_Paglabuan","path":"ElectricBillCalculator_Pioray_Paglabuan","path_with_namespace":"CCC_CS322_WebDesign2_2017-2018_CS3A/ElectricBillCalculator_Pioray_Paglabuan","created_at":"2018-02-19T11:28:00.871Z","default_branch":"master","tag_list":[],"topics":[],"ssh_url_to_repo":"git@gitlab.com:CCC_CS322_WebDesign2_2017-2018_CS3A/ElectricBillCalculator_Pioray_Paglabuan.git","http_url_to_repo":"https://gitlab.com/CCC_CS322_WebDesign2_2017-2018_CS3A/ElectricBillCalculator_Pioray_Paglabuan.git","web_url":"https://gitlab.com/CCC_CS322_WebDesign2_2017-2018_CS3A/ElectricBillCalculator_Pioray_Paglabuan","readme_url":null,"forks_count":0,"avatar_url":null,"star_count":0,"last_activity_at":"2018-02-21T02:41:04.073Z","namespace":{"id":2224919,"name":"CCC_CS322_WebDesign2_2017-2018_CS3A","path":"CCC_CS322_WebDesign2_2017-2018_CS3A","kind":"group","full_path":"CCC_CS322_WebDesign2_2017-2018_CS3A","parent_id":null,"avatar_url":null,"web_url":"https://gitlab.com/groups/CCC_CS322_WebDesign2_2017-2018_CS3A"},"container_registry_image_prefix":"registry.gitlab.com/ccc_cs322_webdesign2_2017-2018_cs3a/electricbillcalculator_pioray_paglabuan","_links":{"self":"https://gitlab.com/api/v4/projects/5505104","issues":"https://gitlab.com/api/v4/projects/5505104/issues","merge_requests":"https://gitlab.com/api/v4/projects/5505104/merge_requests","repo_branches":"https://gitlab.com/api/v4/projects/5505104/repository/branches","labels":"https://gitlab.com/api/v4/projects/5505104/labels","events":"https://gitlab.com/api/v4/projects/5505104/events","members":"https://gitlab.com/api/v4/projects/5505104/members","cluster_agents":"https://gitlab.com/api/v4/projects/5505104/cluster_agents"},"packages_enabled":null,"empty_repo":false,"archived":false,"visibility":"internal","resolve_outdated_diff_discussions":false,"repository_object_format":"sha1","issues_enabled":true,"merge_requests_enabled":true,"wiki_enabled":true,"jobs_enabled":true,"snippets_enabled":true,"container_registry_enabled":true,"service_desk_enabled":true,"can_create_merge_request_in":true,"issues_access_level":"enabled","repository_access_level":"enabled","merge_requests_access_level":"enabled","forking_access_level":"enabled","wiki_access_level":"enabled","builds_access_level":"enabled","snippets_access_level":"enabled","pages_access_level":"public","analytics_access_level":"enabled","container_registry_access_level":"enabled","security_and_compliance_access_level":"private","releases_access_level":"enabled","environments_access_level":"enabled","feature_flags_access_level":"enabled","infrastructure_access_level":"enabled","monitor_access_level":"enabled","model_experiments_access_level":"enabled","model_registry_access_level":"enabled","emails_disabled":false,"emails_enabled":true,"shared_runners_enabled":true,"lfs_enabled":true,"creator_id":1808874,"import_status":"none","open_issues_count":0,"description_html":"","updated_at":"2024-01-18T21:16:08.026Z","ci_config_path":null,"public_jobs":true,"shared_with_groups":[],"only_allow_merge_if_pipeline_succeeds":false,"allow_merge_on_skipped_pipeline":null,"request_access_enabled":false,"only_allow_merge_if_all_discussions_are_resolved":false,"remove_source_branch_after_merge":null,"printing_merge_request_link_enabled":true,"merge_method":"merge","squash_option":"default_off","enforce_auth_checks_on_uploads":true,"suggestion_commit_message":null,"merge_commit_template":null,"squash_commit_template":null,"issue_branch_template":null,"warn_about_potentially_unwanted_characters":true,"autoclose_referenced_issues":true,"max_artifacts_size":null,"external_authorization_classification_label":"","requirements_enabled":false,"requirements_access_level":"enabled","security_and_compliance_enabled":false,"compliance_frameworks":[],"permissions":{"project_access":null,"group_access":null}}]

#[cfg(test)]
mod projects_tests {
    use super::*;

    #[tokio::test]
    async fn test_projects_get_helper() -> Result<(), Box<dyn std::error::Error>> {
        for project_key in &["/68749765", "/puterize%2Fglabu"] {
            let result =
                projects_get_helper(project_key, &[("license", "true"), ("statistics", "true")])
                    .await?;

            let result = String::from_utf8(result)?;
            // eprintln!("{}", &result);
            let project = serde_json::from_str::<Project>(&result)?;
            assert_eq!(project.name, "glabu");
            assert_eq!(project.name_with_namespace, "puterize / glabu");
            assert_eq!(&project.namespace.unwrap().full_path, "puterize");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_project_get_by_id() -> Result<(), Box<dyn std::error::Error>> {
        for project_key in &["68749765", "puterize%2Fglabu", "puterize/glabu"] {
            let project = project_get_by_id(project_key).await?;
            assert_eq!(project.name, "glabu");
            assert_eq!(project.name_with_namespace, "puterize / glabu");
            assert_eq!(&project.namespace.unwrap().full_path, "puterize");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_project_search() -> Result<(), Box<dyn std::error::Error>> {
        let projects = projects_search("glabu", true).await?;
        let n = projects.len();
        assert_eq!(n, 1);
        let project = &projects[0];
        assert_eq!(project.name, "glabu");
        assert_eq!(project.name_with_namespace, "puterize / glabu");
        assert_eq!(project.path_with_namespace, "puterize/glabu");
        Ok(())
    }
}
