# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project aspires to eventually use [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## Unreleased

### Changes
- Several default rules have been revised to improve performance of the matching engine and to produce fewer false positives.
  In particular, several rules previously had avoided using a trailing `\b` anchor after secret content which could include a literal `-` character, due to a matching discrepancy between Hyperscan and Rust's `regex` library.
  These have been revised to use a more complicated but functional anchoring pattern.


## [v0.11.0](https://github.com/praetorian-inc/noseyparker/releases/v0.11.0) (2022-12-30)

This is the first released version of Nosey Parker.
Its `scan`, `summarize`, and `report` commands are functional.
It is able to scan files, directories, and the complete history of Git repositories at several hundred megabytes per second per core.
It comes with 58 rules.

A prebuilt Docker image for this release is available for x86_64 architectures:
```shell
docker pull ghcr.io/praetorian-inc/noseyparker:v0.11.0
```
