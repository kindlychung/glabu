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

use super::projects::project_get;
use super::setup::{gitlab_api_url, gitlab_token, httpclient};
use crate::endpoints::PrintOutput;
use crate::models::{PackageFileInfo, PackageInfo, SortDirection};
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::borrow::Borrow;
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
	Type
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
	PendingDestruction
}


/// Struct for listing packages of a project.
/// This struct holds the necessary information to list packages in a project.
/// See https://docs.gitlab.com/api/packages/#list-packages
pub struct ProjectPackageList {
	id: impl ToString,
	order_by: Option<ProjectPackageListOrderBy>,
	sort: Option<SortDirection>,
	package_name: Option<String>,
	package_version: Option<String>,
	include_versionless: Option<bool>,
	status: Option<PackageStatus>,
}

impl ProjectPackageList {
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
			id,
			order_by: None,
			sort: None,
			package_name: None,
			package_version: None,
			include_versionless: None,
			status: None,
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
	pub fn order_by(mut self, order_by: ProjectPackageListOrderBy) -> Self {
		self.order_by = Some(order_by);
		self
	}

	pub fn sort(mut self, sort: SortDirection) -> Self {
		self.sort = Some(sort);
		self
	}

	pub fn package_type(mut self, package_type: PackageType) -> Self {
		self.package_name = Some(package_type);
		self
	}

	pub fn package_name(mut self, name: &str) -> Self {
		self.package_name = Some(name.to_string());
		self
	}
	pub fn package_version(mut self, version: &str) -> Self {
		self.package_version = Some(version.to_string());
		self
	}
	pub fn include_versionless(mut self, include: bool) -> Self {
		self.include_versionless = Some(include);
		self
	}
}

/// Info needed to list packages of a project.
#[derive(Debug, Clone)]
pub struct PackageAction {
    pub project_id: u64,
    pub package_name: String,
}

impl PackageAction {
    /// Creates a new `PackageAction` instance from a project path.
    ///
    /// # Arguments
    ///
    /// * `project_path` - The path to the project.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `PackageAction` instance or an error.
    pub async fn with_project_path(project_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let project_id = project_get(project_path).await?.id;
        Ok(Self {
            project_id,
            package_name: "".to_string(),
        })
    }

    /// Creates a new `PackageAction` instance from a project ID.
    ///
    /// # Arguments
    ///
    /// * `project_id` - The ID of the project.
    ///
    /// # Returns
    ///
    /// A `PackageAction` instance.
    pub async fn with_project_id(project_id: u64) -> Self {
        Self {
            project_id,
            package_name: "".to_string(),
        }
    }

    /// Sets the package name for the `PackageAction` instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the package.
    ///
    /// # Returns
    ///
    /// The `PackageAction` instance with the updated package name.
    pub fn package_name(mut self, name: &str) -> Self {
        self.package_name = name.to_string();
        self
    }

    /// Lists packages of a project.
    ///
    /// # Arguments
    ///
    /// * `latest` - A flag indicating whether to list only the latest package.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PackageListItem` or an error.
    pub async fn run(self, latest: bool) -> Result<Vec<PackageInfo>, Box<dyn std::error::Error>> {
        let mut url_path = format!(
            "/projects/{}/packages?package_name={}",
            self.project_id, self.package_name
        );
        if latest {
            url_path.push_str("&order_by=created_at&sort=desc&per_page=1");
        }
        let response = httpclient()
            .get(gitlab_api_url(&url_path, None))
            .header("Private-Token", gitlab_token())
            .send()
            .await?;
        let status = response.status();
        if status == 404 {
            return Err("PackageNotFound".into());
        }
        let content = response.text().await?;
        let packages: Vec<PackageInfo> = serde_json::from_str(&content)?;
        Ok(packages)
    }

