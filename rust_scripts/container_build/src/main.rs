use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

static MESSAGES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
pub fn messages() -> &'static Mutex<Vec<String>> {
    MESSAGES.get_or_init(|| Mutex::new(Vec::new()))
}

fn build_and_push_images() -> Result<()> {
    let sh = Shell::new()?;
    let registry = "registry.gitlab.com/puterize/glabu";

    // Check if podman is installed
    if cmd!(sh, "podman --version").run().is_err() {
        eprintln!("Podman is not installed. Please install it first.");
        std::process::exit(1);
    }

    // Get git commit hash
    let commit_hash = cmd!(sh, "git rev-parse --short HEAD")
        .read()
        .context("Failed to get git commit hash")?;

    let tag_root = format!("{}:{}", registry, commit_hash);

    // Check if manifest exists and remove it
    if cmd!(sh, "podman manifest exists {tag_root}").run().is_ok() {
        println!("Manifest {} already exists, removing it first...", tag_root);
        cmd!(sh, "podman manifest rm {tag_root}")
            .run()
            .context("Failed to remove existing manifest")?;
    }

    // Create new manifest
    cmd!(sh, "podman manifest create {tag_root}")
        .run()
        .context("Failed to create manifest")?;

    for arch in &["amd64", "arm64"] {
        let tag = format!("{}-{}", tag_root, arch);
        println!("Building image {}...", tag);
        cmd!(
            sh,
            "podman build --platform linux/{arch} --build-arg TARGETPLATFORM=linux/{arch} -t {tag} -f glabu/Dockerfile ./glabu"
        )
        .run()
        .context(format!("Failed to build image for {}", arch))?;

        println!("Pushing image: {}", tag);
        cmd!(sh, "podman push {tag}")
            .run()
            .context(format!("Failed to push image for {}", arch))?;

        // Copy binaries to target folder if architecture matches
		// Note that since we can only run the docker image for the current architecture,
		// but we need to copy the binaries for both architectures, so we made sure that
		// the docker image for arm64 also contains the amd64 binary
		// and vice versa. This way we can copy both binaries from the same image.
		// This means we can run the arm64 image and copy the amd64 binary from it
		// and vice versa.
        if osarch::current_arch().is_match(arch) {
            // Create container to extract binary
            let container_id = cmd!(sh, "podman create {tag}")
                .read()
                .context(format!("Failed to create container for {}", arch))?;

            // Copy binary from container
            sh.create_dir("./target")?;
            cmd!(
                sh,
                "podman cp {container_id}:/app/glabu_aarch64 ./target/"
            )
            .run()
            .context("Failed to copy binary glabu_aarch64")?;
            cmd!(
                sh,
                "podman cp {container_id}:/app/glabu_x86_64 ./target/"
            )
            .run()
            .context("Failed to copy binary glabu_x86_64")?;

            // Clean up container
            cmd!(sh, "podman rm -v {container_id}")
                .run()
                .context(format!("Failed to remove container {}", container_id))?;
			let arch = cmd!(sh, "arch").read().context("Failed to get architecture")?;
			
			let msg = format!(r####"
>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
To install the glabu binary for {arch}:

sudo install ./target/glabu_{arch} /usr/local/bin/glabu
>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
			"####);
			messages().lock().unwrap().push(msg);
        }
		// Add to manifest
		cmd!(sh, "podman manifest add {tag_root} {tag}")
			.run()
			.context(format!("Failed to add {} to manifest", arch))?;
    }

    println!("Pushing manifest: {}", tag_root);
    cmd!(sh, "podman manifest push {tag_root}")
        .run()
        .context("Failed to push manifest")?;

    Ok(())
}

fn main() -> Result<()> {
    // Build and push images
    build_and_push_images().context("Failed to build and push images")?;
    println!("All images built and pushed successfully.");

	for msg in messages().lock().unwrap().iter() {
		println!("{}", msg);
	}

    Ok(())
}
