# Installation

## Using the installer

You can install Eclair using the installer script:

```bash
curl -L https://install.eclair.so | bash
```

## Installing from binaries

The latest release binaries are available at:

- [Linux x86](https://eclair-releases.s3.eu-west-2.amazonaws.com/x86_64-unknown-linux-gnu/eclair)
- [macOS x86](https://eclair-releases.s3.eu-west-2.amazonaws.com/x86_64-apple-darwin/eclair)
- [macOS arm64](https://eclair-releases.s3.eu-west-2.amazonaws.com/aarch64-apple-darwin/eclair)

Just download the binary, make it executable, and preferably put it in a directory on the path.

## Installing the latest version from source

This requires to have Rust and Cargo installed. If you don't have it, you can install it by following the instructions on the [official website](https://www.rust-lang.org/tools/install).

```
cargo install --git https://github.com/danhper/eclair.git eclair
```
