#!/bin/bash

set -eo pipefail

current_dir=$(dirname "$0")
tmp_dir="$(dirname $current_dir)/tmp"

latest_tag=$(git describe --abbrev=0 --tags)

new_tag="v$(awk -F '"' '/^version/ {print $2}' Cargo.toml)"

if [ "$latest_tag" == "$new_tag" ]; then
    echo "Tag $new_tag already exists" >&2
    echo "Check that Cargo.toml version has been updated" >&2
    exit 1
fi

if [[ $new_tag == *"-dev"* ]]; then
    echo "Version $new_tag includes 'dev'" >&2
    exit 1
fi

if ! grep -q "$new_tag" CHANGELOG.md; then
    echo "$new_tag has not been added to the CHANGELOG.md" >&2
    exit 1
fi

$current_dir/generate-release-body.sh $tmp_dir

echo -e "Creating tag \033[1m$new_tag\033[0m with release body:\n"

cat "$tmp_dir/release-body.md"

git tag $new_tag -s -m "Eclair $new_tag"
