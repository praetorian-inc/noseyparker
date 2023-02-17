ARG RUST_VER=1.65
ARG VECTORSCAN_VER=5.4.8
ARG VECTORSCAN_SHA=71fae7ee8d63e1513a6df762cdb5d5f02a9120a2422cf1f31d57747c2b8d36ab

################################################################################
# Base stage
################################################################################
FROM rust:$RUST_VER AS base_builder

ARG VECTORSCAN_VER
ARG VECTORSCAN_SHA

ENV HYPERSCAN_ROOT "/vectorscan/build"

WORKDIR "/vectorscan"

ADD https://github.com/VectorCamp/vectorscan/archive/refs/tags/vectorscan/$VECTORSCAN_VER.tar.gz ./vectorscan.tar.gz

# Install dependencies
RUN apt-get update &&\
    apt-get install -y \
    build-essential \
    cmake \
    git \
    libboost-dev \
    ninja-build \
    pkg-config \
    ragel &&\
    apt-get clean &&\
    # Build vectorscan from source
    echo "$VECTORSCAN_SHA vectorscan.tar.gz" | sha256sum -c &&\
    tar --strip-components 1 -xzf vectorscan.tar.gz &&\
    rm -rf vectorscan.tar.gz &&\
    cmake -S . -B build -GNinja -DCMAKE_BUILD_TYPE=Release -DFAT_RUNTIME=OFF &&\
    cmake --build build

################################################################################
# Build Rust dependencies, caching stage
################################################################################
# This stage exists so that dependencies of Nosey Parker can be preserved in
# the Docker cache.
#
# Building dependencies only is not naturally supported out-of-the box with
# Cargo, and so requires some machinations.

FROM base_builder AS dependencies_builder

WORKDIR "/noseyparker"

# Copy Cargo files for downloading and building dependencies
COPY ["Cargo.toml", "Cargo.lock",  "./"]

# Create stub directory structure and files to cause Cargo to download and
# compile dependencies
RUN mkdir -p  ./src/bin/noseyparker &&\
    mkdir -p ./benches &&\
    # Benches to avoid error:
    # can't find `microbench` bench at `benches/microbench.rs`
    touch ./benches/microbench.rs &&\
    # Lib stub to avoid:
    # error: couldn't read src/lib.rs
    touch ./src/lib.rs &&\
    # Stub main required for compile
    echo "fn main() {}" > ./src/bin/noseyparker/main.rs &&\
    # Run the build
    cargo build --release --profile release --locked

################################################################################
# Build application
################################################################################
FROM dependencies_builder AS app_builder

WORKDIR "/noseyparker"

COPY . .

# Update file timestamps on any stubs from previous stage. This will cause
# compilation to happen if needed, otherwise Cargo build can skip files with
# timestamps older than the previous stage's build command
RUN touch \
    ./benches/microbench.rs \
    ./src/lib.rs \
    ./src/bin/noseyparker/main.rs

RUN cargo install --root /usr/local --profile release --locked --path .

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim

COPY --from=app_builder /usr/local/bin/noseyparker /usr/local/bin/noseyparker

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
