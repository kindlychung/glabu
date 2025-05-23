# (1) Base stage with dependencies
FROM --platform=$BUILDPLATFORM public.ecr.aws/docker/library/rust:alpine AS chef
WORKDIR /app
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig zig
RUN cargo install --locked cargo-zigbuild cargo-chef
RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

# # (2) Set OpenSSL env vars for static linking
# ENV OPENSSL_NO_VENDOR=0
# ENV OPENSSL_STATIC=1
# ENV OPENSSL_LIB_DIR=/usr/lib
# ENV OPENSSL_INCLUDE_DIR=/usr/include


# (3) Planner stage (unchanged)
FROM chef AS planner
COPY . .
RUN ls -lah
RUN cargo chef prepare --recipe-path recipe.json

# (4) Builder stage (with OpenSSL fixes)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json --release --zigbuild \
  --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-musl

# (5) Final build with static OpenSSL
COPY . .
RUN ls -lah
RUN cargo zigbuild --package glabu -r --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-musl && \
  mkdir /app/linux && \
  cp target/aarch64-unknown-linux-musl/release/glabu /app/glabu_aarch64 && \
  cp target/x86_64-unknown-linux-musl/release/glabu /app/glabu_x86_64 

# (6) Runtime stage (unchanged)
FROM --platform=${TARGETPLATFORM} alpine:latest AS runtime
ARG TARGETPLATFORM
RUN echo "**** TARGETPLATFORM: ${TARGETPLATFORM} ****"
WORKDIR /app
# change the name of the binary to match the output of the `arch` command
COPY --from=builder /app/glabu_aarch64 /app/
COPY --from=builder /app/glabu_x86_64 /app/
CMD "/app/glabu_$(arch)"