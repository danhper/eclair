#!/usr/bin/env bash

set -eo pipefail

TMP_DIR=${1:-tmp}

mkdir -p "$TMP_DIR"

awk '/^## v/{ if (flag) { exit 0 } flag = 1 } flag' CHANGELOG.md | sed -e '$d' -e '1,2d' > $TMP_DIR/release-body.md
