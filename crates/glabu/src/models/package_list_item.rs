use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub tags: Vec<String>,
    pub created_at: Option<String>,
    pub last_downloaded_at: Option<String>,
    pub package_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageFileInfo {
    pub id: u64,
    pub package_id: u64,
    pub created_at: String,
    pub file_name: String,
    pub size: Option<u64>,
    pub file_md5: Option<String>,
    pub file_sha1: Option<String>,
    pub file_sha256: Option<String>,
    // these are not present in the response, but are needed for later processing
    pub version: Option<String>,
    pub name: Option<String>,
}
