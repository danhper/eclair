#!/usr/bin/env bash
set -eo pipefail

# This script is used to install the latest version of Eclair binary into Foundry's bin directory
# The content of the script is mostly borrowed from foundryup


BASE_DIR=${XDG_CONFIG_HOME:-$HOME}
FOUNDRY_DIR=${FOUNDRY_DIR:-"$BASE_DIR/.foundry"}
FOUNDRY_BIN_DIR="$FOUNDRY_DIR/bin"
OUTPUT_FILE="$FOUNDRY_BIN_DIR/eclair"

LINUX_X86_URL=https://eclair-releases.s3.eu-west-2.amazonaws.com/x86_64-unknown-linux-gnu/eclair
MACOS_X86_URL=https://eclair-releases.s3.eu-west-2.amazonaws.com/x86_64-apple-darwin/eclair
MACOS_ARM64_URL=https://eclair-releases.s3.eu-west-2.amazonaws.com/aarch64-apple-darwin/eclair


tolower() {
  echo "$1" | awk '{print tolower($0)}'
}

err() {
  say "$1" >&2
  exit 1
}

check_cmd() {
  command -v "$1" &>/dev/null
}

# Downloads $1 into $2 or stdout
download() {
  if [ -n "$2" ]; then
    # output into $2
    if check_cmd curl; then
      curl -#o "$2" -L "$1"
    else
      wget --show-progress -qO "$2" "$1"
    fi
  else
    # output to stdout
    if check_cmd curl; then
      curl -#L "$1"
    else
      wget --show-progress -qO- "$1"
    fi
  fi
}

main() {
    ARCHITECTURE=$(tolower $(uname -m))
    if [ "${ARCHITECTURE}" = "x86_64" ]; then
        # Redirect stderr to /dev/null to avoid printing errors if non Rosetta.
        if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null)" = "1" ]; then
            ARCHITECTURE="arm64" # Rosetta.
        else
            ARCHITECTURE="amd64" # Intel.
        fi
    elif [ "${ARCHITECTURE}" = "arm64" ] ||[ "${ARCHITECTURE}" = "aarch64" ] ; then
        ARCHITECTURE="arm64" # Arm.
    else
        ARCHITECTURE="amd64" # Amd.
    fi


    URL=""
    PLATFORM=$(tolower $(uname -s))
    case $PLATFORM in
        linux)
            URL=$LINUX_X86_URL
        ;;
        darwin|mac*)
            if [ "${ARCHITECTURE}" = "arm64" ]; then
                URL=$MACOS_ARM64_URL
            else
                URL=$MACOS_X86_URL
            fi
        ;;
        *)
        err "unsupported platform: $PLATFORM ($ARCHITECTURE)"
        ;;
    esac


    download $URL $OUTPUT_FILE
    chmod +x $OUTPUT_FILE
    echo "Eclair binary has been installed to $OUTPUT_FILE"
}

main
