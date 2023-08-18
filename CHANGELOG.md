# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project aspires to use [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## Unreleased


## [v0.14.0](https://github.com/praetorian-inc/noseyparker/releases/v0.14.0) (2023-08-17)

A [prebuilt multiplatform Docker image](https://github.com/praetorian-inc/noseyparker/pkgs/container/noseyparker/119658519?tag=v0.14.0) for this release is available for x86_64 and ARM64 architectures:
```
docker pull ghcr.io/praetorian-inc/noseyparker:v0.14.0
```

### Additions
- Running `noseyparker --version` now emits many compile-time details about the build, which can be useful for troubleshooting ([#48](https://github.com/praetorian-inc/noseyparker/issues/48)).

- The `github` and `scan` commands now support accessing GitHub Enterprise Server instances using the new `--github-api-url URL` parameter ([#53](https://github.com/praetorian-inc/noseyparker/pull/53)—thank you @AdnaneKhan!).

- New rules have been added:

  - Amazon Resource Name
  - AWS S3 Bucket (subdomain style)
  - AWS S3 Bucket (path style)
  - Google Cloud Storage Bucket (subdomain style)
  - Google Cloud Storage Bucket (path style)
  - HuggingFace User Access Token ([#54](https://github.com/praetorian-inc/noseyparker/pull/54)—thank you @AdnaneKhan!)

- Rules are now required to have a globally-unique identifier ([#62](https://github.com/praetorian-inc/noseyparker/pull/62))

- Two new advanced global command-line parameters have been exposed:

  - `--rlimit-nofile LIMIT` to control the maximum number of open file descriptors
  - `--enable-backtraces BOOL` to control whether backtraces are printed upon panic

- The snippet length for matches found by the `scan` command can now be controlled with the new `--snippet-length BYTES` parameter.

- The Git repository cloning behavior in the `scan` command can now be controlled with the new `--git-clone-mode {mirror,bare}` parameter.

- The `scan` command now collects additional metadata about blobs.
  This metadata includes size in bytes and guessed mime type based on filename extension.
  Optionally, if the non-default `libmagic` Cargo feature is enabled, the mime type and charset are guessed by passing the content of the blob through `libmagic` (the guts of the `file` command-line program).

  By default, all this additional metadata is recorded into the datastore for each blob in which matches are found.
  This can be more precisely controlled using the new `--blob-metadata={all,matching,none}` parameter.

  This newly-collected metadata is included in output of the `report` command.

- The `scan` command now collects additional metadata about blobs found within Git repositories.
  Specifically, for each blob found in Git repository history, the set of commits where it was introduced and the accompanying pathname for the blob is collected ([#16](https://github.com/praetorian-inc/noseyparker/issues/16)).
  This is enabled by default, but can be controlled using the new `--git-blob-provenance={first-seen,minimal}` parameter.

  This newly-collected metadata is included in output of the `report` command.

### Changes
- The datastore schema has been changed in an incompatible way such that migrating existing datastores to the new version is not possible.
  This was necessary to support the significantly increased metadata that is now collected when scanning.
  Datastores from earlier releases of Nosey Parker cannot be used with this release; instead, the inputs will have to be rescanned with a new datastore.

- The JSON and JSONL output formats for the `report` command have changed slightly.
  In particular, the `.matches[].provenance` field is now an array of objects instead of a single object, making it possible to handle situations where a blob is discovered multiple ways.
  The `provenenance` objects have some renamed fields, and contain significantly more metadata than before.


- Existing rules were modified to reduce both false positives and false negatives:

  - Generic Password (double quoted)
  - Generic Password (single quoted)

- The default size of match snippets has been increased from 128 bytes before and after to 256.
  This typically gives 4-7 lines of context before and after each match.

- When a Git repository is cloned, the default behavior is to match `git clone --bare` instead of `git clone --mirror`.
  This new default behavior results in cloning potentially less content, but avoids cloning content from forks from repositories hosted on GitHub.

- The command-line help has been refined for clarity.

- Scanning performance has been improved on particular workloads by as much as 2x by recording matches to the datastore in larger batches.
  This is particularly relevant to heavy multithreaded scanning workloads where the inputs have many matches.

### Fixes
- Python is no longer required as a build-time dependency for `vectorscan-sys`.

- A typo was fixed in the Okta API Key rule that caused it to truncate the secret.

- The `scan` command now correctly reports the number of newly-seen matches when reusing an existing datastore.


## [v0.13.0](https://github.com/praetorian-inc/noseyparker/releases/v0.13.0) (2023-04-24)

A [prebuilt multiplatform Docker image](https://github.com/praetorian-inc/noseyparker/pkgs/container/noseyparker/88043263?tag=v0.13.0) for this release is available for x86_64 and ARM64 architectures:
```
docker pull ghcr.io/praetorian-inc/noseyparker:v0.13.0
```

### Changes
- Nosey Parker now statically links against a bundled version of [Vectorscan](https://github.com/Vectorcamp/vectorscan) for regular expression matching instead of [Hyperscan](https://github.com/intel/hyperscan) ([#5](https://github.com/praetorian-inc/noseyparker/issues/5)).
  This makes building from source simpler, particularly for ARM-based platforms.
  This also simplifies distribution, as a precompiled `noseyparker` binary now has no runtime library dependencies on non-default libraries.

- Several existing rules were modified to reduce false positives and false negatives:

  - Generic API Key
  - Telegram Bot Token

### Additions
- New rules have been added:

  - Generic Username and Password (quoted)
  - Generic Username and Password (unquoted)
  - Generic Password (double quoted)
  - Generic Password (single quoted)
  - Grafana API Token
  - Grafana Cloud API Token
  - Grafana Service Account Token
  - Postman API Key

- References have been added for several rules:

  - Twilio API Key
  - Dynatrace Token

### Fixes
- The Docker image now has the `git` binary installed. Previously this was missing, causing the `scan` command to fail when the `--git-url`, `--github-user`, or `--github-organization` input specifiers were used ([#38](https://github.com/praetorian-inc/noseyparker/issues/38)).


## [v0.12.0](https://github.com/praetorian-inc/noseyparker/releases/v0.12.0) (2023-03-02)

A [prebuilt Docker image](https://github.com/praetorian-inc/noseyparker/pkgs/container/noseyparker/74541424?tag=v0.12.0) for this release is available for x86_64 architectures:
```
docker pull ghcr.io/praetorian-inc/noseyparker:v0.12.0
```

### Additions
- The `scan` command can now be given Git https URLs, GitHub usernames, and GitHub organization names as inputs, and will enumerate, clone, and scan as appropriate ([#14](https://github.com/praetorian-inc/noseyparker/issues/14)).

- Nosey Parker now has rudimentary support for enumerating repositories from GitHub users and organizations ([#15](https://github.com/praetorian-inc/noseyparker/issues/15)).
  The new `github repos list` command uses the GitHub REST API to enumerate repositories belonging to one or more users or organizations.
  An optional GitHub Personal Access Token can be provided via the `NP_GITHUB_TOKEN` environment variable.

- Nosey Parker now has an optional `rule_profiling` crate feature that causes performance-related statistics to be collected and reported when scanning.
  This feature imposes some performance cost and is only useful to rule authors, and so is disabled by default.

- Many new rules have been added:

  - Adobe OAuth Client Secret
  - Age Identity (X22519 secret key)
  - Age Recipient (X25519 public key)
  - crates.io API Key
  - DigitalOcean Application Access Token
  - DigitalOcean Personal Access Token
  - DigitalOcean Refresh Token
  - Figma Personal Access Token
  - GitLab Personal Access Token
  - GitLab Pipeline Trigger Token
  - GitLab Runner Registration Token
  - Google OAuth Client Secret (prefixed)
  - New Relic API Service Key
  - New Relic Admin API Key
  - New Relic Insights Insert Key
  - New Relic Insights Query Key
  - New Relic License Key
  - New Relic License Key (non-suffixed)
  - New Relic Pixie API Key
  - New Relic Pixie Deploy Key
  - New Relic REST API Key
  - NPM Access Token (fine-grained)
  - OpenAI API Key
  - Segment Public API Token
  - Shopify Access Token (Custom App)
  - Shopify Access Token (Legacy Private App)
  - Shopify Access Token (Public App)
  - Shopify App Secret
  - Shopify Domain
  - RubyGems API Key
  - Telegram Bot Token

  These rules match token formats that are well-specified fixed-length strings with notable prefixes or suffixes, and so should produce very few false positives.

- Several existing rules were modified to improve signal-to-noise:

  - Azure Connection String
  - Credentials in ODBC Connection String
  - PyPI Upload Token

- The `report` command now offers rudimentary SARIF support ([#4](https://github.com/praetorian-inc/noseyparker/issues/4)).
  Thank you @Coruscant11!

### Changes
- Several default rules have been revised to improve performance of the matching engine and to produce fewer false positives.
  In particular, several rules previously had avoided using a trailing `\b` anchor after secret content which could include a literal `-` character, due to a matching discrepancy between Hyperscan and Rust's `regex` library.
  These have been revised to use a more complicated but functional anchoring pattern.

- The `JSON Web Token (base64url-encoded)` rule has been changed to only produce a single match group instead of three.

- The `Google Client Secret` rule has been improved to detect additional occurrences and has been renamed to `Google OAuth Client Secret`.

- Blobs are now deduplicated at enumeration time when first enumerating a Git repository, rather than only at scan time. This results in more accurate progress bars.

- When scanning, Git repositories are now opened twice: once at input enumeration time, and once at scanning time.
  This drastically reduces the amount of memory required to scan a large number of Git repositories.

### Fixes
- When scanning, the datastore is now explicitly excluded from filesystem enumeration.
  This ensures that files used internally for Nosey Parker's operation are not inadvertently scanned ([#32](https://github.com/praetorian-inc/noseyparker/issues/32)).


## [v0.11.0](https://github.com/praetorian-inc/noseyparker/releases/v0.11.0) (2022-12-30)

This is the first released version of Nosey Parker.
Its `scan`, `summarize`, and `report` commands are functional.
It is able to scan files, directories, and the complete history of Git repositories at several hundred megabytes per second per core.
It comes with 58 rules.

A [prebuilt Docker image](https://github.com/praetorian-inc/noseyparker/pkgs/container/noseyparker/61045424?tag=v0.11.0) for this release is available for x86_64 architectures:
```shell
docker pull ghcr.io/praetorian-inc/noseyparker:v0.11.0
```
