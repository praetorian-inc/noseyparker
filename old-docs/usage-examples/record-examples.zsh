#!/bin/zsh

set -e

rm -rf scratch gifs && mkdir scratch gifs

for example in examples/*.tape; do
    echo "### $example"
    scratch_dir="scratch/$(basename "$example" .tape)"
    mkdir "$scratch_dir"

    preprocessed="$scratch_dir/input.tape"

    cat common-config.tape "$example" >"$preprocessed"

    (cd "$scratch_dir" && vhs input.tape && cp output.gif "../../gifs/$(basename "$example" .tape).gif")
done

# other examples to create:
# - listing rules
# - checking rules
# - reporting in SARIF format
# - reporting in JSONL format
# - dumping the report JSON schema
