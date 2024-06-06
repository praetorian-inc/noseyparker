#!/bin/zsh

set -eu

fatal() {
    echo >&2 "error: $@"
    exit 1
}

banner() {
    echo "################################################################################"
    echo "# $@"
    echo "################################################################################"
}

ensure_has_program() {
    prog="$1"
    command -v "$prog" >/dev/null || fatal "missing program $prog"
}


# The directory containing this script
HERE=${0:a:h}

# To include debug symbols in the release?
INCLUDE_DEBUG=1

################################################################################
# Parse arguments
################################################################################
while (( $# > 0 )); do
    case $1 in
        --no-debug)
            INCLUDE_DEBUG=0
            shift
        ;;

        *)
            fatal "unknown argument '$1'"
        ;;
    esac
done


################################################################################
# Determine build configuration
################################################################################
# Figure out which platform we are creating a release for
case $(uname) in
    Darwin*)
        PLATFORM="macos"
        CARGO_FEATURES="release"
    ;;

    Linux*)
        PLATFORM="linux"
        CARGO_FEATURES="release"
    ;;

    *)
        fatal "unknown platform"
    ;;
esac

################################################################################
# Environment sanity checking
################################################################################
if [[ $PLATFORM == 'linux' ]]; then
    ensure_has_program ldd
    ensure_has_program readelf
elif [[ $PLATFORM == 'macos' ]]; then
else
    fatal "unknown platform $PLATFORM"
fi
ensure_has_program cmake
ensure_has_program make
ensure_has_program sha256sum

banner "Build configuration"
echo "uname: $(uname)"
echo "uname -p: $(uname -p)"
echo "arch: $(arch)"
echo "PLATFORM: $PLATFORM"
echo "CARGO_FEATURES: $CARGO_FEATURES"
echo "INCLUDE_DEBUG: $INCLUDE_DEBUG"


################################################################################
# Create release directory tree
################################################################################
# Go to the repository root
cd "$HERE/.."

# Where should the release output get put?
RELEASE_DIR="release"

# Where does `cargo` build stuff?
CARGO_BUILD_DIR="target/release"

# What is the name of the program?
NOSEYPARKER="noseyparker"

mkdir "$RELEASE_DIR" || fatal "could not create release directory"
mkdir "$RELEASE_DIR"/{bin,share,share/completions,share/man,share/man/man1,share/"${NOSEYPARKER}"} || fatal "could not create release directory tree"

################################################################################
# Build release version of noseyparker
#
# WARNING: If the invocation below changes, update the Dockerfile as well.
################################################################################
banner "Building release with Cargo"
cargo build --locked --profile release --features "$CARGO_FEATURES" || fatal "failed to build ${NOSEYPARKER}"

################################################################################
# Copy artifacts into the release directory tree
################################################################################
banner "Assembling release dir"
NP="$PWD/$RELEASE_DIR/bin/${NOSEYPARKER}"
cp -p "$CARGO_BUILD_DIR/noseyparker-cli" "$NP" || fatal "failed to copy ${NOSEYPARKER}"

# Copy CHANGELOG.md, LICENSE, and README.md
cp -p CHANGELOG.md LICENSE README.md "$RELEASE_DIR/" || fatal "failed to copy assets"

if (( $INCLUDE_DEBUG )); then
    if [[ $PLATFORM == 'linux' ]]; then
        cp -rp "$CARGO_BUILD_DIR/noseyparker-cli.dwp" "$NP.dwp" || fatal "failed to copy ${NOSEYPARKER}.dwp"
    elif [[ $PLATFORM == 'macos' ]]; then
        cp -rp "$CARGO_BUILD_DIR/noseyparker-cli.dSYM" "$NP.dSYM" || fatal "failed to copy ${NOSEYPARKER}.dSYM"
    else
        fatal "unknown platform $PLATFORM"
    fi
fi

################################################################################
# Generate shell completion scripts
################################################################################
banner "Generating shell completion scripts"
for SHELL in bash zsh fish powershell elvish; do
    "$NP" generate shell-completions --shell "$SHELL" >"${RELEASE_DIR}/share/completions/${NOSEYPARKER}.$SHELL"
done

################################################################################
# Generate manpages
################################################################################
banner "Generating manpages"
"$NP" generate manpages --output "${RELEASE_DIR}/share/man/man1"

################################################################################
# Generate JSON schema
################################################################################
banner "Generating JSON schema"
"$NP" generate json-schema --output "${RELEASE_DIR}/share/${NOSEYPARKER}/report-schema.json"

################################################################################
# Sanity checking
################################################################################
banner "Release file sha256 digests"
find "$RELEASE_DIR" -type f -print0 | xargs -0 sha256sum | sort -k2

banner "Release disk use"
find "$RELEASE_DIR" -type f -print0 | xargs -0 du -shc | sort -h -k1,1

if [[ $PLATFORM == 'linux' ]]; then
    banner "ldd output for ${NOSEYPARKER}"
    ldd "$NP" || true

    banner "readelf -d output for ${NOSEYPARKER}"
    readelf -d "$NP"

elif [[ $PLATFORM == 'macos' ]]; then
    banner "otool -L output for ${NOSEYPARKER}"
    otool -L "$NP"

    banner "otool -l output for ${NOSEYPARKER}"
    otool -l "$NP"

else
    fatal "unknown platform $PLATFORM"
fi

banner "${NOSEYPARKER} --version"
"$NP" --version

banner "Complete!"
