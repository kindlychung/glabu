use std::path::PathBuf;

use clap::Parser;
use glabu::{
    cli::{Cli, Commands},
    endpoints::{
        packages::{GenericPackageOp, ProjectPackageListOp},
        projects::{ProjectCreate, ProjectDelete, ProjectForkPrivate, projects_search},
    },
};

fn encode_project_id(project: &str) -> String {
	let mut project_id = project.to_string();
	if project_id.contains('/') {
		project_id = project_id.replace('/', "%2F");
	}
	project_id
}

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
    }
    Ok(())
}
