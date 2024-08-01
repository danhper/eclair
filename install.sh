#!/usr/bin/env bash
set -eo pipefail

# This script is used to install the latest version of Eclair binary into Foundry's bin directory
# The content of the script is largely borrowed from foundryup


BASE_DIR=${XDG_CONFIG_HOME:-$HOME}
FOUNDRY_DIR=${FOUNDRY_DIR:-"$BASE_DIR/.foundry"}
FOUNDRY_BIN_DIR="$FOUNDRY_DIR/bin"
OUTPUT_FILE="$FOUNDRY_BIN_DIR/eclair"

NIGHTLY_BASE_URL="https://eclair-releases.s3.eu-west-2.amazonaws.com/%s/eclair"
GITHUB_RELEASE_BASE_URL="https://github.com/danhper/eclair/releases/download/%s"

tolower() {
  echo "$1" | awk '{print tolower($0)}'
}

latest_release() {
  curl -s https://api.github.com/repos/danhper/eclair/releases/latest | grep -i "tag_name" | awk -F '"' '{print $4}'
}

say() {
  printf "eclair: %s\n" "$1"
}

warn() {
  say "warning: ${1}" >&2
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

get_architecture() {
    architecture=$(tolower $(uname -m))
    if [ "${architecture}" = "x86_64" ]; then
      # Redirect stderr to /dev/null to avoid printing errors if non Rosetta.
      if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null)" = "1" ]; then
          architecture="arm64" # Rosetta.
      else
          architecture="amd64" # Intel.
      fi
    elif [ "${architecture}" = "arm64" ] ||[ "${architecture}" = "aarch64" ] ; then
      architecture="arm64" # Arm.
    else
      architecture="amd64" # Amd.
    fi
    echo $architecture
}

main() {
  while [[ -n $1 ]]; do
    case $1 in
      --)               shift; break;;

      -v|--version)     shift; ECLAIR_VERSION=$1;;
      --arch)           shift; ECLAIR_ARCHITECTURE=$1;;
      --platform)       shift; ECLAIR_PLATFORM=$1;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        warn "unknown option: $1"
        usage
        exit 1
    esac; shift
  done

  if [ -z "$ECLAIR_ARCHITECTURE" ]; then
    ECLAIR_ARCHITECTURE=$(get_architecture)
  fi

  if [ -z "$ECLAIR_PLATFORM" ]; then
    ECLAIR_PLATFORM=$(tolower $(uname -s))
    if [ "$ECLAIR_PLATFORM" = "darwin" ]; then
      ECLAIR_PLATFORM="macos"
    fi
  fi

  case $ECLAIR_VERSION in
    ""|latest)
      BASE_URL="$NIGHTLY_BASE_URL"
      ;;
    release)
      BASE_URL="$(printf $GITHUB_RELEASE_BASE_URL $(latest_release))/eclair-%s"
      ;;
    v*)
      BASE_URL="$(printf $GITHUB_RELEASE_BASE_URL $ECLAIR_VERSION)/eclair-%s"
      ;;
    *)
      err "invalid version: $ECLAIR_VERSION"
      ;;
  esac

  ECLAIR_TARGET="$ECLAIR_PLATFORM-$ECLAIR_ARCHITECTURE"
  URL="$(printf $BASE_URL $ECLAIR_TARGET)"

  download $URL $OUTPUT_FILE
  chmod +x $OUTPUT_FILE
  echo "Eclair binary has been installed to $OUTPUT_FILE"
}

main $@
