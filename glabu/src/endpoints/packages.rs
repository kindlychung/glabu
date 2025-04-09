//! Module for handling package-related operations in GitLab.
//!
//! This module provides functionalities to list packages, filter package files,
//! and download specific package files based on patterns or filenames.
//!
//! Gitlab organizes packages like this:
//!
//! ```yaml
//! project:
//!   packages:
//!     - name: pack1
//!       version: 1.55.0
//!       package_type: generic
//!       files:
//!         - file_name: pack1_1.55.0_Windows_x86_64_installer.exe
//!           file_md5: <KEY>
//!         - file_name: pack1_1.55.0_Linux_x86_64.tgz
//!           file_md5: <KEY>
//!     - name: pack1
//!       version: 1.54.0
//!       package_type: generic
//!       files:
//!         - file_name: pack1_1.54.0_Windows_x86_64_installer.exe
//!           file_md5: <KEY>
//!         - file_name: pack1_1.54.0_Linux_x86_64.tgz
//!           file_md5: <KEY>
//!     - name: pack2
//!       version: 1.55.0
//!       package_type: generic
//!       files:
//!         - file_name: pack2_1.55.0_Windows_x86_64_installer.exe
//!           file_md5: <KEY>
//!         - file_name: pack2_1.55.0_Linux_x86_64.tgz
//!           file_md5: <KEY>
//!     - name: pack2
//!       version: 1.54.0
//!       package_type: generic
//!       files:
//!         - file_name: pack2_1.54.0_Windows_x86_64_installer.exe
//!           file_md5: <KEY>
//!         - file_name: pack2_1.54.0_Linux_x86_64.tgz
//!           file_md5: <KEY>
//! ```
//!
//! Note the layout above is just conceptual, the actual response from the API is different.
//! See the [GitLab API documentation](https://docs.gitlab.com/user/packages/generic_packages) for more details.

use super::setup::{gitlab_api_url_with_query, gitlab_token, httpclient};
use crate::endpoints::setup::gitlab_api_url;
use crate::endpoints::PrintOutput;
use crate::models::{PackageFileInfo, PackageInfo, SortDirection};
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

trait PackageFileFilter {
    fn filter(&self, files: &PackageFileInfo) -> bool;
}

/// Struct for filtering package files using a regex pattern.
pub struct PatternFilter(Regex);

/// Struct for filtering package files by matching the exact filename.
pub struct FilenameFilter(String);

impl PackageFileFilter for PatternFilter {
    fn filter(&self, files: &PackageFileInfo) -> bool {
        self.0.is_match(&files.file_name)
    }
}

impl PackageFileFilter for FilenameFilter {
    fn filter(&self, files: &PackageFileInfo) -> bool {
        files.file_name == self.0
    }
}

/// Creates a filter based on the provided pattern or filename.
///
/// # Arguments
///
/// * `pattern` - An optional regex pattern for filtering package files.
/// * `filename` - An optional exact filename for filtering package files.
///
/// # Returns
///
/// A boxed trait object implementing `PackageFileFilter`.
fn make_filter(pattern: Option<Regex>, filename: Option<String>) -> Box<dyn PackageFileFilter> {
    if let Some(pattern) = pattern {
        Box::new(PatternFilter(pattern))
    } else if let Some(filename) = filename {
        Box::new(FilenameFilter(filename))
    } else {
        panic!("Either pattern or filename must be provided")
    }
}

/// Enum for sorting packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectPackageListOrderBy {
    CreatedAt,
    Name,
    Version,
    Type,
}

/// Enum for package types.
/// One of conan, maven, npm, pypi, composer, nuget, helm, terraform_module, or golang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    Conan,
    Maven,
    Npm,
    Pypi,
    Composer,
    Nuget,
    Helm,
    TerraformModule,
    Golang,
}

/// Enum for package status.
/// One of default, hidden, processing, error, or pending_destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageStatus {
    Default,
    Hidden,
    Processing,
    Error,
    PendingDestruction,
}

/// Struct for listing packages of a project.
/// This struct holds the necessary information to list packages in a project.
/// See https://docs.gitlab.com/api/packages/#list-packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPackageListOp {
    /// ID or URL-encoded path of the project.
    id: String,
    /// The field to use as order. One of created_at (default), name, version, or type.
    order_by: Option<ProjectPackageListOrderBy>,
    /// The direction of the order, either asc (default) for ascending order or desc for descending order.
    sort: Option<SortDirection>,
    /// Filter the returned packages by type. One of conan, maven, npm, pypi, composer, nuget, helm, terraform_module, or golang.
    package_type: Option<PackageType>,
    /// Filter the project packages with a fuzzy search by name.
    package_name: Option<String>,
    /// Filter the project packages by version. If used in combination with include_versionless, then no versionless packages are returned. Introduced in GitLab 16.6.
    package_version: Option<String>,
    /// When set to true, versionless packages are included in the response.
    include_versionless: Option<bool>,
    /// Filter the returned packages by status. One of default, hidden, processing, error, or pending_destruction.
    status: Option<PackageStatus>,
    /// Page number (default: 1).
    per_page: Option<u64>,
    /// Number of items per page (default: 20, max 1000).
    page: Option<u64>,
}

