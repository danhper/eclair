# Installation

## Using the installer

You can install the latest version of Eclair (latest push on main branch) using the installer script:

```bash
curl -L https://install.eclair.so | bash
```

This will install Eclair in the Foundry bin directory, typically `~/.foundry/bin`.

If you want the latest published release instead, you can use the following command:

```bash
curl -L https://install.eclair.so | bash -s -- --version release
```

To install a specific version, you can use the following command:

```bash
curl -L https://install.eclair.so | bash -s -- --version VERSION
```

where `VERSION` should be replaced with [a release version](https://github.com/danhper/eclair/releases).

## Installing from binaries

The latest release binaries are available at:

- [Linux x86](https://eclair-releases.s3.eu-west-2.amazonaws.com/x86_64-unknown-linux-gnu/eclair)
- [macOS x86](https://eclair-releases.s3.eu-west-2.amazonaws.com/x86_64-apple-darwin/eclair)
- [macOS arm64](https://eclair-releases.s3.eu-west-2.amazonaws.com/aarch64-apple-darwin/eclair)

Just download the binary, make it executable, and preferably put it in a directory on the path.

The release binaries can be found on the [releases page](https://github.com/danhper/eclair/releases).

## Installing the latest version from source

This requires to have Rust and Cargo installed. If you don't have it, you can install it by following the instructions on the [official website](https://www.rust-lang.org/tools/install).

```
cargo install --git https://github.com/danhper/eclair.git eclair
```
