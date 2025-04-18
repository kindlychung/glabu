use std::path::PathBuf;

use crate::endpoints::{
    packages::{GenericPackageOp, ProjectPackageListOp},
    projects::{ProjectCreate, ProjectDelete, ProjectForkPrivate, projects_search},
};
use clap::Parser;

fn encode_project_id(project: &str) -> String {
    let mut project_id = project.to_string();
    if project_id.contains('/') {
        project_id = project_id.replace('/', "%2F");
    }
    project_id
}

use clap::Subcommand;

use crate::models::ProjectVisibility;

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

    /// Create a private fork of a project
    ProjectForkPrivate {
        /// Url of the project to fork
        #[arg(short = 'u', long)]
        project_url: String,
        /// Name of the forked project
        #[arg(short = 'n', long)]
        targe_name: String,
        /// Description of the forked project
        #[arg(short = 'd', long, default_value = "")]
        description: String,
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
    /// Generates shell completion scripts
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

pub async fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::PackageDownload {
            project,
            package_name,
            package_version,
            latest,
            package_file,
            regex,
            output_dir,
        } => {
            let project = encode_project_id(&project);
            let mut pf = GenericPackageOp::new(&project, &package_name, "");
            pf.package_version = package_version;
            if latest {
                pf.package_version = None;
            }
            pf.download_files(output_dir, regex, package_file).await?;
        }
        Commands::PackageUpload {
            project,
            package_name,
            package_version,
            file_path,
            file_name,
        } => {
            let project = encode_project_id(&project);
            let generic_package_op = GenericPackageOp::new(&project, &package_name, "");
            let file_path: PathBuf = PathBuf::from(&file_path);
            if !file_path.exists() {
                return Err(format!("File not found: {}", &file_path.display()).into());
            }
            let file_name = file_name.unwrap_or_else(|| {
                file_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .ok_or("File name not found")
                    .unwrap()
            });
            generic_package_op
                .upload_package_file(&package_version, &file_name, file_path)
                .await?;
        }
        Commands::ProjectCreate {
            project,
            group,
            description,
            visibility,
            mirror_to_github,
        } => {
            let project = encode_project_id(&project);
            let project_action = match group {
                Some(group) => ProjectCreate::for_group(&project, &group)
                    .await?
                    .description(&description)
                    .visibility(visibility),
                None => ProjectCreate::new(&project)
                    .description(&description)
                    .visibility(visibility),
            };
            let res = project_action.run(mirror_to_github).await?;
            let res_json = serde_json::to_string_pretty(&res)?;
            println!("{}", res_json);
        }
        Commands::ProjectDelete { project } => {
            let project = encode_project_id(&project);
            ProjectDelete::new(&project).await?.run().await?;
        }
        Commands::ProjectSearch { term } => {
            let res = projects_search(&term, true).await?;
            let res_json = serde_json::to_string_pretty(&res)?;
            println!("{}", res_json);
        }
        Commands::PackageFileList {
            project,
            package_name,
            package_version,
        } => {
            let project = encode_project_id(&project);
            let package_list_op = ProjectPackageListOp::new(&project)
                .package_name(Some(package_name.as_str().into()))
                .package_version(Some(package_version.as_str().into()));
            let files = package_list_op.list().await?;
            let files_json = serde_json::to_string_pretty(&files)?;
            println!("{}", files_json);
        }
        Commands::ProjectForkPrivate {
            project_url,
            targe_name,
            description,
        } => {
            let fork_op =
                ProjectForkPrivate::new(&project_url, &targe_name).description(&description);
            let res = fork_op.run().await?;
            let res_json = serde_json::to_string_pretty(&res)?;
            println!("{}", res_json);
        }
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let cmd_name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, cmd_name, &mut std::io::stdout());
        }
    }
    Ok(())
}
