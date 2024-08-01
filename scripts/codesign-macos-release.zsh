#!/bin/zsh

set -eu

################################################################################
# utilities
################################################################################
fatal() {
    echo >&2 "error: $@"
    exit 1
}

debug() {
    echo >&2 "DEBUG: $@"
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

################################################################################
# configuration
################################################################################
# The directory containing this script
HERE=${0:a:h}

FILES_TO_SIGN=(
    bin/noseyparker
)

CODESIGN_IDENTITY='Developer ID Application: Praetorian Security, Inc. (59HE95V9N4)'
KEYCHAIN_PROFILE='praetorian'


################################################################################
# Let's go!
################################################################################
if (( $# != 2 )); then
    echo >&2 "usage: $0 RELEASE_TGZ OUTPUT_TGZ"
    exit 1
fi

INPUT="$1"
OUTPUT="$2"

debug "INPUT = $INPUT"
debug "OUTPUT = $OUTPUT"

SCRATCH="$(mktemp -d)" || fatal "failed to create scratch directory"
debug "SCRATCH = $SCRATCH"

# extract input tarfile
tar -C "$SCRATCH" -xf "$INPUT" || fatal "failed to extract input"

banner "Codesigning files"
(
    cd "$SCRATCH"
    debug "codesign -s \"$CODESIGN_IDENTITY\" -o runtime $FILES_TO_SIGN"
    codesign -s "$CODESIGN_IDENTITY" -o runtime $FILES_TO_SIGN || fatal "failed to sign $FILES_TO_SIGN"
) || fatal "failed to sign files"

banner "Notarizing files"
# notarization only accepts zip files and .dmg bundles, not individual signed files,
# so zip all the signed files up
(
    cd "$SCRATCH"
    zip combined.zip $FILES_TO_SIGN

    # submit and wait for notarization
    xcrun notarytool submit --wait --keychain-profile "$KEYCHAIN_PROFILE" combined.zip || fatal "failed to notarize files"

    rm combined.zip
)

# create output tarfile
tar -C "$SCRATCH" -acf "$OUTPUT" --no-xattrs --no-mac-metadata . || fatal "failed to create output tarfile"

rm -rf "$SCRATCH"

banner "Codesigned release written to $OUTPUT"
