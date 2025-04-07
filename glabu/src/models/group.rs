use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Group {
    pub id: u64,
    pub web_url: String,
    pub name: String,
    pub path: String,
    pub description: String,
    pub visibility: String,
    pub share_with_group_lock: bool,
    pub require_two_factor_authentication: bool,
    pub two_factor_grace_period: u32,
    pub project_creation_level: String,
    pub auto_devops_enabled: Option<bool>,
    pub subgroup_creation_level: String,
    pub emails_disabled: bool,
    pub emails_enabled: bool,
    pub mentions_disabled: Option<bool>,
    pub lfs_enabled: bool,
    pub math_rendering_limits_enabled: bool,
    pub lock_math_rendering_limits_enabled: bool,
    pub default_branch: Option<String>,
    pub default_branch_protection: u32,
    pub default_branch_protection_defaults: Option<DefaultBranchProtection>,
    pub avatar_url: Option<String>,
    pub request_access_enabled: bool,
    pub full_name: String,
    pub full_path: String,
    pub created_at: String, // Using String instead of chrono::DateTime
    pub parent_id: Option<u64>,
    pub organization_id: u64,
    pub shared_runners_setting: String,
    pub max_artifacts_size: Option<u64>,
    pub ldap_cn: Option<String>,
    pub ldap_access: Option<u64>,
    pub wiki_access_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DefaultBranchProtection {
    pub allowed_to_push: Vec<AccessLevel>,
    pub allow_force_push: bool,
    pub allowed_to_merge: Vec<AccessLevel>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AccessLevel {
    pub access_level: u32,
}
