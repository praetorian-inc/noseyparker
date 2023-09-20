################################################################################
# Build `noseyparker`
#
# We use the alpine current, since its smaller than most debian releases.
################################################################################
FROM rust:1.72-alpine3.18 AS builder

# Install dependencies
#
# Note: clang is needed for `bindgen`, used by `vectorscan-sys`.
RUN apk add --no-cache --no-interactive \
        cmake \
        ninja-build \
        musl-dev \
        make\ 
        openssl \
        build-base \
        openssl-dev \
        git \
        perl \
        &&\
    apk cache clean 

WORKDIR "/noseyparker"

COPY . .

RUN CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    cargo install --root /usr/local --profile release --features release --locked --path crates/noseyparker-cli

################################################################################
# Build a smaller image just for running the `noseyparker` binary
################################################################################
FROM alpine:3.18 as runner

# Add `git` so that noseyparker's git and github integration works
RUN apk add --no-cache --no-interactive git
COPY --from=builder /usr/local/bin/noseyparker /usr/local/bin/noseyparker

# Tip when running: use a volume mount: `-v "$PWD:/scan"` to make for handling of paths on the command line
WORKDIR "/scan"

ENTRYPOINT ["noseyparker"]
