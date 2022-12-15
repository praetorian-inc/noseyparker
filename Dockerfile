################################################################################
# Build Nosey Parker
################################################################################
FROM rust:1.65 as build

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    libboost-dev \
    ninja-build \
    pkg-config \
    ragel

# Build vectorscan from source
WORKDIR "/vectorscan"
ADD https://github.com/VectorCamp/vectorscan/archive/refs/tags/vectorscan/5.4.8.tar.gz vectorscan.tar.gz
RUN echo '71fae7ee8d63e1513a6df762cdb5d5f02a9120a2422cf1f31d57747c2b8d36ab vectorscan.tar.gz' | sha256sum -c && \
    tar --strip-components 1 -xzf vectorscan.tar.gz && \
    cmake -S . -B build -GNinja -DFAT_RUNTIME=OFF && \
    cmake --build build

# Build Nosey Parker
WORKDIR "/noseyparker"
COPY . .
# build against from-source vectorscan
ENV HYPERSCAN_ROOT "/vectorscan/build"
# XXX it would be nice if this could store crates.io index and dependency builds in the Docker cache
RUN cargo build --release

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim
COPY --from=build /noseyparker/target/release/noseyparker /usr/bin/noseyparker

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
