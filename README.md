# Nosey Parker: Find secrets in textual data

## Overview

Nosey Parker is a CLI tool that finds secrets and sensitive information in textual data.
It is essentially a special-purpose `grep`-like tool for detection of secrets.

It has been designed for offensive security (e.g., enabling lateral movement on red teams), but it can also be useful for defensive security testing.
It has found secrets in hundreds of offensive security engagements at [Praetorian](https://praetorian.com).

**Key features:**
- **Flexiblity:** It natively scans files, directories, GitHub, and Git history, and has an extensible input enumeration mechanism
- **Field-tested rules:** It uses regular expressions with [183 patterns](crates/noseyparker/data/default/builtin/rules) chosen for high precision based on feedback from security engineers
- **Signal-to-noise:** It deduplicates matches that share the same secret, reducing review burden by 10-1000x or more
- **Speed & scalability:** it can scan at GB/s on a multicore system, and has scanned inputs as large as 20TB during security engagements

The typical workflow is three phases:

1. Scan inputs of interest using the `scan` command
2. Report details of scan results using the `report` command
3. Review and triage findings

## Installation

### [Homebrew](https://brew.sh) formula

```shell
brew install noseyparker
```


### Prebuilt binaries

The [latest release page](https://github.com/praetorian-inc/noseyparker/releases/latest) contains prebuilt binaries for x86_64/aarch64 Linux and macOS.


### Docker: x86_64/aarch64

```shell
docker pull ghcr.io/praetorian-inc/noseyparker:latest
```

The **most recent commit** is also available via the `main` tag.

### Docker: x86_64/aarch64, Alpine base:

```shell
docker pull ghcr.io/praetorian-inc/noseyparker-alpine:latest
```

The **most recent commit** is also available via the `main` tag.


### Arch Linux package

<https://aur.archlinux.org/packages/noseyparker>


### Windows

Nosey Parker does not build natively on Windows ([#121](https://github.com/praetorian-inc/noseyparker/issues/121)).
It _is_ possible to run on Windows using [WSL1](https://en.wikipedia.org/wiki/Windows_Subsystem_for_Linux) and the native Linux release.


### Building from source

<details>

#### 1. Install prerequisites
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

#### 2. Build using the [`create-release.zsh`](scripts/create-release.zsh) script
```shell
$ rm -rf release && ./scripts/create-release.zsh
```

If successful, this will produce a directory structure at `release` populated with release artifacts.
The command-line program will be at `release/bin/noseyparker`.
</details>


## Getting help

Running the `noseyparker` binary without arguments prints top-level help and exits.
You can get abbreviated help for a particular command by running `noseyparker COMMAND -h`.
More detailed help is available with the `help` command or long-form `--help` option.

The prebuilt releases also include manpages that collect the command-line help in one place.
These manpages converted into Markdown format are also included in the repository [here](docs/v0.17.0/man/man1).

If you have a question that's not answered by this documentation, please [start a discussion](https://github.com/praetorian-inc/noseyparker/discussions/new/choose).


## Terminology and data model

### The datastore
The _datastore_ is a special directory that Nosey Parker uses to record its findings and maintain its internal state.
A datastore will be implicitly created by the `scan` command if needed.

### Blobs
Each scanned input is called a _blob_. Each blob has a unique blob ID, which is a SHA-1 digest computed the same way `git` does.

### Provenance
Each blob has one or more _provenance_ entries associated with it.
A provenance entry is metadata that describes how the input was discovered, such as a file on the filesystem or a file in Git repository history.

### Rules
Nosey Parker is a rule-based system that uses regular expressions.
Each _rule_ has a single pattern with at least one capture group that isolates the match content from the surrounding context.
You can list available rules with `noseyparker rules list`.

### Rulesets
A collection of rules is organized into a _ruleset_.
Nosey Parker's default ruleset includes rules that detect things that appear to be secrets.
Other rulesets are available; you can list them with `noseyparker rules list.`

### Matches
When a rule's pattern matches an input, it produces a _match_.
A match is uniquely defined by a rule, blob ID, start byte offset, and end byte offset; these fields are used to compute a unique match identifier.

### Findings
Matches that share a rule and capture groups are combined into a _finding_.
In other words, a _finding_ is a group of matches.
This is Nosey Parker's top-level unit of reporting.


## Usage examples

### NOTE: When using Docker...

When using the Docker image, replace `noseyparker` in the following commands with a Docker invocation that uses a mounted volume:

```shell
docker run -v "$PWD":/scan ghcr.io/praetorian-inc/noseyparker:latest <ARGS>
```

The Docker container runs with `/scan` as its working directory, so mounting `$PWD` at `/scan` in the container will make tab completion and relative paths in your command-line invocation work.


### Scan filesystem content, including local Git repos
![Screenshot showing Nosey Parker's workflow for scanning the filesystem for secrets](docs/usage-examples/gifs/02-scan-git-history.gif)

Nosey Parker has native support for scanning files, directories, and the entire history of Git repositories.

For example, if you have a Git clone of [CPython](https://github.com/python/cpython) locally at `cpython.git`, you can scan it with the `scan` command.
Nosey Parker will create a new datastore at `cpython.np` and saves its findings there.
(The name `cpython.np` is innessential, and can be whatever you want.)
```
$ noseyparker scan -d cpython.np cpython.git
Scanned 19.19 GiB from 335,849 blobs in 17 seconds (1.11 GiB/s); 2,178/2,178 new matches

 Rule                            Findings   Matches   Accepted   Rejected   Mixed   Unlabeled
──────────────────────────────────────────────────────────────────────────────────────────────
 Generic API Key                        1         8          0          0       0           1
 Generic Password                       8     1,283          0          0       0           8
 Generic Username and Password          2        40          0          0       0           2
 HTTP Bearer Token                      1       108          0          0       0           1
 PEM-Encoded Private Key               61       151          0          0       0          61
 netrc Credentials                     27       588          0          0       0          27

Run the `report` command next to show finding details.
```

See `noseyparker help scan` for more details.

### Scan a Git repo from an HTTPS URL

For example, to scan the Nosey Parker repo itself:
```
noseyparker scan --datastore np.noseyparker --git-url https://github.com/praetorian-inc/noseyparker
```

See `noseyparker help scan` for more details.

### Scan Git repos of a GitHub user or organization

Use `--github-user=USER` or `--github-org=ORG`. For example, to scan accessible repositories belonging to the [`octocat`](https://github.com/octocat) user:
```
noseyparker scan --datastore np.noseyparker --github-user octocat
```

These input specifiers will use an optional GitHub token if available in the `NP_GITHUB_TOKEN` environment variable.
Providing an access token gives a higher API rate limit and may make additional repositories accessible to you.

See `noseyparker help scan` for more details.


### Interactively review and annotate findings
See the companion project, [Nosey Parker Explorer](https://github.com/praetorian-inc/noseyparkerexplorer):
![Screenshot showing the main interface of Nosey Parker Explorer](https://github.com/praetorian-inc/noseyparkerexplorer/blob/32e9133600c79eee53cd9000e37b71792e555fdd/docs/img/main-screen.png?raw=true)

### Report findings in human-readable text format
![Screenshot showing Nosey Parker's workflow for rendering its findings in human-readable format](docs/usage-examples/gifs/03-report-human.gif)


### Report findings in JSON format
![Screenshot showing Nosey Parker's workflow for rendering its findings in JSON format](docs/usage-examples/gifs/04-report-json.gif)


### Summarize findings

Nosey Parker prints out a summary of its findings when it finishes scanning.
You can also run this step separately after scanning:
```
$ noseyparker summarize --datastore np.cpython

 Rule                      Distinct Groups   Total Matches
───────────────────────────────────────────────────────────
 PEM-Encoded Private Key             1,076           1,192
 Generic Secret                        331             478
 netrc Credentials                      42           3,201
 Generic API Key                         2              31
 md5crypt Hash                           1               2
```

Additional output formats are supported, including JSON and JSON lines, via the `--format=FORMAT` option.

See `noseyparker help summarize` for more details.


### Enumerate repositories from GitHub

Use `github repos list` command to list URLs for repositories belonging to GitHub users or organizations.
This command uses the GitHub REST API to enumerate repositories belonging to users or organizations.
For example:
```
$ noseyparker github repos list --user octocat
https://github.com/octocat/Hello-World.git
https://github.com/octocat/Spoon-Knife.git
https://github.com/octocat/boysenberry-repo-1.git
https://github.com/octocat/git-consortium.git
https://github.com/octocat/hello-worId.git
https://github.com/octocat/linguist.git
https://github.com/octocat/octocat.github.io.git
https://github.com/octocat/test-repo1.git
```

This command will use an optional GitHub token if available in the `NP_GITHUB_TOKEN` environment variable.
Providing an access token gives a higher API rate limit and may make additional repositories accessible to you.

Additional output formats are supported, including JSON and JSON lines, via the `--format=FORMAT` option.

See `noseyparker help github` for more details.


## Integrations

Nosey Parker has a few third-party integrations:

- Nosey Parker is packaged in [Homebrew](https://formulae.brew.sh/formula/noseyparker)
- Nosey Parker is packaged in [Arch Linux](https://aur.archlinux.org/packages/noseyparker)
- A [GitHub Action](https://github.com/bpsizemore/noseyparker-action) that runs Nosey Parker is available
- [DefectDojo](https://defectdojo.org) includes a [parser for Nosey Parker v0.16 JSON](https://github.com/DefectDojo/django-DefectDojo/blob/c182e9ca9d8f981c15de2018f948fe69c4d1a800/docs/content/en/integrations/parsers/file/noseyparker.md)
- [Nemesis](https://github.com/SpecterOps/Nemesis) includes support for Nosey Parker

If you have an integration you'd like to share that's not listed here, please create a PR.


## Contributing

Ask questions or share ideas in the [Discussions](https://github.com/praetorian-inc/noseyparker/discussions) area.

Contributions are welcome, especially new regex rules.
Developing new regex rules is detailed in a [separate document](docs/RULES.md).

If you are considering making significant code changes, please [open an issue](https://github.com/praetorian-inc/noseyparker/issues/new) or [start a discussion](https://github.com/praetorian-inc/noseyparker/discussions/new/choose) first.

This project has a number of [pre-commit](https://pre-commit.com/) hooks enabled that you are encouraged to use.
To install them in your local repo, make sure you have `pre-commit` installed and run:
```
$ pre-commit install
```
These checks will help to quickly detect simple errors.


## License

Nosey Parker is licensed under the [Apache License, Version 2.0](LICENSE).

Any contribution intentionally submitted for inclusion in Nosey Parker by you, as defined in the Apache 2.0 license, shall be licensed as above, without any additional terms or conditions.

Nosey Parker also includes vendored copies of several other packages released under the Apache License and other permissive licenses; see [`LICENSE`](LICENSE) for details.
