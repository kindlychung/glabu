use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub avatar_url: String,
    pub bio: String,
    pub bot: bool,
    pub can_create_group: bool,
    pub can_create_project: bool,
    pub color_scheme_id: i64,
    pub commit_email: String,
    #[serde(default)]
    pub confirmed_at: Option<String>, // ISO 8601 timestamp as string
    #[serde(default)]
    pub created_at: Option<String>, // ISO 8601 timestamp as string
    #[serde(default)]
    pub current_sign_in_at: Option<String>, // ISO 8601 timestamp as string
    pub discord: String,
    pub email: String,
    pub external: bool,
    pub extra_shared_runners_minutes_limit: Option<i64>,
    pub id: u64,
    pub identities: Vec<Identity>,
    pub job_title: String,
    #[serde(default)]
    pub last_activity_on: Option<String>, // Date as "YYYY-MM-DD"
    #[serde(default)]
    pub last_sign_in_at: Option<String>, // ISO 8601 timestamp as string
    pub name: String,
    pub organization: String,
    pub private_profile: bool,
    pub projects_limit: i64,
    pub public_email: String,
    pub shared_runners_minutes_limit: Option<i64>,
    pub state: String,
    pub two_factor_enabled: bool,
    pub username: String,
    pub web_url: String,
    pub website_url: String,
    #[serde(default)]
    pub work_information: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Identity {
    pub extern_uid: String,
    pub provider: String,
    #[serde(default)]
    pub saml_provider_id: Option<String>,
}
