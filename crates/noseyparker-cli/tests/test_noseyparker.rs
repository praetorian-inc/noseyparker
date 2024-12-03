//! Integration tests for Nosey Parker

mod common;
use common::*;

mod datastore;

mod generate;

// XXX We'd like to actually run these tests when the `github` feature is disabled, but use
// different snapshot files in that case. But that's not easy to do.
#[cfg(feature = "github")]
mod github;

mod help;
mod report;
mod rules;
mod scan;

// TODO(test): add test for scanning with `--github-user`
// TODO(test): add test for scanning with `--github-org`
// TODO(test): add test for caching behavior of rescanning `--git-url`
// TODO(test): add test for scanning multiple times with changing `--git-clone-mode` option
// TODO(test): add test for scanning with `--git-clone-mode bare` and `--git-clone-mode mirror`
// TODO(test): add test for scanning with `--github-api-url`
// TODO(test): add test using a non-default `--github-api-url URL`
// TODO(test): add tests for SARIF output format

// TODO(test): add tests for blob metadata recording
// TODO(test): add tests for rerunning with changing `--blob-metadata` and `--git-blob-provenance` options

// TODO(test): add tests for trying to open existing datastores from other Nosey Parker versions
// TODO(test): add tests for enumerating GitHub Enterprise with the `--ignore-certs` optino
// TODO(test): add tests for `scan --git-url=URL --ignore-certs`
// TODO(test): add test case that validates `report -f json` output against the JSON schema (see the `jsonschema` crate)
// TODO(test) add test that failing to set rlimits at startup doesn't crash, but outputs a warning
