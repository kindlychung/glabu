use anyhow::{Context, Result};
use xshell::{Shell, cmd};

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
            "podman build --platform linux/{arch} -t {tag} -f glabu/Dockerfile ./glabu"
        )
        .run()
        .context(format!("Failed to build image for {}", arch))?;

        println!("Pushing image: {}", tag);
        cmd!(sh, "podman push {tag}")
            .run()
            .context(format!("Failed to push image for {}", arch))?;

        // Create container to extract binary
        let container_id = cmd!(sh, "podman create {tag}")
            .read()
            .context(format!("Failed to create container for {}", arch))?;

        // Copy binary from container
        sh.create_dir("./target")?;
        cmd!(
            sh,
            "podman cp {container_id}:/app/prog ./target/glabu-{arch}"
        )
        .run()
        .context(format!("Failed to copy binary for {}", arch))?;

        // Install binary locally if architecture matches
        if osarch::current_os_arch().is_match(arch) {
            println!("Installing the binary locally");
            let cwd = sh.current_dir();
            let binary_name = cwd.file_name().and_then(|n| n.to_str()).unwrap_or("glabu");

            cmd!(
                sh,
                "podman cp {container_id}:/app/prog /usr/local/bin/{binary_name}"
            )
            .run()
            .context("Failed to install binary locally")?;
        }

        // Clean up container
        cmd!(sh, "podman rm -v {container_id}")
            .run()
            .context(format!("Failed to remove container for {}", arch))?;

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

    Ok(())
}
