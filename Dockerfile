ARG RUST_VER=1.65

################################################################################
# Build Rust dependencies, caching stage
################################################################################
# This stage exists so that dependencies of Nosey Parker can be preserved in
# the Docker cache.
#
# Building dependencies only is not naturally supported out-of-the box with
# Cargo, and so requires some machinations.

FROM rust:$RUST_VER AS builder

# Install dependencies
#
# Note: clang is needed for `bindgen`, used by `vectorscan-sys`.
RUN apt-get update &&\
    apt-get install -y \
    build-essential \
    clang \
    cmake \
    git \
    ninja-build \
    pkg-config &&\
    apt-get clean

WORKDIR "/noseyparker"


# # Copy Cargo files for downloading dependencies.
# #
# # This causes more Docker caching, allowing `updating crates.io index` to be
# # avoided unless project dependencies change.
# #
# # See https://github.com/rust-lang/cargo/issues/2644 for an odyssey about this
# # dependency caching idea.
# COPY ["Cargo.toml", "Cargo.lock",  "."]
# COPY ["vectorscan/Cargo.toml", "vectorscan/Cargo.toml"]
# COPY ["vectorscan-sys/Cargo.toml", "vectorscan-sys/Cargo.toml"]
#
# # Have cargo download the dependencies
# #
# # Note: we stub benches to avoid error: can't find `microbench` bench at `benches/microbench.rs`
# # Note: we stub the lib files as well to avoid errors
# RUN mkdir -p benches && touch benches/microbench.rs &&\
#     mkdir -p src && touch src/lib.rs &&\
#     mkdir -p vectorscan/src && touch vectorscan/src/lib.rs &&\
#     mkdir -p vectorscan-sys/src && touch vectorscan-sys/src/lib.rs &&\
#     cargo fetch
#
# # Build the dependencies
# #
# # Note: we have to stub the main source to have this work without error
# # Note: we have to stub the build.rs script for vectorscan-sys to have this work without error
# RUN mkdir -p src/bin/noseyparker && echo "fn main() {}" > src/bin/noseyparker/main.rs &&\
#     echo "fn main() {}" > vectorscan-sys/build.rs &&\
#     cargo build --profile release --locked

COPY . .

# # Update file timestamps on any stubs from previous steps. This will cause
# # compilation to happen if needed, otherwise Cargo build can skip files with
# # timestamps older than the previous stage's build command
# RUN touch \
#     benches/microbench.rs \
#     src/lib.rs \
#     src/bin/noseyparker/main.rs \
#     vectorscan/src/lib.rs \
#     vectorscan-sys/build.rs
#     vectorscan-sys/src/lib.rs

# the net.git-fetch-with-cli=true bit here is to avoid OOM when building for non-native platforms using qemu
# https://github.com/rust-lang/cargo/issues/10781#issuecomment-1441071052
RUN cargo install --config net.git-fetch-with-cli=true --root /usr/local --profile release --locked --path crates/noseyparker-bin

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim as runner

# Add `git` so that noseyparker's git and github integration works
RUN apt-get update && apt-get install -y git && apt-get clean

COPY --from=builder /usr/local/bin/noseyparker /usr/local/bin/noseyparker

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
