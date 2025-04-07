use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub description: String,
    pub name: String,
    pub name_with_namespace: String,
    pub path: String,
    pub path_with_namespace: String,
    pub created_at: String,
    pub default_branch: String,
    pub tag_list: Vec<String>,
    pub topics: Vec<String>,
    pub ssh_url_to_repo: String,
    pub http_url_to_repo: String,
    pub web_url: String,
    pub readme_url: Option<String>,
    pub forks_count: u64,
    pub avatar_url: Option<String>,
    pub star_count: u64,
    pub last_activity_at: String,
    pub namespace: Option<Namespace>,
    pub container_registry_image_prefix: String,
    pub links: Option<Links>,
    pub packages_enabled: bool,
    pub empty_repo: bool,
    pub archived: bool,
    pub visibility: String,
    pub owner: Option<Owner>,
    pub resolve_outdated_diff_discussions: bool,
    pub container_expiration_policy: Option<ContainerExpirationPolicy>,
    pub repository_object_format: String,
    pub issues_enabled: bool,
    pub merge_requests_enabled: bool,
    pub wiki_enabled: bool,
    pub jobs_enabled: bool,
    // pub snippets_enabled: bool,
    // pub container_registry_enabled: bool,
    // pub service_desk_enabled: bool,
    // pub service_desk_address: String,
    // pub can_create_merge_request_in: bool,
    // pub issues_access_level: String,
    // pub repository_access_level: String,
    // pub merge_requests_access_level: String,
    // pub forking_access_level: String,
    // pub wiki_access_level: String,
    // pub builds_access_level: String,
    // pub snippets_access_level: String,
    // pub pages_access_level: String,
    // pub analytics_access_level: String,
    // pub container_registry_access_level: String,
    // pub security_and_compliance_access_level: String,
    // pub releases_access_level: String,
    // pub environments_access_level: String,
    // pub feature_flags_access_level: String,
    // pub infrastructure_access_level: String,
    // pub monitor_access_level: String,
    // pub model_experiments_access_level: String,
    // pub model_registry_access_level: String,
    // pub emails_disabled: bool,
    // pub emails_enabled: bool,
    // pub shared_runners_enabled: bool,
    // pub lfs_enabled: bool,
    // pub creator_id: i64,
    // pub import_url: Option<String>,
    // pub import_type: Option<String>,
    // pub import_status: String,
    // pub import_error: Option<String>,
    pub open_issues_count: i64,
    pub description_html: String,
    pub updated_at: String,
    // pub ci_default_git_depth: i64,
    // pub ci_delete_pipelines_in_seconds: Option<i64>,
    // pub ci_forward_deployment_enabled: bool,
    // pub ci_forward_deployment_rollback_allowed: bool,
    // pub ci_job_token_scope_enabled: bool,
    // pub ci_separated_caches: bool,
    // pub ci_allow_fork_pipelines_to_run_in_parent_project: bool,
    // pub ci_id_token_sub_claim_components: Vec<String>,
    // pub build_git_strategy: String,
    // pub keep_latest_artifact: bool,
    // pub restrict_user_defined_variables: bool,
    // pub ci_pipeline_variables_minimum_override_role: String,
    // pub runners_token: String,
    // pub runner_token_expiration_interval: Option<i64>,
    // pub group_runners_enabled: bool,
    // pub auto_cancel_pending_pipelines: String,
    // pub build_timeout: i64,
    // pub auto_devops_enabled: bool,
    // pub auto_devops_deploy_strategy: String,
    // pub ci_push_repository_for_job_token_allowed: bool,
    // pub ci_config_path: String,
    // pub public_jobs: bool,
    // pub shared_with_groups: Vec<()>, // Empty array in example
    // pub only_allow_merge_if_pipeline_succeeds: bool,
    // pub allow_merge_on_skipped_pipeline: Option<bool>,
    // pub request_access_enabled: bool,
    // pub only_allow_merge_if_all_discussions_are_resolved: bool,
    // pub remove_source_branch_after_merge: bool,
    // pub printing_merge_request_link_enabled: bool,
    // pub merge_method: String,
    // pub squash_option: String,
    // pub enforce_auth_checks_on_uploads: bool,
    // pub suggestion_commit_message: Option<String>,
    // pub merge_commit_template: Option<String>,
    // pub squash_commit_template: Option<String>,
    // pub issue_branch_template: Option<String>,
    // pub warn_about_potentially_unwanted_characters: bool,
    // pub autoclose_referenced_issues: bool,
    // pub max_artifacts_size: Option<i64>,
    // pub external_authorization_classification_label: String,
    // pub requirements_enabled: bool,
    // pub requirements_access_level: String,
    // pub security_and_compliance_enabled: bool,
    // pub compliance_frameworks: Vec<()>, // Empty array in example
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Namespace {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub full_path: String,
    pub parent_id: Option<i64>,
    pub avatar_url: String,
    pub web_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    pub self_link: String,
    pub issues: String,
    pub merge_requests: String,
    pub repo_branches: String,
    pub labels: String,
    pub events: String,
    pub members: String,
    pub cluster_agents: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub id: i64,
    pub username: String,
    pub name: String,
    pub state: String,
    pub locked: bool,
    pub avatar_url: String,
    pub web_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerExpirationPolicy {
    pub cadence: String,
    pub enabled: bool,
    pub keep_n: i64,
    pub older_than: String,
    pub name_regex: String,
    pub name_regex_keep: Option<String>,
    pub next_run_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectPushMirrorPayload {
    pub url: String,
    pub enabled: bool,
    pub only_protected_branches: bool,
    pub keep_divergent_refs: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCreatePayload {
    pub name: String,
    pub namespace_id: Option<u64>,
    pub description: Option<String>,
    pub visibility: ProjectVisibility,
    pub initialize_with_readme: Option<bool>,
}

/// use snake_case here for serde
#[serde(rename_all = "snake_case")]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, ValueEnum)]
pub enum ProjectVisibility {
    Public,
    Internal,
    Private,
}
