use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectRelease {
    name: String,
    tag_name: String,
    description: String,
    created_at: String,
    released_at: String,
    upcoming_release: bool,
    author: Author,
    commit: Commit,
    // milestones: Option<Vec<Milestone>>,
    commit_path: String,
    tag_path: String,
    assets: Assets,
    evidences: Vec<Evidence>,
    // _links: Links,
}

#[derive(Serialize, Deserialize, Debug)]
struct Author {
    id: u64,
    username: String,
    name: String,
    state: String,
    locked: bool,
    avatar_url: String,
    web_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Commit {
    id: String,
    short_id: String,
    created_at: String,
    parent_ids: Vec<String>,
    title: String,
    message: String,
    author_name: String,
    author_email: String,
    authored_date: String,
    committer_name: String,
    committer_email: String,
    committed_date: String,
    // trailers: Value,
    // extended_trailers: Value,
    web_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Milestone {
    id: u64,
    iid: u64,
    group_id: u64,
    title: String,
    description: String,
    state: String,
    created_at: String,
    updated_at: String,
    due_date: Option<String>,
    start_date: Option<String>,
    expired: bool,
    web_url: String,
    issue_stats: IssueStats,
}

#[derive(Serialize, Deserialize, Debug)]
struct IssueStats {
    total: u64,
    closed: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Assets {
    count: u64,
    sources: Vec<Source>,
    links: Vec<Link>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Source {
    format: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Link {
    id: Option<u64>,
    name: String,
    url: String,
    direct_asset_url: String,
    link_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Evidence {
    sha: String,
    filepath: String,
    collected_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Links {
    closed_issues_url: String,
    closed_merge_requests_url: String,
    merged_merge_requests_url: String,
    // opended_issues_url: String,
    // opended_merge_requests_url: String,
    #[serde(rename = "self")]
    selflink: String,
}