    /// Lists package files for a specific version.
    ///
    /// # Arguments
    ///
    /// * `version` - The version of the package.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PackageFilesItem` or an error.
    pub async fn package_files_by_version(
        self,
        version: &str,
    ) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
        let project_id = self.project_id;
        let package_files = ListPackageFiles::with_project_id(project_id)
            .await
            .package_version(version)
            .run("", "")
            .await?
            .into_iter()
            .map(|mut x| {
                x.version = Some(version.to_string());
                x.name = Some(self.package_name.clone());
                x
            })
            .collect();
        Ok(package_files)
    }

    /// Lists package files for the latest package.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PackageFilesItem` or an error.
    pub async fn latest_package_files(
        self,
    ) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
        let project_id = self.project_id;
        let latest_package = self.clone().run(true).await?;
        // eprintln!("latest_package: {:?}", latest_package);
        let version = latest_package.first().unwrap().version.clone();
        let latest_package_id = latest_package.first().unwrap().id;
        // eprintln!("latest_package_id: {}", latest_package_id);
        let package_files = package_files_get(project_id, latest_package_id, "", "")
            .await?
            .into_iter()
            .map(|mut x| {
                x.version = Some(version.clone());
                x.name = Some(self.package_name.clone());
                x
            })
            .collect();
        Ok(package_files)
    }

    /// Downloads files that match a regex pattern from the latest package or a specific version.
    ///
    /// # Arguments
    ///
    /// * `pattern` - An optional regex pattern for filtering package files.
    /// * `filename` - An optional exact filename for filtering package files.
    /// * `version` - An optional package version to download files from.
    /// * `latest` - A flag indicating whether to download files from the latest package.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an error.
    pub async fn download_files(
        self,
        output_dir: PathBuf,
        pattern: Option<String>,
        filename: Option<String>,
        version: Option<String>,
        latest: Option<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pattern = pattern.map(|x| Regex::new(&x).unwrap());
        let filter = make_filter(pattern, filename);
        let package_files = if Some(true) == latest {
            self.clone().latest_package_files().await?
        } else if let Some(version) = version {
            eprintln!("download package version: {}", version);
            self.clone().package_files_by_version(&version).await?
        } else {
            return Err("Please provide either package version or latest flag".into());
        };
        let mut outputs = vec![];
        for package_file in &package_files {
            if !filter.filter(package_file) {
                continue;
            }
            let package_file_path = format!(
                "/projects/{}/packages/generic/{}/{}/{}",
                self.project_id,
                package_file.name.clone().unwrap(),
                package_file.version.clone().unwrap(),
                package_file.file_name
            );
            let url = gitlab_api_url(&package_file_path, None);
            let output_file = if output_dir.is_dir() {
                output_dir.join(&package_file.file_name)
            } else {
                eprintln!("Warning: ouput_dir is not a directory, use /tmp as fallback");
                PathBuf::from("/tmp").join(&package_file.file_name)
            };
            let output_str = output_file.as_path().to_str().unwrap().to_string();
            let _ = download_file(&url, &output_file).await?;
            outputs.push(output_str);
        }
        let msg = serde_yaml::to_value(PrintOutput {
            status: "ok".to_string(),
            output: outputs,
        })?;
        println!("{}", serde_yaml::to_string(&msg)?);
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
        let url = gitlab_api_url(&url_path, None);
        let file = tokio::fs::read(file_path).await?;
        let response = httpclient()
            .put(&url)
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
pub async fn download_file<P>(url: &str, output_file: P) -> Result<(), Box<dyn std::error::Error>>
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

/// Info needed to list files of a package.
///
/// This struct holds the necessary information to list files of a specific package in a project.
pub struct ListPackageFiles {
    pub project_id: u64,
    pub package_name: String,
    pub package_version: String,
}

impl ListPackageFiles {
    /// Creates a new instance of `ListPackageFiles` for a given project path.
    ///
    /// # Arguments
    ///
    /// * `project_path` - The path to the project.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `ListPackageFiles` instance or an error.
    pub async fn with_project_path(project_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let project_id = project_get(project_path).await?.id;
        Ok(Self {
            project_id,
            package_name: "".to_string(),
            package_version: "".to_string(),
        })
    }

    /// Creates a new instance of `ListPackageFiles` for a given project ID.
    ///
    /// # Arguments
    ///
    /// * `project_id` - The ID of the project.
    ///
    /// # Returns
    ///
    /// A `ListPackageFiles` instance.
    pub async fn with_project_id(project_id: u64) -> Self {
        Self {
            project_id,
            package_name: "".to_string(),
            package_version: "".to_string(),
        }
    }

    pub fn package_name(mut self, name: &str) -> Self {
        self.package_name = name.to_string();
        self
    }

    pub fn package_version(mut self, version: &str) -> Self {
        self.package_version = version.to_string();
        self
    }

    /// Lists files of a package.
    ///
    /// # Returns
    /// A `Result` containing a vector of `PackageFilesItem` or an error.
    pub async fn run(
        &self,
        order_by: &str,
        sort: &str,
    ) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
        let packages = packages_get(
            self.project_id,
            &[
                ("package_name", self.package_name.as_str()),
                ("package_version", self.package_version.as_str()),
            ],
        )
        .await?;
        let package = packages
            .first()
            .ok_or::<Box<dyn std::error::Error>>("PackageNotFound".into())?;
        package_files_get(self.project_id, package.id, order_by, sort).await
    }
}

/// Fetch information of packages
pub async fn packages_get_helper<I, K, V>(
    project_id: u64,
    path: &str,
    query: I,
) -> Result<Vec<u8>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    K: AsRef<str>,
    V: AsRef<str>,
    I::Item: Borrow<(K, V)>,
{
    let url = Url::parse_with_params(
        &format!(
            "https://gitlab.com/api/v4/projects/{}/packages{}",
            project_id, path
        ),
        query,
    )?;
    let response = httpclient()
        .get(url)
        .header("Private-Token", gitlab_token())
        .send()
        .await?;
    let json_bytes = response.bytes().await?.to_vec();
    Ok(json_bytes)
}

/// Fetch information of packages
pub async fn packages_get<I, K, V>(
    project_id: u64,
    query: I,
) -> Result<Vec<PackageInfo>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    K: AsRef<str>,
    V: AsRef<str>,
    I::Item: Borrow<(K, V)>,
{
    let json = packages_get_helper(project_id, "", query).await?;
    let packages: Vec<PackageInfo> = serde_json::from_slice(&json)?;
    eprintln!(
        "Found {} packages for project {}",
        packages.len(),
        project_id
    );
    Ok(packages)
}

/// Get package files
pub async fn package_files_get(
    project_id: u64,
    package_id: u64,
    order_by: &str,
    sort: &str,
) -> Result<Vec<PackageFileInfo>, Box<dyn std::error::Error>> {
    let path = format!("/{}/package_files", package_id);
    let mut query = vec![];
    if order_by != "" {
        query.push(("order_by", order_by));
    }
    if sort != "" {
        query.push(("sort", sort));
    }
    let json = packages_get_helper(project_id, &path, query).await?;
    let package_files: Vec<PackageFileInfo> = serde_json::from_slice(&json)?;
    eprintln!(
        "Found {} package files for package {} from project {}",
        package_files.len(),
        package_id,
        project_id
    );
    Ok(package_files)
}

/// Enum for sorting package files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PackageFilesSortBy {
    Id,
    CreatedAt,
    FileName,
}
