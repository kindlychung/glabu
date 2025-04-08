use std::path::PathBuf;

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

pub fn compress_and_upload() -> Result<()> {
    let sh = Shell::new()?;
    let archs = &["amd64", "arm64"];
    // Get git commit hash
    let commit_hash = cmd!(sh, "git rev-parse --short HEAD")
        .read()
        .context("Failed to get git commit hash")?;

    // find out which binary to use for glabu to upload itself
    let mut binary_for_current_arch: Option<PathBuf> = None;
    for arch in archs {
        let binary = format!("glabu-{}", arch);
        let binary_path = PathBuf::from("./target").join(&binary);
        // Compress the binary with UPX
        println!("Compressing the binary");
        cmd!(sh, "upx {binary_path}")
            .run()
            .context("Failed to compress binary with UPX")
            .map_or_else(|_| Ok::<(), anyhow::Error>(()), |_| Ok(()))?;

        let osarch_regex = osarch::current_arch();
        dbg!(&osarch_regex);
        if osarch_regex.is_match(arch) {
            // it's ok to move binary_path out of the loop, since it's not used after this
            binary_for_current_arch = Some(binary_path);
            break;
        } else {
            eprintln!(
                "Binary for {} is not compatible with the current architecture",
                arch
            );
        }
    }

    if binary_for_current_arch.is_none() {
        return Err(anyhow::anyhow!(
            "No binary found for the current architecture"
        ));
    }
    let binary_for_current_arch = binary_for_current_arch.unwrap();

    for arch in archs {
        let binary = format!("glabu-{}", arch);
        let binary_path = PathBuf::from("./target").join(&binary);
        // Upload to GitLab
        let file_name = binary_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("glabu");
        println!("Uploading the binary {} to the gitlab...", &file_name);
        cmd!(
            sh,
            "{binary_for_current_arch} package-upload puterize/prebuilt --package-name glabu --package-version {commit_hash} --file-name {file_name} --file-path {binary_path}"
        )
        .run()
        .context("Failed to upload binary to GitLab")?;
    }

    // Print installation instructions
    println!(
        r###"
>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
If you want to install glabu globally, run the following command:
	"###
    );
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
