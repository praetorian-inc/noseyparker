# Installation


## Prebuilt binaries

The [latest release page](https://github.com/praetorian-inc/noseyparker/releases/latest) contains prebuilt binaries for x86_64/aarch64 Linux and macOS.


## [Homebrew](https://brew.sh)

```shell
brew install noseyparker
```


## Docker

### x86_64 or aarch64, Debian base

```shell
docker pull ghcr.io/praetorian-inc/noseyparker:latest
```

The **most recent commit** is also available via the `main` tag.

### x86_64 or aarch64, Alpine base

```shell
docker pull ghcr.io/praetorian-inc/noseyparker-alpine:latest
```

The **most recent commit** is also available via the `main` tag.


## Arch Linux

<https://aur.archlinux.org/packages/noseyparker>


## Windows

Nosey Parker does not build natively on Windows ([#121](https://github.com/praetorian-inc/noseyparker/issues/121)).
It _is_ possible to run on Windows using [WSL1](https://en.wikipedia.org/wiki/Windows_Subsystem_for_Linux) and the native Linux release.


## Building from source

### 1. Install prerequisites
This has been tested with several versions of Ubuntu Linux and macOS on both x86_64 and aarch64.

Required dependencies:

- `cargo`: recommended approach: install from <https://rustup.rs>
- `cmake`: needed for building the `vectorscan-sys` crate and some other dependencies
- `boost`: needed for building the `vectorscan-sys` crate (supported version `>=1.57`)
- `git`: needed for embedding version information into the `noseyparker` CLI
- `patch`: needed for building the `vectorscan-sys` crate
- `pkg-config`: needed for building the `vectorscan-sys` crate
- `sha256sum`: needed for computing digests (often provided by the `coreutils` package)
- `zsh`: needed for build scripts

### 2. Build using the [`create-release.zsh`](scripts/create-release.zsh) script
```shell
$ rm -rf release && ./scripts/create-release.zsh
```

If successful, this will produce a directory structure at `release` populated with release artifacts.
The command-line program will be at `release/bin/noseyparker`.