impl ProjectPackageListOp {
    /// Creates a new `ProjectPackageList` instance.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the project.
    ///
    /// # Returns
    ///
    /// A `ProjectPackageList` instance.
    pub fn new(id: impl ToString) -> Self {
        Self {
            id: id.to_string(),
            order_by: None,
            sort: None,
            package_type: None,
            package_name: None,
            package_version: None,
            include_versionless: None,
            status: None,
            per_page: Some(100),
            page: None,
        }
    }

    /// Sets the order by field for the package list.
    ///
    /// # Arguments
    ///
    /// * `order_by` - The field to order by.
    ///
    /// # Returns
    ///
    /// The updated `ProjectPackageList` instance.
    pub fn order_by(mut self, order_by: Option<ProjectPackageListOrderBy>) -> Self {
		self.order_by = order_by;
        self
    }

    pub fn sort(mut self, sort: Option<SortDirection>) -> Self {
		self.sort = sort;
        self
    }

    pub fn package_type(mut self, package_type: Option<PackageType>) -> Self {
		self.package_type = package_type;
        self
    }

    pub fn package_name(mut self, package_name: Option<String>) -> Self {
		self.package_name = package_name;
        self
    }
    pub fn package_version(mut self, package_version: Option<String>) -> Self {
		self.package_version = package_version;
        self
    }
    pub fn include_versionless(mut self, include_versionless: Option<bool>) -> Self {
		self.include_versionless = include_versionless;
        self
    }
    pub fn status(mut self, status: Option<PackageStatus>) -> Self {
		self.status = status;
        self
    }
    pub fn per_page(mut self, per_page: Option<u64>) -> Self {
		self.per_page = per_page;
        self
    }
    pub fn page(mut self, page: Option<u64>) -> Self {
		self.page = page;
        self
    }

    pub fn latest(&mut self)  {
        self.sort = Some(SortDirection::Desc);
        self.order_by = Some(ProjectPackageListOrderBy::CreatedAt);
        self.per_page = Some(1);
        self.page = Some(1);
    }

    pub async fn list(&self) -> Result<Vec<PackageInfo>, Box<dyn std::error::Error>> {
        let self_json = serde_json::to_string(&self)?;
        let self_map: HashMap<String, Option<String>> = serde_json::from_str(&self_json)?;
        let query: Vec<(&str, &str)> = self_map
            .iter()
            .filter_map(|(key, value)| {
                value.as_ref().map(|value| {
                    // eprintln!("key: {}, value: {}", key, value);
                    (key.as_str(), value.as_str())
                })
            })
            .collect();
        let json = packages_get_helper(self.id.clone(), "", query).await?;
        let packages = serde_json::from_slice::<Vec<PackageInfo>>(&json)?;
        Ok(packages)
    }

    pub async fn first(&self) -> Result<PackageInfo, Box<dyn std::error::Error>> {
        let mut packages = self.list().await?;
        let res = packages.pop().ok_or::<Box<dyn std::error::Error>>("PackageNotFound".into())?;
        Ok(res)
    }

    pub async fn package_by_id(
        &self,
        package_id: u64,
    ) -> Result<PackageInfo, Box<dyn std::error::Error>> {
        let path = format!("/{}", package_id);
        let json = packages_get_helper(self.id.clone(), &path, vec![("", "")]).await?;
        let package = serde_json::from_slice::<PackageInfo>(&json)?;
        Ok(package)
    }

    pub async fn package_files(
        &self,
        package: &PackageInfo,
    ) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
        let path = format!("/{}/package_files", &package.id);
        let json = packages_get_helper(self.id.clone(), &path, vec![("", "")]).await?;
        let package_files = serde_json::from_slice::<Vec<PackageFileInfo>>(&json)?;
        let package_files = package_files
            .into_iter()
            .map(|mut package_file| {
                package_file.version = Some(package.version.clone());
                package_file.name = Some(package.name.clone());
                package_file
            })
            .collect();
        Ok(package_files)
    }

    pub async fn package_files_by_version(
        &mut self,
        version: &str,
    ) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
        self.package_version = Some(version.to_string());
        let package = self.first().await?;
        let package_files = self.package_files(&package).await?;
        Ok(package_files)
    }

    pub async fn package_files_latest_version(
        &mut self,
    ) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
        self.latest();
        let package = self.first().await?;
        let package_files = self.package_files(&package).await?;
        Ok(package_files)
    }
}

/// Info need for uploading/downloading generic package files.
/// See gitlab api doc: https://docs.gitlab.com/user/packages/generic_packages/
#[derive(Debug, Clone)]
pub struct GenericPackageOp {
    ///  Your project ID or URL-encoded path
    pub project_id: String,
    /// Name of your package
    pub package_name: String,
    /// Version of your package, if not provided, the latest version will be used
    pub package_version: Option<String>,
    /// The file name
    pub file_name: String,
}

impl GenericPackageOp {
    pub fn new(project_id: impl ToString, package_name: &str, file_name: &str) -> Self {
        Self {
            project_id: project_id.to_string(),
            package_name: package_name.to_string(),
            file_name: file_name.to_string(),
            package_version: None,
        }
    }

