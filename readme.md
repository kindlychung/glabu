# glabu: cli for interacting with gitlab api

`glabu` is a tool helps me with some gitlab related operations.

## Compile

```bash
# produce dynamic linked binary
# the --package arg is given here because the cli is in the glabu subproject/package
# if you don't supply the --package arg, cargo will build all packages, which is in itself not a problem, 
# it just takes a bit more time
cargo build --release --package glabu 
# produce multiarch, statically linked binary using zigbuild
cargo zigbuild -r --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-musl  --package glabu 
```

## Shell completion

## Upload package to gitlab

```bash
cargo run -p package_release
```

