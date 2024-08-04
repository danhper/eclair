#!/usr/bin/env bash

set -eo pipefail

TMP_DIR=${1:-tmp}

NIGHTLY_BASE_URL="https://eclair-releases.s3.eu-west-2.amazonaws.com/%s/eclair"

mkdir -p "$TMP_DIR"

versions=(linux-amd64 macos-amd64 macos-arm64)

for version in "${versions[@]}"; do
  wget "$(printf "$NIGHTLY_BASE_URL" "$version")" -O "$TMP_DIR/eclair-$version"
done
