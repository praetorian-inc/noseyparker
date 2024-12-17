################################################################################
# Build an image used for all building actions
#
# We use the oldest Debian-based image that can build Nosey Parker without trouble.
# This is done in an effort to link against an older glibc, so that the built
# binary (which is *not* statically linked, but does not dynamically link with
# non-standard runtime libraries) can be copied out of the container and run on
# more Linux machines than would otherwise be possible.
#
# See https://github.com/praetorian-inc/noseyparker/issues/58.
################################################################################
FROM rust:1.81-bullseye AS chef
# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN cargo install --locked cargo-chef

WORKDIR "/noseyparker"

################################################################################
# Generate a `recipe.json` file to capture the set of information required to
# build the dependencies of `noseyparker`.
################################################################################
FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

################################################################################
# Build `noseyparker`
################################################################################
FROM chef AS builder

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
        cmake \
        libboost-all-dev \
        zsh \
        && \
    apt-get clean

COPY --from=planner /noseyparker/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer
# Arguments match arguments specified in `create-release.zsh` script
RUN cargo chef cook --locked --profile release --features "release" --recipe-path recipe.json

COPY . .

RUN ./scripts/create-release.zsh && cp -r release /release

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim AS runner

# Add `git` so that noseyparker's git and github integration works
RUN apt-get update && \
    apt-get install -y git && \
    apt-get clean

COPY --from=builder /release /usr/local/

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
