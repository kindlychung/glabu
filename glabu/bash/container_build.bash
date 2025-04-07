#!/bin/bash

set -e
cd "$(dirname "$0")/.." || exit 1



reg="registry.gitlab.com/puterize/glabu"
commit_hash=$(git rev-parse --short HEAD)
tag_root="$reg:$commit_hash"

if podman manifest exists "$tag_root"; then
	echo "Manifest ${tag_root} already exists, removing it first..."
	podman manifest rm "$tag_root"
fi
podman manifest create "$tag_root"
for arch in amd64 arm64; do
	tag="$tag_root-${arch}"
	echo "Building image ${tag}..."
	podman build --platform linux/$arch -t "$tag" .
	echo "Pushing image: $tag"
	podman push "$tag"
	container_id=$(podman create "$tag")
	podman cp "$container_id:/app/prog" "./target/glabu-${arch}"
	if [[ "$arch" == "$(uname -m)" ]]; then
		echo "Installing the binary locally"
		podman cp "$container_id:/app/prog" "/usr/local/bin/$(basename "$PWD")"
	fi
	podman rm -v "$container_id"
	podman manifest add "$tag_root" "$tag"
done
echo "Pushing manifest: $tag_root"
podman manifest push "$tag_root"
