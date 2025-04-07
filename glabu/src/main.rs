use std::path::PathBuf;

use clap::Parser;
use glabu::{
    cli::{Cli, Commands},
    endpoints::{
        packages::{ListPackageFiles, PackageAction},
        projects::{ProjectCreate, ProjectDelete, ProjectSearch},
    },
    models::ProjectVisibility,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            let pf = PackageAction::with_project_path(&project)
                .await?
                .package_name(package_name.as_str());

            pf.download_files(
                output_dir,
                regex,
                package_file,
                package_version,
                Some(latest),
            )
            .await?;
        }
        Commands::PackageUpload {
            project,
            package_name,
            package_version,
            file_path,
            file_name,
        } => {
            let pf = PackageAction::with_project_path(&project)
                .await?
                .package_name(package_name.as_str());
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
            pf.upload_package_file(&package_version, &file_name, file_path)
                .await?;
        }
        Commands::ProjectCreate {
            project,
            group,
            description,
            visibility,
            mirror_to_github,
        } => {
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
            ProjectDelete::new(&project).await?.run().await?;
        }
        Commands::ProjectSearch { term } => {
            let res = ProjectSearch::new(&term).run().await?;
            let res_json = serde_json::to_string_pretty(&res)?;
            println!("{}", res_json);
        }
        Commands::PackageFileList {
            project,
            package_name,
            package_version,
        } => {
            let files = PackageAction::with_project_path(&project)
                .await?
                .package_name(package_name.as_str())
                .package_files_by_version(package_version.as_str())
                .await?;
            let files_json = serde_json::to_string_pretty(&files)?;
            println!("{}", files_json);
        }
    }
    Ok(())
}
