################################################################################
# Build `noseyparker`
#
# We use the oldest Debian-based image that can build Nosey Parker without trouble.
# This is done in an effort to link against an older glibc, so that the built
# binary (which is *not* statically linked, but does not dynamically link with
# non-standard runtime libraries) can be copied out of the container and run on
# more Linux machines than would otherwise be possible.
#
# See https://github.com/praetorian-inc/noseyparker/issues/58.
################################################################################
FROM rust:1.71-bullseye AS builder

# Install dependencies
#
# Note: clang is needed for `bindgen`, used by `vectorscan-sys`.
RUN apt-get update &&\
    apt-get install -y \
        cmake \
        zsh \
        &&\
    apt-get clean

WORKDIR "/noseyparker"

COPY . .

RUN ./scripts/create-release.zsh && cp -r release /release

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM debian:11-slim as runner

# Add `git` so that noseyparker's git and github integration works
RUN apt-get update &&\
    apt-get install -y git &&\
    apt-get clean

COPY --from=builder /release /usr/local/

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
