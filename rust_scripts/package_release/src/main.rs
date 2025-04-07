use anyhow::{Context, Result};
use xshell::{Shell, cmd};

pub fn compress_and_upload() -> Result<()> {
    let sh = Shell::new()?;
    let archs = &["amd64", "arm64"];
    // Get git commit hash
    let commit_hash = cmd!(sh, "git rev-parse --short HEAD")
        .read()
        .context("Failed to get git commit hash")?;

    let mut binary_for_current_arch;
    for arch in archs {
        let binary = format!("glabu-{}", arch);
        let binary_path = PathBuf::from("./target").join(&binary);
        // Compress the binary with UPX
        println!("Compressing the binary");
        cmd!(sh, "upx {binary_path}")
            .run()
            .context("Failed to compress binary with UPX")
            .map_err(|e| {
                eprintln!("Ignore error from upx: {}", e);
                Ok(())
            })?;

        if osarch::current_os_arch().is_match(arch) {
            binary_for_current_arch = binary_path.clone();
            break;
        }
    }

    for arch in archs {
        let binary = format!("glabu-{}", arch);
        let binary_path = PathBuf::from("./target").join(&binary);
        // Upload to GitLab
        println!("Uploading the binary to the gitlab...");
        let file_name = format!(
            "{}-{}",
            binary_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("glabu"),
            arch
        );

        cmd!(
            sh,
            "{binary_for_current_arch} package-upload puterize/prebuilt --package-name glabu --package-version {commit_hash} --file-name {file_name} --file-path {binary_path}"
        )
        .run()
        .context("Failed to upload binary to GitLab")?;
    }

    // Print installation instructions
    println!("If you want to install glabu globally, run the following command:");
    println!(
        "sudo install {} /usr/local/bin/glabu",
        binary_for_current_arch.display()
    );

    Ok(())
}

fn main() -> Result<()> {
    // Build and push images
    compress_and_upload().context("Failed to build and push images")?;
    println!("All binaries uploaded as packages successfully.");
    Ok(())
}
