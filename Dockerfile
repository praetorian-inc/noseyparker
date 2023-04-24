################################################################################
# Build `noseyparker`
################################################################################
FROM rust:1.68 AS builder

# Install dependencies
#
# Note: clang is needed for `bindgen`, used by `vectorscan-sys`.
RUN apt-get update &&\
    apt-get install -y \
        cmake \
        ninja-build \
        &&\
    apt-get clean

WORKDIR "/noseyparker"

COPY . .

RUN CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    cargo install --root /usr/local --profile release --locked --path crates/noseyparker-cli

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim as runner

# Add `git` so that noseyparker's git and github integration works
RUN apt-get update &&\
    apt-get install -y git &&\
    apt-get clean

COPY --from=builder /usr/local/bin/noseyparker /usr/local/bin/noseyparker

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
