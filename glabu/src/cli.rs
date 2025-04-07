use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::models::{Project, ProjectVisibility};

/// GitLab Utility (glabu) - A command-line tool for interacting with GitLab api v4
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new project
    ProjectCreate {
        /// Name of the project
        project: String,
        /// Group of the project
        #[arg(short = 'g', long)]
        group: Option<String>,
        /// Description of the project
        #[arg(short = 'd', long, default_value = "")]
        description: String,
        #[arg(short = 'v', long, default_value_t = ProjectVisibility::Private, value_enum)]
        visibility: ProjectVisibility,
        #[arg(short, long, default_value_t = false)]
        mirror_to_github: bool,
    },
    /// Delete a project
    ProjectDelete {
        /// Full path to the project, for example: owner/project
        project: String,
    },
    /// Search for project
    ProjectSearch {
        /// Query term
        term: String,
    },
    // /// List all packages in the project's package registry
    // List {
    //     /// Full path to the project, for example: owner/project
    //     project: String,
    //     /// Name of the package
    //     #[arg(short = 'n', long)]
    //     package_name: String,
    //     /// Show the latest package
    //     #[arg(short = 'l', long)]
    //     latest: bool,
    // },
    /// Download package file(s)
    PackageDownload {
        /// Full path to the project, for example: owner/project
        project: String,
        /// Name of the package
        #[arg(short = 'n', long)]
        package_name: String,
        /// Version of the package
        #[arg(short = 'v', long)]
        package_version: Option<String>,
        #[arg(short, long, default_value_t = false)]
        latest: bool,
        /// Specify the package file to download
        #[arg(short = 'f', long)]
        package_file: Option<String>,
        /// Filename regex to filter files
        #[arg(short = 'r', long)]
        regex: Option<String>,
        /// Output file directory
        #[arg(short = 'o', long, default_value = "/tmp")]
        output_dir: PathBuf,
    },
    /// Upload a single package file
    PackageUpload {
        /// Full path to the project, for example: owner/project
        project: String,
        /// Name of the package
        #[arg(short = 'n', long)]
        package_name: String,
        /// Version of the package
        #[arg(short = 'v', long)]
        package_version: String,
        /// Specify the package file to upload
        #[arg(short = 'f', long)]
        file_path: String,
        #[arg(short = 'm', long)]
        file_name: Option<String>,
    },
    /// List files of a given package (with a given version)
    PackageFileList {
        /// Full path to the project, for example: owner/project
        project: String,
        /// Name of the package
        #[arg(short = 'n', long)]
        package_name: String,
        /// Version of the package
        #[arg(short = 'v', long)]
        package_version: String,
    },
}
