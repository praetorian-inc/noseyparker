# Nosey Parker: Find secrets in textual data

## Overview

Nosey Parker is a command-line tool that finds secrets and sensitive information in textual data. It is useful both for offensive and defensive security testing.

**Key features:**
- It can natively scan files, directories, and Git repository history
- It uses regular expression matching with a set of [141 patterns](crates/noseyparker/data/default/builtin/rules) chosen for high signal-to-noise based on experience and feedback from offensive security engagements
- It deduplicates its findings, grouping matches together that share the same secret, which in practice can reduce review burden by 100x or more
- It is fast: it can scan at hundreds of megabytes per second on a single core, and is able to scan 100GB of Linux kernel source history in less than 2 minutes on an older MacBook Pro
- It scales: it has scanned inputs as large as 20TiB during security engagements

An internal version of Nosey Parker has found secrets in hundreds of offensive security engagements at [Praetorian](https://praetorian.com).
The internal version has additional capabilities for false positive suppression and a rule-free machine learning-based detection engine.
Read more in blog posts [here](https://www.praetorian.com/blog/nosey-parker-ai-secrets-scanner-release/) and [here](https://www.praetorian.com/blog/six-months-of-finding-secrets-with-nosey-parker/).


## Installation

### Homebrew formula

Nosey Parker is available in [Homebrew](https://brew.sh):
```shell
$ brew install noseyparker
```


### Prebuilt binaries

Prebuilt binaries are available for x86_64 Linux and x86_64/aarch64 macOS on the [latest release page](https://github.com/praetorian-inc/noseyparker/releases/latest).
This is a simple way to get started and will give good performance.


### Docker images

<details>

A prebuilt multiplatform Docker image is available for the latest release for x86_64 and aarch64:
```shell
$ docker pull ghcr.io/praetorian-inc/noseyparker:latest
```

Additionally, A prebuilt Docker image is also available for the most recent commit for x86_64:
```shell
$ docker pull ghcr.io/praetorian-inc/noseyparker:edge
```

Finally, an additional prebuilt [Alpine-based](https://hub.docker.com/_/alpine) Docker image is also available for the most recent commit for x86_64:
```shell
$ docker pull ghcr.io/praetorian-inc/noseyparker-alpine:edge
```

**Note:** The Docker images run noticeably slower than a native binary, particularly on macOS.

</details>


### Arch Linux package

Nosey Parker is available in the [Arch User Repository](https://aur.archlinux.org/packages/noseyparker).


### Building from source

<details>

#### 1. Install prerequisites
This has been tested with several versions of Ubuntu Linux on x86_64 and with macOS on both x86_64 and aarch64.

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


## Usage

### Overview

Nosey Parker is essentially a special-purpose `grep`-like tool for detection of secrets.
The typical workflow is three phases:

1. Scan inputs of interest using the `scan` command
2. Report details of scan results using the `report` command
3. Review and triage findings

The scanning and reporting steps are implemented as separate commands because you may wish to generate several reports from one expensive scan run.


### Getting help

Running the `noseyparker` binary without arguments prints top-level help and exits.
You can get abbreviated help for a particular command by running `noseyparker COMMAND -h`.
More detailed help is available with the `help` command or long-form `--help` option.

The prebuilt releases also include manpages that collect the command-line help in one place.
These manpages converted into Markdown format are also included in the repository [here](docs/v0.17.0/man/man1).

If you have a question that's not answered by this documentation, feel free to [start a discussion](https://github.com/praetorian-inc/noseyparker/discussions/new/choose).


### Terminology and data model

<details>

#### The datastore
Most Nosey Parker commands use a _datastore_, which is a special directory that Nosey Parker uses to record its findings and maintain its internal state.
A datastore will be implicitly created by the `scan` command if needed.

#### Blobs
Each input that Nosey Parker scans is called a _blob_, and has a unique blob ID, which is a SHA-1 digest computed the same way `git` does.

#### Provenance
Each blob has one or more _provenance_ entries associated with it.
A provenance entry is metadata that describes how the input was discovered, such as a file on the filesystem or an entry in Git repository history.

#### Rules
Nosey Parker is a rule-based system that uses regular expressions.
Each _rule_ has a single pattern with at least one capture group that isolates the match content from the surrounding context.
You can list available rules with `noseyparker rules list`.

#### Rulesets
A collection of rules is organized into a _ruleset_.
Nosey Parker's default ruleset includes rules that detect things that appear to be hardcoded secrets.
Other rulesets are available; you can list them with `noseyparker rules list.`

#### Matches
When a rule's pattern matches an input, it produces a _match_.
A match is defined by a rule, blob ID, start byte offset, and end byte offset; these fields are used to determine a unique match identifier.

#### Findings
Matches that were produced by the same rule and share the same capture groups are grouped into a _finding_.
In other words, a _finding_ is a group of matches.
This is Nosey Parker's top-level unit of reporting.

</details>


## Usage examples

### NOTE: When using Docker...

If you are using the Docker image, replace `noseyparker` in the following commands with a Docker invocation that uses a mounted volume:

```shell
docker run -v "$PWD":/scan ghcr.io/praetorian-inc/noseyparker:latest <ARGS>
```

The Docker container runs with `/scan` as its working directory, so mounting `$PWD` at `/scan` in the container will make tab completion and relative paths in your command-line invocation work.


### Scan inputs for secrets

#### Filesystem content, including local Git repos
![Screenshot showing Nosey Parker's workflow for scanning the filesystem for secrets](docs/usage-examples/gifs/02-scan-git-history.gif)

<details>

Nosey Parker has built-in support for scanning files, recursively scanning directories, and scanning the entire history of Git repositories.

For example, if you have a Git clone of [CPython](https://github.com/python/cpython) locally at `cpython.git`, you can scan its entire history with the `scan` command.
Nosey Parker will create a new datastore at `np.cpython` and saves its findings there.
(The name `np.cpython` is nonessential; it can be whatever you want.)
```
$ noseyparker scan --datastore np.cpython cpython.git
Found 28.30 GiB from 18 plain files and 427,712 blobs from 1 Git repos [00:00:04]
Scanning content  ████████████████████ 100%  28.30 GiB/28.30 GiB  [00:00:53]
Scanned 28.30 GiB from 427,730 blobs in 54 seconds (538.46 MiB/s); 4,904/4,904 new matches

 Rule                      Distinct Groups   Total Matches
───────────────────────────────────────────────────────────
 PEM-Encoded Private Key             1,076           1,192
 Generic Secret                        331             478
 netrc Credentials                      42           3,201
 Generic API Key                         2              31
 md5crypt Hash                           1               2

Run the `report` command next to show finding details.
```
</details>


#### Git repos given URL, GitHub username, or GitHub organization name

<details>

Nosey Parker can also scan Git repos that have not already been cloned to the local filesystem.
The `--git-url URL`, `--github-user NAME`, and `--github-org NAME` options to `scan` allow you to specify repositories of interest.

For example, to scan the Nosey Parker repo itself:
```
$ noseyparker scan --datastore np.noseyparker --git-url https://github.com/praetorian-inc/noseyparker
```

For example, to scan accessible repositories belonging to [`octocat`](https://github.com/octocat):
```
$ noseyparker scan --datastore np.noseyparker --github-user octocat
```

These input specifiers will use an optional GitHub token if available in the `NP_GITHUB_TOKEN` environment variable.
Providing an access token gives a higher API rate limit and may make additional repositories accessible to you.

See `noseyparker help scan` for more details.
</details>


### Report findings

<details>

To see details of Nosey Parker's findings, use the `report` command.
This prints out a text-based report designed for human consumption:
```
$ noseyparker report --datastore np.cpython
Finding 1/1452: Generic API Key
Match: QTP4LAknlFml0NuPAbCdtvH4KQaokiQE
Showing 3/29 occurrences:

    Occurrence 1:
    Git repo: clones/cpython.git
    Blob: 04144ceb957f550327637878dd99bb4734282d07
    Lines: 70:61-70:100

        e buildbottest

        notifications:
          email: false
          webhooks:
            urls:
              - https://python.zulipchat.com/api/v1/external/travis?api_key=QTP4LAknlFml0NuPAbCdtvH4KQaokiQE&stream=core%2Ftest+runs
            on_success: change
            on_failure: always
          irc:
            channels:
              # This is set to a secure vari

    Occurrence 2:
    Git repo: clones/cpython.git
    Blob: 0e24bae141ae2b48b23ef479a5398089847200b3
    Lines: 174:61-174:100

        j4 -uall,-cpu"

        notifications:
          email: false
          webhooks:
            urls:
              - https://python.zulipchat.com/api/v1/external/travis?api_key=QTP4LAknlFml0NuPAbCdtvH4KQaokiQE&stream=core%2Ftest+runs
            on_success: change
            on_failure: always
          irc:
            channels:
              # This is set to a secure vari
...
```

(Note: the findings above are synthetic, invalid secrets.)
Additional output formats are supported, including JSON, JSON lines, and SARIF (experimental), via the `--format=FORMAT` option.
</details>

#### Human-readable text format
![Screenshot showing Nosey Parker's workflow for rendering its findings in human-readable format](docs/usage-examples/gifs/03-report-human.gif)

#### JSON format
![Screenshot showing Nosey Parker's workflow for rendering its findings in JSON format](docs/usage-examples/gifs/04-report-json.gif)


### Summarize findings

<details>

Nosey Parker prints out a summary of its findings when it finishes scanning.
You can also run this step separately:
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
</details>


### Enumerate repositories from GitHub

<details>

To list URLs for repositories belonging to GitHub users or organizations, use the `github repos list` command.
This command uses the GitHub REST API to enumerate repositories belonging to one or more users or organizations.
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

An optional GitHub Personal Access Token can be provided via the `NP_GITHUB_TOKEN` environment variable.
Providing an access token gives a higher API rate limit and may make additional repositories accessible to you.

Additional output formats are supported, including JSON and JSON lines, via the `--format=FORMAT` option.

See `noseyparker help github` for more details.
</details>


## Integrations

Nosey Parker has a few third-party integrations:

- Nosey Parker is packaged in [Homebrew](https://formulae.brew.sh/formula/noseyparker)
- Nosey Parker is packaged in [Arch Linux](https://aur.archlinux.org/packages/noseyparker)
- A [GitHub Action](https://github.com/bpsizemore/noseyparker-action) that runs Nosey Parker is available
- [DefectDojo](https://defectdojo.org) includes a [parser for Nosey Parker v0.16 JSON](https://github.com/DefectDojo/django-DefectDojo/blob/c182e9ca9d8f981c15de2018f948fe69c4d1a800/docs/content/en/integrations/parsers/file/noseyparker.md)
- [Nemesis](https://github.com/SpecterOps/Nemesis) includes support for Nosey Parker
- [segfault.net root servers include Nosey Parker v0.16.0](https://www.thc.org/segfault/)

If you have an integration you'd like to share that's not listed here, please create a PR.


## Contributing

Feel free to ask questions or share ideas in the [Discussions](https://github.com/praetorian-inc/noseyparker/discussions) page.

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
