#!/bin/bash

cd "$(dirname "$0")/.." || exit 1

arch=$(uname -m)
echo "Running on architecture: $arch"
binary=$"target/$arch-unknown-linux-musl/release/glabu"
# sudo chown "$USER" "$binary"
commit_hash=$(git rev-parse --short HEAD)
echo "Compressing the binary"
upx "$binary"
echo "Uploading the binary to the gitlab..."
"$binary" package-upload fa_rfro_mwrong/prebuilt --package-name glabu --package-version "$commit_hash" --file-name "$(basename "$binary")-$(uname -m)" --file-path "$binary"
echo "If you want install glabu globally, run the following command:"
echo "sudo install $binary /usr/local/bin/glabu"