	pub fn package_name(mut self, package_name: &str) -> Self {
		self.package_name = package_name.to_string();
		self
	}

	pub fn package_version(mut self, pv: Option<String>) -> Self {
		self.package_version = pv;
		self
	}
	pub fn file_name(mut self, file_name: &str) -> Self {
		self.file_name = file_name.to_string();
		self
	}


    pub async fn download_files(
        self,
        output_dir: PathBuf,
        pattern: Option<String>,
        filename: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pattern = pattern.map(|x| Regex::new(&x).unwrap());
        let filter = make_filter(pattern, filename);
        let mut project_packages_list_op =
            ProjectPackageListOp::new(&self.project_id).package_name(Some(self.package_name.clone()));
        let package_files = if let Some(version) = self.package_version.as_ref() {
            project_packages_list_op
                .package_files_by_version(version)
                .await?
        } else {
            project_packages_list_op
                .package_files_latest_version()
                .await?
        };

        let mut outputs = vec![];
        for package_file in &package_files {
            if !filter.filter(package_file) {
                continue;
            }
            let package_file_path = format!(
                "/projects/{}/packages/generic/{}/{}/{}",
                self.project_id,
                package_file.name.as_ref().unwrap(),
                package_file.version.as_ref().unwrap(),
                package_file.file_name.as_str()
            );
            let url = gitlab_api_url(&package_file_path, )?;
            let output_file = if output_dir.is_dir() {
                output_dir.join(&package_file.file_name)
            } else {
                eprintln!("Warning: ouput_dir is not a directory, use /tmp as fallback");
                PathBuf::from("/tmp").join(&package_file.file_name)
            };
            let output_str = output_file.as_path().to_str().unwrap().to_string();
            let _ = download_file(url, &output_file).await?;
            outputs.push(output_str);
        }
        let msg = PrintOutput {
            status: "ok".to_string(),
            output: outputs,
        };
        let msg = serde_json::to_string_pretty(&msg)?;
        println!("{}", msg);
        Ok(())
    }

    /// Uploads a package file to the GitLab package registry.
    /// Based on this curl:
    ///
    /// ```bash
    /// curl --location --header "PRIVATE-TOKEN: $GITLAB_TOKEN" --upload-file ./target/release/glabu "https://gitlab.com/api/v4/projects/61010542/packages/generic/glabu/0.1/glabu-linux-aarch64"
    /// ```
    pub async fn upload_package_file(
        &self,
        package_version: &str,
        file_name: &str,
        file_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url_path = format!(
            "/projects/{}/packages/generic/{}/{}/{}",
            self.project_id, self.package_name, package_version, file_name
        );
        let url = gitlab_api_url(&url_path, )?;
        let file = tokio::fs::read(file_path).await?;
        let response = httpclient()
            .put(url)
            .header("Private-Token", gitlab_token())
            .body(file)
            .send()
            .await?;
        let status = response.status();
        let content = response.text().await?;
        dbg!(&content);
        if status != 201 {
            return Err(format!(
                "Upload failed with status: {}, and message: {}",
                status, &content
            )
            .into());
        }
        println!("{}", content);
        Ok(())
    }
}

/// Downloads a file from a given URL.
///
/// # Arguments
///
/// * `url` - The URL of the file to download.
/// * `output_file` - The path where the file should be saved.
///
/// # Returns
///
/// A `Result` indicating success or an error.
pub async fn download_file<P>(url: Url, output_file: P) -> Result<(), Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
{
    let response = httpclient()
        .get(url)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let status = response.status();
    let content = response.bytes().await?;
    if status != 200 {
        return Err(format!(
            "DownloadFileErr: {}",
            String::from_utf8(content.to_vec()).unwrap_or(status.to_string())
        )
        .into());
    }
    let mut file = File::create(output_file)?;
    file.write_all(&content)?;
    Ok(())
}

/// Helper function to delete package related info.
pub async fn delete_package_helper(
    project_id: impl ToString,
    package_id: u64,
	path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let url = gitlab_api_url(&format!(
		"/projects/{}/packages/{}{}",
		project_id.to_string(),
		package_id,
		path
	))?;
    let response = httpclient()
        .delete(url)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let status = response.status();
    let content = response.text().await?;
    eprintln!("delete_package status: {}", status);
	eprintln!("delete_package content: {}", content);
    if status != 200 {
        return Err(format!("DeletePackageErr: {}", status).into());
    }
    Ok(())
}

pub async fn delete_package(
    project_id: impl ToString,
    package_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("/{}", package_id);
    delete_package_helper(project_id, package_id, &path).await?;
	Ok(())
}

pub async fn delete_package_file(
    project_id: impl ToString,
    package_id: u64,
    package_file_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("/{}/package_files/{}", package_id, package_file_id);
    delete_package_helper(project_id, package_file_id, &path).await?;
	Ok(())
}

/// Helper function for fetching information of packages
pub async fn packages_get_helper<I, K, V>(
    project_id: impl ToString,
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
		"/projects/{}/packages{}",
		project_id.to_string(),
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