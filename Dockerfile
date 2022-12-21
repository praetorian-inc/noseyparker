ARG RUST_VER=1.65
ARG VECTORSCAN_VER=5.4.8
ARG VECTORSCAN_SHA=71fae7ee8d63e1513a6df762cdb5d5f02a9120a2422cf1f31d57747c2b8d36ab

################################################################################
# Base stage
################################################################################
FROM rust:$RUST_VER AS base

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
FROM base AS dependencies

WORKDIR "/noseyparker"

COPY ["Cargo.toml", "Cargo.lock",  "./"]

# Make directory structure
RUN mkdir -p  ./src/bin/noseyparker &&\
    mkdir -p ./benches &&\
    # Create stub files for build
    touch ./benches/microbench.rs ./src/lib.rs &&\
    echo "fn main() {}" > ./src/bin/noseyparker/main.rs &&\
    # Run the build
    cargo build --bin noseyparker --release

################################################################################
# Build application
################################################################################
FROM dependencies AS build

WORKDIR "/noseyparker"

COPY . .

# Update file timestamps
RUN touch ./benches/microbench.rs ./src/lib.rs ./src/bin/noseyparker/main.rs

RUN cargo build --release

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim

COPY --from=build /noseyparker/target/release/noseyparker /usr/bin/noseyparker

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
